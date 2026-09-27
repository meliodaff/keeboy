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
