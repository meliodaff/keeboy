// Release builds run as a GUI app: no console window, just the system tray icon.
// Debug builds keep the console so `cargo run` still shows the banner and status log.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod hook;
mod tray;

use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use rodio::OutputStream;
use audio::AudioState;
use hook::{start_keyboard_hook, KeyAudioEvent};
use tray::{alert, run_tray_loop, SingleInstance};

fn main() {
    // Only one hook may own the keyboard, otherwise every keystroke double-fires.
    let _instance_guard = match SingleInstance::acquire("Keeboy_SingleInstance_Mutex") {
        Some(guard) => guard,
        None => {
            alert(
                "Keeboy is already running",
                "Look for the keycap icon in your system tray (near the clock).\n\
                 Right-click it to change switches, adjust volume, or exit.",
            );
            return;
        }
    };

    println!(r#"
  ███████╗  ███████╗  ███████╗ ██████╗   ██████╗  ██╗   ██╗
  ██╔════╝  ██╔════╝  ██╔════╝ ██╔══██╗ ██╔═══██╗ ╚██╗ ██╔╝
  █████╗    █████╗    █████╗   ██████╔╝ ██║   ██║  ╚████╔╝ 
  ██╔══╝    ██╔══╝    ██╔══╝   ██╔══██╗ ██║   ██║   ╚██╔╝  
  ███████╗  ███████╗  ███████╗ ██████╔╝ ╚██████╔╝    ██║   
  ╚══════╝  ╚══════╝  ╚══════╝ ╚═════╝   ╚═════╝     ╚═╝   
    Lightweight Mechanical Keyboard Sound Engine (v0.1.0)
    "#);

    println!("[Keeboy] Initializing audio banks...");
    let audio_state = Arc::new(AudioState::new());

    println!("[Keeboy] Loaded Switch Profiles:");
    for (i, name) in audio_state.soundpack_names().iter().enumerate() {
        let active = if i == audio_state.current_soundpack_index() { " (Active)" } else { "" };
        println!("   [{}] {}{}", i + 1, name, active);
    }

    // High-priority audio channel
    let (tx, rx) = sync_channel::<KeyAudioEvent>(256);

    // Audio dispatcher thread: owns OutputStream (WASAPI) and plays incoming key events
    let audio_state_worker = Arc::clone(&audio_state);
    let _audio_thread = std::thread::Builder::new()
        .name("keeboy-audio-worker".into())
        .spawn(move || {
            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(res) => res,
                Err(err) => {
                    eprintln!("[Keeboy] FATAL: Failed to open default audio output device: {}", err);
                    // No console in release, so the failure has to be visible somewhere.
                    alert(
                        "Keeboy: no audio output device",
                        &format!(
                            "Could not open the default Windows audio output device.\n\n{}\n\n\
                             Check your playback device, then start Keeboy again.",
                            err
                        ),
                    );
                    return;
                }
            };

            // Label our Volume Mixer slider now that the session exists, so Keeboy can
            // be turned down independently of music, games and calls.
            match audio::session::label_mixer_session("Keeboy") {
                Ok(()) => println!("[Keeboy] Volume Mixer session labelled 'Keeboy'."),
                Err(hr) => eprintln!(
                    "[Keeboy] Note: could not label the Volume Mixer session (hr=0x{:08X}). \
                     The slider still works.",
                    hr
                ),
            }

            while let Ok(event) = rx.recv() {
                audio_state_worker.play_key(&stream_handle, event.key_type, event.is_press, event.pan);
            }
        })
        .expect("Failed to spawn audio worker thread");

    println!("[Keeboy] Installing global Windows keyboard hook...");
    let _hook_thread = start_keyboard_hook(tx);

    println!("[Keeboy] Ready! System tray icon is active.");
    println!("[Keeboy] -> Type anywhere in Windows to produce mechanical switch sounds.");
    println!("[Keeboy] -> Right-click the Keeboy icon in your system tray to change switches or volume.");

    // Run tray loop on main thread
    run_tray_loop(Arc::clone(&audio_state));

    println!("[Keeboy] Shutdown complete. Goodbye!");
}
