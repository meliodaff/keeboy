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
  - Right-click tray menu to switch profiles, adjust volume, mute, or exit.
- **🎚️ Three Independent Volume Controls**:
  - **Master volume** — overall level (10% to 125%).
  - **Key release volume** — upstroke level set separately from keypresses, including
    **Off** if you want presses only and no release click.
  - **Per-profile volume** — trim each switch profile individually so a naturally louder
    profile can be level-matched against the others.
  - All three stack into a capped gain, so they can never drive the output into clipping.
- **🔈 Its Own Windows Volume Mixer Slider**:
  - Appears in the Volume Mixer as **Keeboy** with the app icon, so typing sounds can be
    turned down independently of music, games and calls.
- **📦 Custom Soundpack Support**:
  - Drop custom soundpack folders into `./soundpacks` (supports standard WAV files `press.wav`, `release.wav`, `space.wav`, `enter.wav`, `backspace.wav`).

---

## 📥 Download

Grab `Keeboy-v<version>-win-x64.zip` from the [Releases](../../releases) page, extract it,
and either:

- **Just run it** — double-click `keeboy.exe`. No window opens; it goes straight to the
  system tray.
- **Install it** — right-click `Install-Keeboy.ps1` → *Run with PowerShell*. This copies
  Keeboy to `%LOCALAPPDATA%\Keeboy` and adds a Start Menu shortcut. Add `-Startup` to
  launch it at login, `-Uninstall` to remove it.

Requirements: **Windows 10 or 11 (64-bit)**. Nothing else — the MSVC runtime is statically
linked, so there is no Visual C++ Redistributable to install and no Rust toolchain needed.

Because the binary isn't code-signed, SmartScreen may warn "Windows protected your PC" on
first run — click *More info* → *Run anyway*. Keeboy installs a global keyboard hook to
know when to play a sound; it does not log, store, or transmit anything you type.

---

## 🚀 Quick Start (from source)

### Install as a Windows app (recommended)

This builds the release binary, copies it to `%LOCALAPPDATA%\Keeboy`, and creates a
Start Menu shortcut — no terminal needed afterwards.

```powershell
powershell -ExecutionPolicy Bypass -File scripts\install.ps1
```

| Flag | Effect |
|---|---|
| `-Startup` | Also launch Keeboy automatically when you log in to Windows |
| `-NoBuild` | Install the already-built `target\release\keeboy.exe` |
| `-NoLaunch` | Install without starting it |
| `-Uninstall` | Remove the installed copy and all shortcuts |

Once installed, launch it from the Start Menu (search "Keeboy"). Release builds run as a
GUI app — **no console window appears**, it just shows up in the system tray.

### Running the built binary directly

- **In PowerShell / CMD**: `.\target\release\keeboy.exe`
- **In Git Bash / bash**: `./target/release/keeboy.exe`

Release builds detach from the terminal entirely; debug builds (`cargo run`) keep the
console so you can see the startup banner and status log.

### Using Keeboy

1. Look for the keycap icon in your **Windows System Tray** (notification area near the clock).
2. Start typing in **any** application (Notepad, browser, Discord, IDE, games).
3. Right-click the **Keeboy** icon to:
   - Switch between **8BitDo Retro**, **8BitDo Super Buttons**, **Pure Deep Thock**, or **Ultra Creamy**.
   - Set **Master Volume**.
   - Set **Key Release Volume** separately (or turn release clicks off entirely).
   - Set **This Profile's Volume** to level-match the current profile against the others.
   - Toggle mute.
   - Exit Keeboy.

To balance Keeboy against everything else on your PC, open the Windows **Volume Mixer**
(right-click the speaker icon → *Open Volume mixer*) and move the **Keeboy** slider.
Volume settings are per-session and reset to defaults on restart.

Only one instance can run at a time — launching a second copy shows a reminder and exits,
so keystrokes never double up.

---

## 🛠️ Building From Source

Requirements:
- Windows 10 / 11
- Rust toolchain (`cargo` + `rustc`)
- Windows SDK (provides `rc.exe`, used to embed the app icon)

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

### Packaging a release for other people

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package.ps1
```

This builds, verifies the binary has no Visual C++ Redistributable dependency, and writes
`dist\Keeboy-v<version>-win-x64.zip` (binary + `Install-Keeboy.ps1` + quick start) along
with its SHA256. Upload that ZIP to a GitHub Release, or just push a version tag and let
CI do it:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

`.github\workflows\release.yml` runs the tests, packages the ZIP, and attaches it to the
release automatically.

To regenerate the app icon after editing its design:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\make-icon.ps1
```

---

## 📁 Project Structure

```
keeboy/
├── Cargo.toml
├── build.rs               # Embeds the app icon resource at compile time
├── .cargo/
│   └── config.toml        # Static CRT link (no VC++ Redistributable needed)
├── .github/workflows/
│   └── release.yml        # Tag push -> tested, packaged, attached to a Release
├── assets/
│   ├── keeboy.ico         # App + tray icon (16/32/48 px)
│   └── keeboy.rc          # Icon + version resource script
├── scripts/
│   ├── install.ps1        # Dev install from the repo (builds, then shortcuts)
│   ├── Install-Keeboy.ps1 # Standalone installer shipped inside the release ZIP
│   ├── package.ps1        # Builds + writes dist\Keeboy-v<ver>-win-x64.zip
│   └── make-icon.ps1      # Regenerates keeboy.ico
├── src/
│   ├── main.rs            # Entrypoint, thread orchestration & console status
│   ├── lib.rs             # Core crate library root
│   ├── hook.rs            # Windows WH_KEYBOARD_LL hook & key mapping
│   ├── tray.rs            # Native Win32 tray icon, popup menu & single-instance guard
│   └── audio/
│       ├── mod.rs         # Audio subsystem exports
│       ├── generator.rs   # Mathematical physical acoustic modeling for switches
│       ├── soundpack.rs   # Soundpack definitions, bundling, and folder loader
│       ├── session.rs     # Windows Volume Mixer session naming (Core Audio COM)
│       └── engine.rs      # In-memory dispatcher + master/release/per-profile volumes
└── tests/
    └── sound_tests.rs     # Automated test suite for acoustic buffers & state
```

---

## 📄 License
MIT
