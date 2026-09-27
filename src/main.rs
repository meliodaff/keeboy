mod audio;
mod hook;
mod tray;

use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use rodio::OutputStream;
use audio::AudioState;
use hook::{start_keyboard_hook, KeyAudioEvent};
use tray::run_tray_loop;

fn main() {
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
                    return;
                }
            };

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
