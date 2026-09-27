use rodio::buffer::SamplesBuffer;
use rodio::{OutputStreamHandle, Source};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use super::generator::KeyType;
use super::soundpack::{get_bundled_soundpacks, SwitchSoundpack};

/// Hard ceiling on the combined gain (master x release x per-profile) so stacking three
/// independent controls can't drive the mixer into clipping.
const MAX_COMBINED_GAIN: f32 = 1.6;

pub struct AudioState {
    soundpacks: Vec<SwitchSoundpack>,
    current_pack_index: AtomicU32,
    volume: AtomicU32, // Master, percentage 0..150
    release_volume: AtomicU32, // Upstroke level relative to master, percentage 0..150
    pack_volumes: Vec<AtomicU32>, // Per-profile trim, percentage 0..150
    is_muted: AtomicBool,
}

impl AudioState {
    pub fn new() -> Self {
        let mut soundpacks = get_bundled_soundpacks();

        // Also check if there's a ./soundpacks folder for custom user soundpacks
        if let Ok(entries) = std::fs::read_dir("soundpacks") {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(custom_pack) = SwitchSoundpack::try_load_from_folder(&entry.path()) {
                        println!("[Keeboy] Loaded custom soundpack: {}", custom_pack.name);
                        soundpacks.push(custom_pack);
                    }
                }
            }
        }

        let pack_volumes = soundpacks.iter().map(|_| AtomicU32::new(100)).collect();

        Self {
            soundpacks,
            current_pack_index: AtomicU32::new(0),
            volume: AtomicU32::new(75),
            release_volume: AtomicU32::new(100),
            pack_volumes,
            is_muted: AtomicBool::new(false),
        }
    }

    pub fn soundpack_names(&self) -> Vec<String> {
        self.soundpacks.iter().map(|p| p.name.clone()).collect()
    }

    pub fn set_soundpack(&self, index: usize) {
        if index < self.soundpacks.len() {
            self.current_pack_index.store(index as u32, Ordering::SeqCst);
        }
    }

    pub fn current_soundpack_index(&self) -> usize {
        self.current_pack_index.load(Ordering::SeqCst) as usize
    }

    pub fn set_volume(&self, percent: u32) {
        self.volume.store(percent.min(150), Ordering::SeqCst);
    }

    pub fn get_volume(&self) -> u32 {
        self.volume.load(Ordering::SeqCst)
    }

    /// Upstroke (key release) level, independent of the master volume. 0 silences
    /// release clicks entirely while leaving keypresses untouched.
    pub fn set_release_volume(&self, percent: u32) {
        self.release_volume.store(percent.min(150), Ordering::SeqCst);
    }

    pub fn get_release_volume(&self) -> u32 {
        self.release_volume.load(Ordering::SeqCst)
    }

    /// Per-profile trim, so a naturally louder switch profile can be balanced against
    /// the others without touching the master volume.
    pub fn set_pack_volume(&self, index: usize, percent: u32) {
        if let Some(slot) = self.pack_volumes.get(index) {
            slot.store(percent.min(150), Ordering::SeqCst);
        }
    }

    pub fn get_pack_volume(&self, index: usize) -> u32 {
        self.pack_volumes
            .get(index)
            .map(|v| v.load(Ordering::SeqCst))
            .unwrap_or(100)
    }

    pub fn toggle_mute(&self) -> bool {
        let current = self.is_muted.load(Ordering::SeqCst);
        let next = !current;
        self.is_muted.store(next, Ordering::SeqCst);
        next
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted.load(Ordering::SeqCst)
    }

    /// Resolved playback gain for a stroke: master x (release trim, upstrokes only)
    /// x current profile trim, capped to keep the sum out of clipping territory.
    pub fn effective_gain(&self, is_press: bool) -> f32 {
        let master = self.volume.load(Ordering::Relaxed) as f32 / 100.0;
        let release = if is_press {
            1.0
        } else {
            self.release_volume.load(Ordering::Relaxed) as f32 / 100.0
        };
        let pack_idx = self.current_pack_index.load(Ordering::Relaxed) as usize;
        let pack = self.get_pack_volume(pack_idx) as f32 / 100.0;

        (master * release * pack).clamp(0.0, MAX_COMBINED_GAIN)
    }

    pub fn play_key(&self, stream_handle: &OutputStreamHandle, key_type: KeyType, is_press: bool, pan: f32) {
        if self.is_muted.load(Ordering::Relaxed) {
            return;
        }

        let base_volume = self.effective_gain(is_press);
        if base_volume <= 0.0 {
            return;
        }

        let pack_idx = self.current_pack_index.load(Ordering::Relaxed) as usize;
        let pack = match self.soundpacks.get(pack_idx) {
            Some(p) => p,
            None => return,
        };

        let sample_buffer = pack.get_sample(key_type, is_press);

        // Organic micro-variations so keystrokes don't sound robotic
        let pitch_jitter = 1.0 + (fastrand::f32() - 0.5) * 0.05;
        let vol_jitter = base_volume * (1.0 + (fastrand::f32() - 0.5) * 0.06);

        // Natural Stereo Panning with Deskmat Cross-Feed (Haas Effect)
        // Eliminates the "shallow" mono feeling by giving true physical acoustic space
        let left_gain = ((1.0 - pan) * 0.55).clamp(0.18, 0.92);
        let right_gain = ((1.0 + pan) * 0.55).clamp(0.18, 0.92);

        // 1.5ms room reflection delay (approx 66 samples at 44.1kHz)
        let delay_samples = 66;
        let n = sample_buffer.len();
        let mut stereo_samples = Vec::with_capacity(n * 2);

        for i in 0..n {
            let direct = sample_buffer[i];
            let delayed = if i >= delay_samples { sample_buffer[i - delay_samples] * 0.25 } else { 0.0 };

            let l = direct * left_gain + delayed * right_gain * 0.45;
            let r = direct * right_gain + delayed * left_gain * 0.45;

            stereo_samples.push(l);
            stereo_samples.push(r);
        }

        let source = SamplesBuffer::new(2, pack.sample_rate, stereo_samples)
            .amplify(vol_jitter)
            .speed(pitch_jitter);

        let _ = stream_handle.play_raw(source.convert_samples());
    }
}
