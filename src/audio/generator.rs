use std::f32::consts::PI;

pub struct SwitchAcousticProfile {
    pub name: &'static str,
    pub is_click_bar: bool,
    pub click_freq: f32,
    pub plate_freq: f32,
    pub case_resonance_freq: f32,
    pub body_thump_freq: f32,
    pub spring_ping_freq: f32,
    pub up_freq: f32,
    pub space_multiplier: f32,
}

/// 8BitDo Retro Mechanical Keyboard (NES / Famicom / C64 Edition):
/// Deep, full-bodied Kailh Box White V2 click-bar switches with rich retro case resonance,
/// solid plate impact, and signature upstroke reset double-click.
pub static EIGHTBITDO_RETRO: SwitchAcousticProfile = SwitchAcousticProfile {
    name: "8BitDo Retro (Kailh Box White V2 Click Bar)",
    is_click_bar: true,
    click_freq: 3350.0,
    plate_freq: 520.0,
    case_resonance_freq: 230.0,
    body_thump_freq: 135.0, // Rich, full-bodied low-end impact prevents "shallow" feel
    spring_ping_freq: 1950.0,
    up_freq: 3600.0,
    space_multiplier: 0.60,
};

/// 8BitDo Dual Super Buttons (Big arcade slam + heavy spring bounce)
pub static EIGHTBITDO_SUPER_BUTTONS: SwitchAcousticProfile = SwitchAcousticProfile {
    name: "8BitDo Super Buttons (Retro Arcade Slam)",
    is_click_bar: false,
    click_freq: 1650.0,
    plate_freq: 320.0,
    case_resonance_freq: 150.0,
    body_thump_freq: 95.0,
    spring_ping_freq: 1250.0,
    up_freq: 420.0,
    space_multiplier: 0.52,
};

/// Pure Deep Thock (Heavy Marble / Tape Mod)
pub static PURE_DEEP_THOCK: SwitchAcousticProfile = SwitchAcousticProfile {
    name: "Pure Deep Thock (Heavy Marble / Tape Mod)",
    is_click_bar: false,
    click_freq: 450.0,
    plate_freq: 175.0,
    case_resonance_freq: 85.0,
    body_thump_freq: 65.0,
    spring_ping_freq: 320.0,
    up_freq: 280.0,
    space_multiplier: 0.50,
};

/// Ultra Creamy (Milky Yellow / 205g0 Bubble Pop)
pub static ULTRA_CREAMY: SwitchAcousticProfile = SwitchAcousticProfile {
    name: "Ultra Creamy (Milky Yellow / KTT Strawberry)",
    is_click_bar: false,
    click_freq: 820.0,
    plate_freq: 440.0,
    case_resonance_freq: 220.0,
    body_thump_freq: 110.0,
    spring_ping_freq: 620.0,
    up_freq: 360.0,
    space_multiplier: 0.58,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyType {
    Regular,
    Space,
    Enter,
    Backspace,
}

/// Deterministic xorshift32 white-noise source.
///
/// Real switch, plate and case impacts are broadband noise bursts ringing through
/// material — not pure tones. Sine-only synthesis is exactly what makes a keyboard
/// sound thin and "shallow", so every impact layer below is noise-excited.
struct Noise(u32);

impl Noise {
    fn new(seed: u32) -> Self {
        Noise(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    #[inline]
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        ((x >> 8) as f32 / 8_388_608.0) - 1.0
    }
}

/// Chamberlin state-variable band-pass, used as a material resonator: white noise in,
/// "struck steel / aluminium plate / plastic housing" out.
struct Resonator {
    f: f32,
    q: f32,
    low: f32,
    band: f32,
}

impl Resonator {
    fn new(freq: f32, q_factor: f32, sample_rate: f32) -> Self {
        let clamped = freq.clamp(20.0, sample_rate * 0.24);
        Self {
            f: (2.0 * (PI * clamped / sample_rate).sin()).clamp(0.0, 1.2),
            q: (1.0 / q_factor).clamp(0.02, 2.0),
            low: 0.0,
            band: 0.0,
        }
    }

    #[inline]
    fn strike(&mut self, input: f32) -> f32 {
        self.low += self.f * self.band;
        let high = input - self.low - self.q * self.band;
        self.band += self.f * high;
        self.band
    }
}

/// One-pole low-pass. Shapes noise into the warm room/air tail that gives the
/// keystroke a sense of physical depth instead of a flat click.
struct AirFilter {
    a: f32,
    y: f32,
}

impl AirFilter {
    fn new(cutoff: f32, sample_rate: f32) -> Self {
        Self {
            a: 1.0 - (-2.0 * PI * cutoff / sample_rate).exp(),
            y: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        self.y += (x - self.y) * self.a;
        self.y
    }
}

/// Generates a realistic 44.1kHz mono PCM buffer with physical acoustic modeling and deep body
pub fn generate_switch_sample(
    profile: &SwitchAcousticProfile,
    key_type: KeyType,
    is_press: bool,
) -> Vec<f32> {
    let sample_rate = 44100.0;

    // Longer tails than a bare click: the low-end body and case resonance need room to
    // decay naturally, otherwise the sound is truncated before it develops any depth.
    let duration = match (key_type, is_press) {
        (KeyType::Space, true) => 0.220,
        (KeyType::Space, false) => 0.095,
        (KeyType::Enter | KeyType::Backspace, true) => 0.175,
        (KeyType::Enter | KeyType::Backspace, false) => 0.085,
        (KeyType::Regular, true) => 0.155,
        (KeyType::Regular, false) => 0.078,
    };

    let total_samples = (sample_rate * duration) as usize;
    let mut buffer = Vec::with_capacity(total_samples);

    let key_pitch_mult = match key_type {
        KeyType::Space => profile.space_multiplier,
        KeyType::Enter => (profile.space_multiplier + 1.0) * 0.50,
        KeyType::Backspace => (profile.space_multiplier + 1.0) * 0.54,
        KeyType::Regular => 1.0,
    };

    // Fixed seed per (profile, key, stroke) so each baked buffer is stable; per-keystroke
    // variation comes from the engine's pitch/volume jitter.
    let seed = (profile.click_freq as u32).wrapping_mul(2_654_435_761)
        ^ (key_type as u32 + 1).wrapping_mul(0x9E37_79B9)
        ^ if is_press { 0x5BF0_3635 } else { 0xB5D2_1C7F };
    let mut noise = Noise::new(seed);

    let plate_freq = profile.plate_freq * key_pitch_mult;
    let case_freq = profile.case_resonance_freq * key_pitch_mult;
    let body_freq = profile.body_thump_freq * key_pitch_mult;

    // Big keys carry much more low-end; without this the added body would swallow the
    // click-bar snap once the buffer is peak-normalized.
    let treble_comp = match key_type {
        KeyType::Space => 1.65,
        KeyType::Enter | KeyType::Backspace => 1.12,
        KeyType::Regular => 1.0,
    };

    // Big keys already get their own chamber layer, so their switch-body low content is
    // trimmed to stop the low end from eating all the normalized headroom.
    let low_trim = match key_type {
        KeyType::Space => 0.70,
        KeyType::Enter | KeyType::Backspace => 0.88,
        KeyType::Regular => 1.0,
    };

    // Material resonators
    let mut contact_res = Resonator::new(profile.click_freq * 1.15, 1.1, sample_rate);
    let mut plate_res = Resonator::new(plate_freq, 2.4, sample_rate);
    let mut case_res = Resonator::new(case_freq * 1.9, 3.2, sample_rate);
    let mut air = AirFilter::new(if profile.is_click_bar { 900.0 } else { 620.0 }, sample_rate);

    for i in 0..total_samples {
        let t = i as f32 / sample_rate;
        let n = noise.next();

        let sample = if is_press {
            if profile.is_click_bar {
                // === 8BITDO RETRO (KAILH BOX WHITE V2) WITH FULL BODY & ACOUSTIC DEPTH ===
                // 1. The crisp steel click-bar snap (tonal core — unchanged character)
                let click_decay = (-t * 820.0).exp();
                let click_snap = ((2.0 * PI * profile.click_freq * t).sin() * 0.65
                    + (2.0 * PI * (profile.click_freq * 1.35) * t).sin() * 0.28
                    + (2.0 * PI * (profile.click_freq * 0.68) * t).sin() * 0.25)
                    * click_decay;

                // 2. Broadband steel contact crack — the transient "grit" a pure sine can't make
                let contact = contact_res.strike(n) * (-t * 1400.0).exp() * 0.68 * treble_comp;

                // 3. Vintage aluminum top-plate impact: tonal ring + noise-excited strike
                let plate_tone = (2.0 * PI * plate_freq * t).sin() * (-t * 58.0).exp() * 0.40;
                let plate_hit = plate_res.strike(n) * (-t * 165.0).exp() * 0.58 * treble_comp;

                // 4. Solid low-end body thump plus a sub-octave for real physical weight.
                // Kept deliberately under the click so it reads as depth, not boom.
                let body = (2.0 * PI * body_freq * t).sin() * (-t * 34.0).exp() * 0.46 * low_trim;
                let sub =
                    (2.0 * PI * (body_freq * 0.5) * t).sin() * (-t * 26.0).exp() * 0.22 * low_trim;

                // 5. Hollow retro ABS chamber — multiple inharmonic modes, not one sine
                let case = ((2.0 * PI * case_freq * t).sin() * (-t * 34.0).exp() * 0.32
                    + (2.0 * PI * (case_freq * 1.47) * t).sin() * (-t * 42.0).exp() * 0.19
                    + (2.0 * PI * (case_freq * 2.09) * t).sin() * (-t * 56.0).exp() * 0.12)
                    * low_trim
                    + case_res.strike(n) * (-t * 90.0).exp() * 0.22;

                // 6. Room / desk air tail: lowpassed noise decaying slowly behind the hit
                let room = air.process(n) * (-t * 30.0).exp() * 0.20;

                // 7. Subtle spring ping resonance
                let spring = (2.0 * PI * profile.spring_ping_freq * t).sin()
                    * (-t * 140.0).exp()
                    * 0.08;

                // 8. Spacebar deep hollow chamber + stabilizer wire acoustic tap
                let wire = if key_type == KeyType::Space {
                    let chamber = (2.0 * PI * 95.0 * t).sin() * (-t * 22.0).exp() * 0.34
                        + (2.0 * PI * 140.0 * t).sin() * (-t * 30.0).exp() * 0.16;
                    let chamber_sub = (2.0 * PI * 62.0 * t).sin() * (-t * 18.0).exp() * 0.14;
                    let tick = (2.0 * PI * 1450.0 * t).sin() * (-t * 160.0).exp() * 0.14;
                    chamber + chamber_sub + tick
                } else {
                    0.0
                };

                let attack = (t / 0.0006).min(1.0);
                let combined = (click_snap * 1.05 * treble_comp
                    + contact
                    + plate_tone
                    + plate_hit
                    + body
                    + sub
                    + case
                    + room
                    + spring
                    + wire)
                    * attack;

                // Warm analog body saturation: fattens the transient so the snap keeps its
                // bite even with the heavier low end underneath it.
                (combined * 1.35).tanh()
            } else {
                // === LINEAR / TACTILE THOCK MODELS ===
                let thock = (2.0 * PI * body_freq * t).sin() * (-t * 30.0).exp() * 0.85;
                let sub = (2.0 * PI * (body_freq * 0.5) * t).sin() * (-t * 22.0).exp() * 0.55;

                let cavity = (2.0 * PI * case_freq * t).sin() * (-t * 40.0).exp() * 0.45
                    + (2.0 * PI * (case_freq * 1.53) * t).sin() * (-t * 55.0).exp() * 0.20;

                // Noise-excited bottom-out knock + plastic contact transient
                let knock = plate_res.strike(n) * (-t * 130.0).exp() * 0.50;
                let contact = contact_res.strike(n) * (-t * 850.0).exp() * 0.30;
                let room = air.process(n) * (-t * 22.0).exp() * 0.26;

                let wire = if key_type == KeyType::Space {
                    (2.0 * PI * (body_freq * 0.72) * t).sin() * (-t * 16.0).exp() * 0.40
                } else {
                    0.0
                };

                let attack = (t / 0.0008).min(1.0);
                ((thock + sub + cavity + knock + contact + room + wire) * attack * 1.1).tanh()
            }
        } else {
            // === UPSTROKE (RELEASE) ===
            if profile.is_click_bar {
                // Kailh Box White V2 upstroke click-bar reset snap + top housing clack
                let reset_decay = (-t * 880.0).exp();
                let reset_click = ((2.0 * PI * profile.up_freq * t).sin() * 0.60
                    + (2.0 * PI * (profile.up_freq * 1.25) * t).sin() * 0.22)
                    * reset_decay;
                let reset_grit = contact_res.strike(n) * (-t * 1500.0).exp() * 0.40;

                // Top housing return clack with retro case body
                let top_clack =
                    (2.0 * PI * 640.0 * key_pitch_mult * t).sin() * (-t * 130.0).exp() * 0.34;
                let housing = plate_res.strike(n) * (-t * 210.0).exp() * 0.32;
                let case_return = (2.0 * PI * (case_freq * 1.2) * t).sin() * (-t * 60.0).exp() * 0.18;
                let body_return =
                    (2.0 * PI * (body_freq * 1.1) * t).sin() * (-t * 55.0).exp() * 0.15;
                let room = air.process(n) * (-t * 55.0).exp() * 0.12;

                let attack = (t / 0.0005).min(1.0);
                let up_sample = (reset_click * 0.62
                    + reset_grit
                    + top_clack
                    + housing
                    + case_return
                    + body_return
                    + room)
                    * attack;
                (up_sample * 1.05).tanh()
            } else {
                let up_clack = (2.0 * PI * profile.up_freq * t).sin() * (-t * 120.0).exp() * 0.40;
                let housing = plate_res.strike(n) * (-t * 190.0).exp() * 0.35;
                let body_return = (2.0 * PI * body_freq * t).sin() * (-t * 50.0).exp() * 0.28;
                let room = air.process(n) * (-t * 38.0).exp() * 0.16;
                let attack = (t / 0.0008).min(1.0);
                ((up_clack + housing + body_return + room) * attack * 1.05).tanh()
            }
        };

        buffer.push(sample);
    }

    // Peak-normalize instead of relying on hard clipping: keeps the transient punch and
    // dynamic range intact, which is a large part of why the hit reads as "deep".
    let peak = buffer.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let target = if is_press { 0.92 } else { 0.44 };
    let gain = if peak > 1e-6 { target / peak } else { 0.0 };

    // Smooth 6ms tail fade so the longer decay never truncates into an audible edge click.
    let fade_len = ((sample_rate * 0.006) as usize).max(1).min(total_samples);

    for (i, s) in buffer.iter_mut().enumerate() {
        let mut v = *s * gain;
        let remaining = total_samples - i;
        if remaining < fade_len {
            let x = remaining as f32 / fade_len as f32;
            v *= (x * PI * 0.5).sin();
        }
        *s = v.clamp(-1.0, 1.0);
    }

    buffer
}
