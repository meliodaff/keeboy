use keeboy::audio::generator::{
    generate_switch_sample, KeyType, EIGHTBITDO_RETRO, EIGHTBITDO_SUPER_BUTTONS,
    PURE_DEEP_THOCK, ULTRA_CREAMY,
};
use keeboy::audio::soundpack::get_bundled_soundpacks;
use keeboy::audio::AudioState;

#[test]
fn test_switch_sample_generation_and_clamping() {
    let profiles = [
        &EIGHTBITDO_RETRO,
        &EIGHTBITDO_SUPER_BUTTONS,
        &PURE_DEEP_THOCK,
        &ULTRA_CREAMY,
    ];
    let key_types = [
        KeyType::Regular,
        KeyType::Space,
        KeyType::Enter,
        KeyType::Backspace,
    ];

    for profile in profiles {
        for key in key_types {
            // Test downstroke (press)
            let press_samples = generate_switch_sample(profile, key, true);
            assert!(!press_samples.is_empty(), "Press samples should not be empty for {}", profile.name);
            for s in &press_samples {
                assert!(*s >= -1.0 && *s <= 1.0, "Sample out of bounds: {}", s);
            }

            // Test upstroke (release)
            let release_samples = generate_switch_sample(profile, key, false);
            assert!(!release_samples.is_empty(), "Release samples should not be empty for {}", profile.name);
            for s in &release_samples {
                assert!(*s >= -1.0 && *s <= 1.0, "Sample out of bounds: {}", s);
            }

            // Spacebar press duration should be longer than regular press duration
            if key == KeyType::Space {
                let reg_press = generate_switch_sample(profile, KeyType::Regular, true);
                assert!(press_samples.len() > reg_press.len(), "Spacebar acoustic cavity should be longer than 1u regular key");
            }
        }
    }
}

#[test]
fn test_soundpack_bundling_and_state() {
    let packs = get_bundled_soundpacks();
    assert_eq!(packs.len(), 4, "Expected 4 bundled switch soundpacks");
    assert!(packs[0].name.contains("8BitDo Retro"));
    assert!(packs[1].name.contains("Super Buttons"));
    assert!(packs[2].name.contains("Pure Deep Thock"));
    assert!(packs[3].name.contains("Ultra Creamy"));

    let state = AudioState::new();
    assert_eq!(state.current_soundpack_index(), 0);
    assert_eq!(state.get_volume(), 75);
    assert!(!state.is_muted());

    state.set_volume(100);
    assert_eq!(state.get_volume(), 100);

    let muted = state.toggle_mute();
    assert!(muted);
    assert!(state.is_muted());

    state.set_soundpack(1);
    assert_eq!(state.current_soundpack_index(), 1);
}

#[test]
fn test_independent_volume_controls() {
    let state = AudioState::new();

    // Defaults: master 75%, release at full relative level, every profile untrimmed.
    assert_eq!(state.get_volume(), 75);
    assert_eq!(state.get_release_volume(), 100);
    assert_eq!(state.get_pack_volume(0), 100);

    // Release volume is independent of master: presses stay put when it changes.
    state.set_volume(100);
    state.set_release_volume(50);
    assert!((state.effective_gain(true) - 1.0).abs() < 1e-6, "press gain should ignore release trim");
    assert!((state.effective_gain(false) - 0.5).abs() < 1e-6, "release gain should be halved");

    // Release can be silenced entirely without muting keypresses.
    state.set_release_volume(0);
    assert_eq!(state.effective_gain(false), 0.0);
    assert!(state.effective_gain(true) > 0.0);

    // Per-profile trim applies only to the selected profile.
    state.set_release_volume(100);
    state.set_pack_volume(0, 50);
    state.set_pack_volume(1, 150);
    assert_eq!(state.get_pack_volume(0), 50);
    assert_eq!(state.get_pack_volume(1), 150);

    state.set_soundpack(0);
    assert!((state.effective_gain(true) - 0.5).abs() < 1e-6);
    state.set_soundpack(1);
    assert!((state.effective_gain(true) - 1.5).abs() < 1e-6);

    // Stacked controls stay clamped so they can't drive the output into clipping.
    state.set_volume(150);
    state.set_release_volume(150);
    state.set_pack_volume(1, 150);
    assert!(state.effective_gain(false) <= 1.6, "combined gain must stay capped");

    // Out-of-range requests are clamped, and unknown profile indexes are inert.
    state.set_volume(500);
    assert_eq!(state.get_volume(), 150);
    state.set_release_volume(500);
    assert_eq!(state.get_release_volume(), 150);
    state.set_pack_volume(999, 100);
    assert_eq!(state.get_pack_volume(999), 100, "missing profile falls back to 100%");
}
