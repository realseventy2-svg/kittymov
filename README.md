# 🐱 KittyMov

<div align="center">

```
 ██╗  ██╗██╗████████╗████████╗██╗   ██╗███╗   ███╗ ██████╗ ██╗   ██╗
 ██║ ██╔╝██║╚══██╔══╝╚══██╔══╝╚██╗ ██╔╝████╗ ████║██╔═══██╗██║   ██║
 █████╔╝ ██║   ██║      ██║    ╚████╔╝ ██╔████╔██║██║   ██║██║   ██║
 ██╔═██╗ ██║   ██║      ██║     ╚██╔╝  ██║╚██╔╝██║██║   ██║╚██╗ ██╔╝
 ██║  ██╗██║   ██║      ██║      ██║   ██║ ╚═╝ ██║╚██████╔╝ ╚████╔╝ 
 ╚═╝  ╚═╝╚═╝   ╚═╝      ╚═╝      ╚═╝   ╚═╝     ╚═╝ ╚═════╝   ╚═══╝  
```

### Modern Cyber-Minimalist Terminal Movie & TV Series Streaming Suite

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux-lightgrey?logo=linux)](https://kernel.org)
[![Built with Ratatui](https://img.shields.io/badge/Built%20with-Ratatui-cyan)](https://ratatui.rs)

</div>

---

**KittyMov** is a native, ultra-responsive Terminal UI for discovering, cataloging, and streaming Movies and TV Shows. Designed with a retro-futuristic dark cyber-minimalist aesthetic (obsidian canvas, neon magenta cursor, cyber-cyan double borders), high-speed concurrency, true 24-bit ANSI poster art, and an automated TV episode marathon engine.

Works seamlessly across all Linux distributions (CachyOS, Arch, Debian, Ubuntu, Fedora, Void, NixOS, etc.) and modern terminal emulators (Kitty, Alacritty, Ghostty, Foot, WezTerm, tmux).

---

## ✨ Features

- ⚡ **Native Rust Performance**: Sub-millisecond startup times, compiled release binary under `8 MB`, low RAM usage.
- 🎨 **BENEXL Cyber-Minimalist Aesthetic**: Obsidian backdrop (`#0b0e14`), glowing cyber-cyan borders (`#00e5ff`), and high-density metadata tables.
- 🖼️ **True 24-Bit Half-Block Posters**: Custom ANSI SGR tokenizer extracting both foreground and background colors (`38;2` and `48;2`) via `chafa` without terminal escape leakage.
- 📺 **TV Episode Marathon Engine**: Seamless auto-play next episode with background pre-fetching during the final 20% of playback and dynamic injection via MPV IPC.
- 🔄 **Anti-Expiration JIT Link Protection**: Resolves and refreshes HLS master playlist tokens just-in-time before playback to eliminate link expiration errors.
- 💾 **Local SQLite Persistence**: Tracks watch history, resume timestamps, and favorites in `~/.config/kittymov/kittymov.db`.
- 🔍 **Interactive Live Search & Episode Guide**: Season-by-season episode browsing with runtime, air date, and synopses.

---

## 🚀 Quick Start

### Installation

#### Arch Linux / CachyOS (PKGBUILD)
```bash
git clone https://github.com/realseventy2-svg/kittymov.git
cd kittymov
makepkg -si
```

#### Cargo (Any Distribution)
```bash
git clone https://github.com/realseventy2-svg/kittymov.git
cd kittymov
cargo build --release
install -Dm755 target/release/kittymov ~/.local/bin/kittymov
```

*Ensure dependencies are installed:*
- `mpv` (video player)
- `chafa` (poster graphics conversion)

---

## 🎮 Usage

```bash
# Launch interactive TUI
kittymov

# Direct search query on launch
kittymov "Arcane"
# or
kittymov --search "Dune"

# Instantly resume your most recent watch
kittymov --resume
```

---

## ⌨️ Keybindings

### Main Catalog

| Key | Action |
| :--- | :--- |
| `1` | **Movies** tab (Trending movies) |
| `2` | **Series** tab (Trending TV shows) |
| `3` | **History** tab (Resume watching) |
| `4` | **Bookmarks** tab (Saved titles) |
| `/` | **Search** catalog (`Enter` to submit, `Esc` to cancel) |
| `↑` / `k` | Move cursor up |
| `↓` / `j` | Move cursor down |
| `Enter` | Play movie or open **Episode Guide** |
| `f` | Bookmark / Unbookmark title |
| `a` | Toggle **Auto-Play Next Episode** (`ON` / `OFF`) |
| `q` | Quit KittyMov |

### Episode Guide Modal

| Key | Action |
| :--- | :--- |
| `←` / `h` / `[` | Previous season |
| `→` / `l` / `]` | Next season |
| `↑` / `k` | Previous episode |
| `↓` / `j` | Next episode |
| `Enter` | Launch episode in player |
| `a` | Toggle Auto-Play mode |
| `Esc` | Close modal |

---

## ⚙️ Configuration

Configuration is located at `~/.config/kittymov/config.json`:

```json
{
  "player": "mpv",
  "auto_play_next": true,
  "audio_language": "eng",
  "sub_language": "eng",
  "hwdec": "auto",
  "tmdb_api_key": "653bb8af90162bd98fc7ee32bcbbfb3d",
  "mpv_extra_args": [
    "--save-position-on-quit",
    "--force-window=immediate",
    "--autofit=85%x85%"
  ]
}
```

---

## 📜 License

Distributed under the [MIT License](LICENSE).
