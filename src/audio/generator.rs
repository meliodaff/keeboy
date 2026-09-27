use std::f32::consts::PI;

pub struct SwitchAcousticProfile {
    pub name: &'static str,
    pub is_click_bar: bool,
    pub click_freq: f32,
    pub plate_freq: f32,
    pub case_resonance_freq: f32,
    pub spring_ping_freq: f32,
    pub up_freq: f32,
    pub space_multiplier: f32,
}

/// 8BitDo Retro Mechanical Keyboard (NES / Famicom / C64 Edition):
/// Powered by Kailh Box White V2 click-bar switches with crisp downstroke snap,
/// vintage aluminum plate clack, retro hollow case resonance, and signature upstroke reset click!
pub static EIGHTBITDO_RETRO: SwitchAcousticProfile = SwitchAcousticProfile {
    name: "8BitDo Retro (Kailh Box White V2 Click Bar)",
    is_click_bar: true,
    click_freq: 3850.0,
    plate_freq: 620.0,
    case_resonance_freq: 260.0,
    spring_ping_freq: 2150.0,
    up_freq: 4100.0,
    space_multiplier: 0.65,
};

/// 8BitDo Dual Super Buttons (Big arcade slam + heavy spring bounce)
pub static EIGHTBITDO_SUPER_BUTTONS: SwitchAcousticProfile = SwitchAcousticProfile {
    name: "8BitDo Super Buttons (Retro Arcade Slam)",
    is_click_bar: false,
    click_freq: 1850.0,
    plate_freq: 340.0,
    case_resonance_freq: 160.0,
    spring_ping_freq: 1420.0,
    up_freq: 480.0,
    space_multiplier: 0.55,
};

/// Pure Deep Thock (Heavy Marble / Tape Mod)
pub static PURE_DEEP_THOCK: SwitchAcousticProfile = SwitchAcousticProfile {
    name: "Pure Deep Thock (Heavy Marble / Tape Mod)",
    is_click_bar: false,
    click_freq: 450.0,
    plate_freq: 180.0,
    case_resonance_freq: 85.0,
    spring_ping_freq: 350.0,
    up_freq: 310.0,
    space_multiplier: 0.52,
};

/// Ultra Creamy (Milky Yellow / 205g0 Bubble Pop)
pub static ULTRA_CREAMY: SwitchAcousticProfile = SwitchAcousticProfile {
    name: "Ultra Creamy (Milky Yellow / KTT Strawberry)",
    is_click_bar: false,
    click_freq: 820.0,
    plate_freq: 440.0,
    case_resonance_freq: 220.0,
    spring_ping_freq: 680.0,
    up_freq: 380.0,
    space_multiplier: 0.60,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyType {
    Regular,
    Space,
    Enter,
    Backspace,
}

/// Generates a realistic 44.1kHz mono PCM buffer for 8BitDo Retro & mechanical switches
pub fn generate_switch_sample(
    profile: &SwitchAcousticProfile,
    key_type: KeyType,
    is_press: bool,
) -> Vec<f32> {
    let sample_rate = 44100.0;
    let duration = match (key_type, is_press) {
        (KeyType::Space, true) => 0.110,
        (KeyType::Space, false) => 0.045,
        (KeyType::Enter | KeyType::Backspace, true) => 0.085,
        (KeyType::Enter | KeyType::Backspace, false) => 0.038,
        (KeyType::Regular, true) => 0.075,
        (KeyType::Regular, false) => 0.032,
    };

    let total_samples = (sample_rate * duration) as usize;
    let mut buffer = Vec::with_capacity(total_samples);

    let key_pitch_mult = match key_type {
        KeyType::Space => profile.space_multiplier,
        KeyType::Enter => (profile.space_multiplier + 1.0) * 0.52,
        KeyType::Backspace => (profile.space_multiplier + 1.0) * 0.56,
        KeyType::Regular => 1.0,
    };

    for i in 0..total_samples {
        let t = i as f32 / sample_rate;

        let sample = if is_press {
            if profile.is_click_bar {
                // === 8BITDO KAILH BOX WHITE V2 CLICK BAR (DOWNSTROKE) ===
                // 1. The iconic Click Bar snap: sharp, snappy, metallic ping
                let click_decay = (-t * 900.0).exp();
                let click_snap = ((2.0 * PI * profile.click_freq * t).sin() * 0.70
                    + (2.0 * PI * (profile.click_freq * 1.38) * t).sin() * 0.35)
                    * click_decay;

                // 2. Vintage aluminum plate bottom-out impact
                let plate_decay = (-t * 70.0).exp();
                let plate = (2.0 * PI * (profile.plate_freq * key_pitch_mult) * t).sin()
                    * plate_decay * 0.55;

                // 3. 8BitDo hollow retro casing body resonance (Famicom/NES ABS casing)
                let case_decay = (-t * 45.0).exp();
                let case = (2.0 * PI * (profile.case_resonance_freq * key_pitch_mult) * t).sin()
                    * case_decay * 0.38;

                // 4. Subtle internal spring shimmer
                let spring_decay = (-t * 160.0).exp();
                let spring = (2.0 * PI * profile.spring_ping_freq * t).sin()
                    * spring_decay * 0.08;

                // 5. Spacebar vintage hollow cavity + stabilizer wire tick
                let wire = if key_type == KeyType::Space {
                    let space_chamber = (2.0 * PI * 135.0 * t).sin() * (-t * 28.0).exp() * 0.45;
                    let space_tick = (2.0 * PI * 1600.0 * t).sin() * (-t * 180.0).exp() * 0.12;
                    space_chamber + space_tick
                } else {
                    0.0
                };

                let attack = (t / 0.0010).min(1.0);
                (click_snap * 0.95 + plate + case + spring + wire) * attack
            } else {
                // Deep / Creamy fallback modes
                let base_freq = profile.plate_freq * key_pitch_mult;
                let thock = (2.0 * PI * base_freq * t).sin() * (-t * 42.0).exp() * 0.85;
                let sub = (2.0 * PI * (base_freq * 0.5) * t).sin() * (-t * 32.0).exp() * 0.65;
                let cavity = (2.0 * PI * profile.case_resonance_freq * t).sin() * (-t * 55.0).exp() * 0.45;
                let attack = (t / 0.0020).min(1.0);
                ((thock + sub + cavity) * attack * 1.3).tanh()
            }
        } else {
            // === UPSTROKE (RELEASE) ===
            if profile.is_click_bar {
                // Kailh Box White V2 resets past the click bar on the upstroke:
                // producing the famous double-click reset snap!
                let reset_decay = (-t * 950.0).exp();
                let reset_click = ((2.0 * PI * profile.up_freq * t).sin() * 0.65
                    + (2.0 * PI * (profile.up_freq * 1.25) * t).sin() * 0.25)
                    * reset_decay;

                // Top housing contact
                let top_clack = (2.0 * PI * 720.0 * key_pitch_mult * t).sin()
                    * (-t * 160.0).exp() * 0.30;

                let attack = (t / 0.0008).min(1.0);
                (reset_click * 0.70 + top_clack) * attack * 0.75
            } else {
                let up_clack = (2.0 * PI * profile.up_freq * t).sin() * (-t * 150.0).exp() * 0.35;
                let attack = (t / 0.0015).min(1.0);
                up_clack * attack
            }
        };

        buffer.push(sample.clamp(-1.0, 1.0));
    }

    buffer
}
