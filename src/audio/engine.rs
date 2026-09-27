use rodio::buffer::SamplesBuffer;
use rodio::{OutputStreamHandle, Source};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use super::generator::KeyType;
use super::soundpack::{get_bundled_soundpacks, SwitchSoundpack};

pub struct AudioState {
    soundpacks: Vec<SwitchSoundpack>,
    current_pack_index: AtomicU32,
    volume: AtomicU32, // Percentage 0..150
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

        Self {
            soundpacks,
            current_pack_index: AtomicU32::new(0),
            volume: AtomicU32::new(75),
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

    pub fn toggle_mute(&self) -> bool {
        let current = self.is_muted.load(Ordering::SeqCst);
        let next = !current;
        self.is_muted.store(next, Ordering::SeqCst);
        next
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted.load(Ordering::SeqCst)
    }

    pub fn play_key(&self, stream_handle: &OutputStreamHandle, key_type: KeyType, is_press: bool) {
        if self.is_muted.load(Ordering::Relaxed) {
            return;
        }

        let pack_idx = self.current_pack_index.load(Ordering::Relaxed) as usize;
        let pack = match self.soundpacks.get(pack_idx) {
            Some(p) => p,
            None => return,
        };

        let sample_buffer = pack.get_sample(key_type, is_press);
        let volume_percent = self.volume.load(Ordering::Relaxed) as f32;
        let base_volume = (volume_percent / 100.0).clamp(0.0, 1.5);

        // Organic micro-variations so keystrokes don't sound robotic:
        // +/- 3% pitch variation
        let pitch_jitter = 1.0 + (fastrand::f32() - 0.5) * 0.06;
        // +/- 4% volume variation
        let vol_jitter = base_volume * (1.0 + (fastrand::f32() - 0.5) * 0.08);

        let source = SamplesBuffer::new(1, pack.sample_rate, (*sample_buffer).clone())
            .amplify(vol_jitter)
            .speed(pitch_jitter);

        let _ = stream_handle.play_raw(source.convert_samples());
    }
}
