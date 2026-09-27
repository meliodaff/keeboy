use std::path::Path;
use std::sync::Arc;
use super::generator::{
    generate_switch_sample, KeyType, SwitchAcousticProfile, EIGHTBITDO_RETRO,
    EIGHTBITDO_SUPER_BUTTONS, PURE_DEEP_THOCK, ULTRA_CREAMY,
};

#[derive(Clone)]
pub struct SwitchSoundpack {
    pub name: String,
    pub sample_rate: u32,
    pub press_regular: Arc<Vec<f32>>,
    pub release_regular: Arc<Vec<f32>>,
    pub press_space: Arc<Vec<f32>>,
    pub release_space: Arc<Vec<f32>>,
    pub press_enter: Arc<Vec<f32>>,
    pub release_enter: Arc<Vec<f32>>,
    pub press_backspace: Arc<Vec<f32>>,
    pub release_backspace: Arc<Vec<f32>>,
}

impl SwitchSoundpack {
    pub fn from_profile(profile: &SwitchAcousticProfile) -> Self {
        Self {
            name: profile.name.to_string(),
            sample_rate: 44100,
            press_regular: Arc::new(generate_switch_sample(profile, KeyType::Regular, true)),
            release_regular: Arc::new(generate_switch_sample(profile, KeyType::Regular, false)),
            press_space: Arc::new(generate_switch_sample(profile, KeyType::Space, true)),
            release_space: Arc::new(generate_switch_sample(profile, KeyType::Space, false)),
            press_enter: Arc::new(generate_switch_sample(profile, KeyType::Enter, true)),
            release_enter: Arc::new(generate_switch_sample(profile, KeyType::Enter, false)),
            press_backspace: Arc::new(generate_switch_sample(profile, KeyType::Backspace, true)),
            release_backspace: Arc::new(generate_switch_sample(profile, KeyType::Backspace, false)),
        }
    }

    pub fn get_sample(&self, key_type: KeyType, is_press: bool) -> Arc<Vec<f32>> {
        match (key_type, is_press) {
            (KeyType::Space, true) => Arc::clone(&self.press_space),
            (KeyType::Space, false) => Arc::clone(&self.release_space),
            (KeyType::Enter, true) => Arc::clone(&self.press_enter),
            (KeyType::Enter, false) => Arc::clone(&self.release_enter),
            (KeyType::Backspace, true) => Arc::clone(&self.press_backspace),
            (KeyType::Backspace, false) => Arc::clone(&self.release_backspace),
            (KeyType::Regular, true) => Arc::clone(&self.press_regular),
            (KeyType::Regular, false) => Arc::clone(&self.release_regular),
        }
    }

    /// Try loading a soundpack from a directory containing wav files:
    /// press.wav, release.wav, space.wav, enter.wav, backspace.wav
    pub fn try_load_from_folder(dir: &Path) -> Option<Self> {
        let name = dir.file_name()?.to_string_lossy().to_string();

        let load_wav = |filename: &str| -> Option<Vec<f32>> {
            let path = dir.join(filename);
            let reader = hound::WavReader::open(path).ok()?;
            let spec = reader.spec();
            let samples: Vec<f32> = match spec.sample_format {
                hound::SampleFormat::Int => {
                    let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                    reader.into_samples::<i32>().filter_map(|s| s.ok().map(|v| v as f32 / max_val)).collect()
                }
                hound::SampleFormat::Float => {
                    reader.into_samples::<f32>().filter_map(|s| s.ok()).collect()
                }
            };
            if samples.is_empty() {
                None
            } else {
                Some(samples)
            }
        };

        let press_regular = Arc::new(load_wav("press.wav").or_else(|| load_wav("press_regular.wav"))?);
        let release_regular = Arc::new(load_wav("release.wav").or_else(|| load_wav("release_regular.wav")).unwrap_or_else(|| (*press_regular).clone()));
        let press_space = Arc::new(load_wav("space.wav").or_else(|| load_wav("press_space.wav")).unwrap_or_else(|| (*press_regular).clone()));
        let release_space = Arc::new(load_wav("release_space.wav").unwrap_or_else(|| (*release_regular).clone()));
        let press_enter = Arc::new(load_wav("enter.wav").or_else(|| load_wav("press_enter.wav")).unwrap_or_else(|| (*press_regular).clone()));
        let release_enter = Arc::new(load_wav("release_enter.wav").unwrap_or_else(|| (*release_regular).clone()));
        let press_backspace = Arc::new(load_wav("backspace.wav").or_else(|| load_wav("press_backspace.wav")).unwrap_or_else(|| (*press_regular).clone()));
        let release_backspace = Arc::new(load_wav("release_backspace.wav").unwrap_or_else(|| (*release_regular).clone()));

        Some(Self {
            name,
            sample_rate: 44100,
            press_regular,
            release_regular,
            press_space,
            release_space,
            press_enter,
            release_enter,
            press_backspace,
            release_backspace,
        })
    }
}

pub fn get_bundled_soundpacks() -> Vec<SwitchSoundpack> {
    vec![
        SwitchSoundpack::from_profile(&EIGHTBITDO_RETRO),
        SwitchSoundpack::from_profile(&EIGHTBITDO_SUPER_BUTTONS),
        SwitchSoundpack::from_profile(&PURE_DEEP_THOCK),
        SwitchSoundpack::from_profile(&ULTRA_CREAMY),
    ]
}
