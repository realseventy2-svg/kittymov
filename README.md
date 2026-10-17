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

**KittyMov** is a native, ultra-responsive Terminal UI for discovering, cataloging, and streaming Movies and TV Shows. Designed with a retro-futuristic dark cyber-minimalist aesthetic (obsidian canvas, neon magenta cursor, cyber-cyan borders), high-speed concurrency, true 24-bit ANSI poster art, multi-provider stream fallback, dedicated OpenSubtitles integration, and an automated TV episode marathon engine.

Works seamlessly across all Linux distributions (CachyOS, Arch, Debian, Ubuntu, Fedora, Void, NixOS, etc.) and modern terminal emulators (Kitty, Alacritty, Ghostty, Foot, WezTerm, tmux).

---

## ✨ Features

- ⚡ **Native Rust Performance**: Sub-millisecond startup times, compiled release binary under `8 MB`, low RAM usage.
- 🎨 **Cinematic Cyber-Minimalist Aesthetic**: Obsidian backdrop (`#101113`), warm amber accents (`#E8B875`), and high-density metadata tables.
- 🖼️ **True 24-Bit Half-Block Posters**: Custom ANSI SGR tokenizer extracting both foreground and background colors (`38;2` and `48;2`) via `chafa` with instant toggle support.
- 🌐 **Multi-Provider Stream Resilience**: Automatic failover across LookMovie, VixCloud, and VixSrc with interactive provider/server selection modal (`s`).
- 💬 **External Subtitles & Dual-Audio Engine**: Direct OpenSubtitles API integration for automatic sub downloads, multi-language subtitle selector (`t`), and Sub/Dub audio toggle (`d`).
- 📺 **TV Episode Marathon Engine**: Seamless auto-play next episode with background pre-fetching during the final 20% of playback and dynamic injection via MPV IPC.
- 🔄 **Anti-Expiration JIT Link Protection**: Resolves and refreshes HLS master playlist tokens just-in-time before playback to eliminate link expiration errors.
- 💾 **Standard XDG Compliance**: Strictly follows XDG Base Directory specification (`$XDG_CONFIG_HOME`, `$XDG_CACHE_HOME`, `$XDG_STATE_HOME`).
- 🩺 **Built-in System Doctor & Privacy Reports**: Quick environment diagnostic tool (`--doctor`) and redacted diagnostic exports.
- 🔒 **Security Hardened**: URL scheme filtering (HTTP/HTTPS enforcement), path traversal protection on cache assets, and auto-backup recovery for corrupted configs.

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

*System Dependencies:*
- `mpv` (recommended media player with IPC socket support)
- `chafa` (poster graphics conversion; optional, can be disabled in settings)

---

## 🎮 Usage

```bash
# Launch interactive TUI
kittymov

# Run environment health check and dependency diagnosis
kittymov --doctor

# Direct search query on launch
kittymov "Arcane"
# or
kittymov --search "Dune"

# Instantly resume your most recent watch
kittymov --resume
```

---

## ⌨️ Keybindings

Press **`?`** or **`F1`** anywhere inside KittyMov to open the contextual Help Modal.

### Global & Main Catalog

| Key | Action |
| :--- | :--- |
| `?` / `F1` | Open **Help & Keybindings** modal |
| `S` / `,` / `F2` | Open **Settings** navigation menu |
| `1` | **Movies** tab (Trending movies) |
| `2` | **Series** tab (Trending TV shows) |
| `3` | **History** tab (Resume watching) |
| `4` | **Bookmarks** tab (Saved titles) |
| `/` | **Search** catalog (`Enter` to submit, `Esc` to cancel) |
| `↑` / `k` | Move cursor up |
| `↓` / `j` | Move cursor down |
| `Enter` | Play movie or open **Episode Guide** |
| `s` | Open **Provider & Server Selector** modal |
| `t` | Open **Subtitle Language Selector** modal |
| `d` | Toggle Audio Mode (**Sub** ↔ **Dub**) |
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
| `s` | Open Provider/Server selector for current episode |
| `t` | Change subtitle language |
| `d` | Toggle Sub / Dub mode |
| `a` | Toggle Auto-Play mode |
| `?` / `F1` | Show Help modal |
| `Esc` | Close modal |

### Settings Menu

| Key | Action |
| :--- | :--- |
| `Tab` / `Shift+Tab` | Switch between Category Sidebar and Content Pane |
| `↑` / `k` | Navigate items |
| `↓` / `j` | Navigate items |
| `←` / `→` / `Space` | Adjust numeric values, cycle options, or toggle booleans |
| `/` | Live fuzzy filter settings across all categories |
| `r` | Reset current category to factory defaults |
| `Esc` | Return to category sidebar or close settings |

---

## ⚙️ Configuration & XDG Storage

KittyMov strictly adheres to standard XDG directories:

- **Configuration:** `$XDG_CONFIG_HOME/kittymov/config.json` (defaults to `~/.config/kittymov/config.json`)
- **Cache:** `$XDG_CACHE_HOME/kittymov/posters/` (defaults to `~/.cache/kittymov/posters/`)
- **Database & Logs:** `$XDG_STATE_HOME/kittymov/kittymov.db` and `kittymov.log` (defaults to `~/.local/state/kittymov/`)

Sample configuration:

```json
{
  "player": "mpv",
  "auto_play_next": true,
  "audio_language": "eng",
  "sub_language": "eng",
  "hwdec": "auto",
  "preferred_provider": "Auto",
  "poster_render_mode": "halfblock",
  "stream_resolution": "1080p",
  "ui_density": "Comfortable",
  "tmdb_api_key": "653bb8af90162bd98fc7ee32bcbbfb3d",
  "mpv_extra_args": [
    "--save-position-on-quit",
    "--force-window=immediate",
    "--autofit=85%x85%"
  ]
}
```

> [!NOTE]
> Environment variable `TMDB_API_KEY` can be used to override the configured TMDB API key without modifying the configuration file.

---

## 📜 License

Distributed under the [MIT License](LICENSE).
