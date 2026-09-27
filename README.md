# ⌨️ Keeboy — Lightweight Mechanical Keyboard Sound Engine

A high-performance, ultra-lightweight Windows utility written in Rust that plays authentic tactile & thocky mechanical keyboard switch sounds as you type anywhere on your PC.

```
  ███████╗  ███████╗  ███████╗ ██████╗   ██████╗  ██╗   ██╗
  ██╔════╝  ██╔════╝  ██╔════╝ ██╔══██╗ ██╔═══██╗ ╚██╗ ██╔╝
  █████╗    █████╗    █████╗   ██████╔╝ ██║   ██║  ╚████╔╝ 
  ██╔══╝    ██╔══╝    ██╔══╝   ██╔══██╗ ██║   ██║   ╚██╔╝  
  ███████╗  ███████╗  ███████╗ ██████╔╝ ╚██████╔╝    ██║   
  ╚══════╝  ╚══════╝  ╚══════╝ ╚═════╝   ╚═════╝     ╚═╝   
```

---

## ✨ Features

- **⚡ Ultra-Lightweight & Efficient**:
  - Single standalone binary: only **~315 KB**.
  - Tiny memory footprint: **< 15 MB RAM** (vs 150MB+ for Electron-based apps).
  - Near **0% idle CPU** usage.
- **🚀 Sub-5ms Instant Response**:
  - Built with native Windows Low-Level Keyboard Hooks (`WH_KEYBOARD_LL`).
  - Pre-decoded in-memory PCM audio buffers with Direct WASAPI audio dispatch.
- **🎧 Switch Profiles Included**:
  - **8BitDo Retro** *(Default)* (Full-bodied Kailh Box White V2 Click Bar, retro aluminum plate clack & double-click reset)
  - **8BitDo Super Buttons** (Giant retro arcade button slam + spring bounce)
  - **Pure Deep Thock** (Heavy marble / tape mod deep woody thock)
  - **Ultra Creamy** (Milky Yellow / 205g0 juicy bubble pop)
- **🔊 3D Spatial Stereo & Natural Room Depth**:
  - Real-time **horizontal stereo panning** based on key layout (left-hand keys pan left, right-hand keys pan right, spacebar center).
  - Binaural room cross-feed / deskmat Haas effect eliminates "shallow/inside-your-head" mono sound and places the keyboard on your physical desk.
- **🎹 Realistic Physical Switch Dynamics**:
  - Separate downstroke (press) and snappy upstroke (release) acoustic models.
  - Dedicated acoustic models for **Spacebar** (deep hollow cavity resonance & wire stabilizer tick), **Enter**, and **Backspace**.
  - Organic micro-pitch (±3%) and volume jitter on every keystroke to eliminate artificial repetition fatigue.
  - Intelligent auto-repeat filter (prevents unnatural machine-gun clicking when holding down a key).
- **🪟 Windows System Tray Integration**:
  - Sits quietly in your notification tray (by the clock).
  - Right-click tray menu to switch profiles, adjust volume (25%, 50%, 75%, 100%), mute, or exit.
- **📦 Custom Soundpack Support**:
  - Drop custom soundpack folders into `./soundpacks` (supports standard WAV files `press.wav`, `release.wav`, `space.wav`, `enter.wav`, `backspace.wav`).

---

## 🚀 Quick Start

### Running Keeboy

- **In Git Bash / bash**:
  ```bash
  ./target/release/keeboy.exe
  ```
- **In PowerShell / CMD**:
  ```powershell
  .\target\release\keeboy.exe
  ```

1. Once launched, you will see a terminal banner and a new icon in your **Windows System Tray** (notification area near the clock).
2. Start typing in **any** application (Notepad, browser, Discord, IDE, games).
3. Right-click the **Keeboy** icon in the system tray to:
   - Switch between **Deep Solid Thock**, **Boba U4T**, **Lubed Gateron Ink Black**, or **Holy Panda**.
   - Change volume levels.
   - Toggle mute.
   - Exit Keeboy.

---

## 🛠️ Building From Source

Requirements:
- Windows 10 / 11
- Rust toolchain (`cargo` + `rustc`)

```powershell
# Clone the repository
git clone https://github.com/your-username/keeboy.git
cd keeboy

# Run test suite
cargo test

# Build optimized release binary
cargo build --release
```

The optimized binary is output to `target\release\keeboy.exe`.

---

## 📁 Project Structure

```
keeboy/
├── Cargo.toml
├── src/
│   ├── main.rs            # Entrypoint, thread orchestration & console status
│   ├── lib.rs             # Core crate library root
│   ├── hook.rs            # Windows WH_KEYBOARD_LL hook & key mapping
│   ├── tray.rs            # Native Win32 system tray icon & context popup menu
│   └── audio/
│       ├── mod.rs         # Audio subsystem exports
│       ├── generator.rs   # Mathematical physical acoustic modeling for switches
│       ├── soundpack.rs   # Soundpack definitions, bundling, and folder loader
│       └── engine.rs      # In-memory low-latency audio dispatcher & state
└── tests/
    └── sound_tests.rs     # Automated test suite for acoustic buffers & state
```

---

## 📄 License
MIT
