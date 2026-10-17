# 🐱 KittyMov

<div align="center">

```
 ██╗  ██╗██╗████████╗████████╗██╗   ██╗███╗   ███╗ ██████╗  ██╗   ██╗
 ██║ ██╔╝██║╚══██╔══╝╚══██╔══╝╚██╗ ██╔╝████╗ ████║██╔═══██╗██║   ██║
 █████╔╝ ██║   ██║      ██║    ╚████╔╝ ██╔████╔██║██║   ██║██║   ██║
 ██╔═██╗ ██║   ██║      ██║     ╚██╔╝  ██║╚██╔╝██║██║   ██║╚██╗ ██╔╝
 ██║  ██╗██║   ██║      ██║      ██║   ██║ ╚═╝ ██║╚██████╔╝ ╚████╔╝ 
 ╚═╝  ╚═╝╚═╝   ╚═╝      ╚═╝      ╚═╝   ╚═╝     ╚═╝ ╚═════╝   ╚═══╝  
```

### A keyboard-driven terminal movie & TV streaming app for Linux

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux-lightgrey?logo=linux)](https://kernel.org)
[![Built with Ratatui](https://img.shields.io/badge/Built%20with-Ratatui-cyan)](https://ratatui.rs)

</div>

---

Hey there! **KittyMov** is a terminal streaming application written in Rust.

I made this firstly for my personal use until I decided to share it with other people who have the same issue or problems I encountered with alot of dead providers in some applications like these to watch movies, tv-series and anime without any playback issues.

It runs in any Linux terminal (Kitty, Alacritty, Ghostty, Foot, WezTerm, tmux, etc.) and gives you a comfortable split-pane browser with cover art, ratings, episode guides, and multi-source streaming failover.

---

## What It Does

- **Instant search & metadata**: Pulls titles, descriptions, ratings, genres, and cast details directly from TMDB without needing a browser.
- **High-resolution terminal cover art**: Automatically downloads posters and converts them using `chafa` with Unicode 13 sextants and quadrants for crisp 24-bit color artwork. If your terminal doesn't support fancy symbols, it falls back to classic half-blocks or clean text-only mode.
- **Multiple streaming providers**: Resolves video streams from multiple sources (such as VixSrc and LookMovie). If one server is slow or down, it automatically tries the next one, or lets you manually select your preferred server and quality with `s`.
- **Subtitles that just work**: Automatically checks the stream provider for built-in subtitle tracks. If none are provided, it queries OpenSubtitles in the background. You can also pick specific languages (`t`) or adjust timing offsets in real time.
- **Broadcast vs. Streaming episode orders**: Animated shows and cartoons (like *Steven Universe*) frequently have different air dates compared to production or streaming orders. You can toggle between Broadcast and Streaming ordering with a single key (`o`).
- **Hands-off episode marathons**: When you're watching a TV series, it keeps track of your position and automatically queues up the next episode.
- **Local history & bookmarks**: Everything stays on your machine. Your watch progress, bookmark list, and preferences are saved locally in SQLite so you can instantly resume where you left off.
- **Beginner-friendly setup wizard**: If you launch it on a fresh system, it checks your environment, verifies dependencies, and helps you set your audio and subtitle preferences right away.

---

## Requirements

Before installing, make sure you have:

1. **`mpv`** (Required) — The video player that handles the actual playback.
2. **`chafa`** (Optional, but recommended) — Used for drawing poster artwork in your terminal. If you don't have it installed, KittyMov will simply run in text-only mode without crashing.

On Arch Linux / CachyOS:
```bash
sudo pacman -S mpv chafa
```

On Debian / Ubuntu:
```bash
sudo apt install mpv chafa
```

On Fedora:
```bash
sudo dnf install mpv chafa
```

---

## Installation

### Method 1: Arch Linux / CachyOS (PKGBUILD)

```bash
git clone https://github.com/realseventy2-svg/kittymov.git
cd kittymov
makepkg -si
```

### Method 2: Building from source with Cargo

```bash
git clone https://github.com/realseventy2-svg/kittymov.git
cd kittymov
cargo build --release
install -Dm755 target/release/kittymov ~/.local/bin/kittymov
```

Make sure `~/.local/bin` is in your `$PATH`.

---

## How to Use It

Launch the main interface:
```bash
kittymov
```

Jump straight into a search:
```bash
kittymov "Spirited Away"
# or
kittymov --search "Arcane"
```

Resume whatever you were watching last:
```bash
kittymov --resume
```

Run a system diagnosis to make sure your tools and paths are configured properly:
```bash
kittymov --doctor
```

Run the initial setup wizard again anytime:
```bash
kittymov --setup
```

---

## Keyboard Shortcuts

You can press **`?`** or **`F1`** anywhere in the app to view the shortcut cheatsheet.

### Main Catalog & Navigation

| Key | What it does |
| :--- | :--- |
| `↑` / `k`, `↓` / `j` | Move selection up or down |
| `1` | Show Trending Movies |
| `2` | Show Trending TV Series |
| `3` | Show Watch History |
| `4` | Show Bookmarks |
| `/` | Search titles (`Enter` to search, `Esc` to cancel) |
| `Enter` | Play movie or open TV episode guide |
| `s` | Open provider and quality selector |
| `t` | Open subtitle language selector |
| `d` | Switch audio mode between Sub and Dub |
| `o` | Toggle TV episode ordering (Broadcast ↔ Streaming) |
| `f` | Bookmark or unbookmark selected title |
| `a` | Toggle auto-play next episode on or off |
| `S` / `,` / `F2` | Open Settings menu |
| `q` | Quit KittyMov |

### Episode Guide Modal

| Key | What it does |
| :--- | :--- |
| `←` / `h` / `[` | Previous season |
| `→` / `l` / `]` | Next season |
| `↑` / `k`, `↓` / `j` | Navigate episodes |
| `Enter` | Start playing selected episode |
| `s` | Pick provider and stream source for this episode |
| `t` | Choose subtitle track |
| `d` | Toggle Sub / Dub audio mode |
| `o` | Toggle Broadcast vs. Streaming episode list |
| `Esc` | Close modal and go back |

### Settings Menu

| Key | What it does |
| :--- | :--- |
| `Tab` | Switch between category list and settings items |
| `↑` / `k`, `↓` / `j` | Move through settings |
| `←` / `→` / `Space` | Change value, cycle options, or toggle settings |
| `/` | Filter settings by typing a keyword |
| `r` | Reset the current category to factory defaults |
| `Esc` | Back to category list or close settings |

---

## Questions & Answers (Q&A)

### Q: Why didn't subtitles show up when I started playing a video?
**A:** Not every streaming source embeds subtitles directly inside the video stream. KittyMov handles this by querying OpenSubtitles as a fallback whenever the primary stream lacks a track for your language. You can also press **`t`** on any title or episode before starting playback to pick from available provider tracks or OpenSubtitles releases.

### Q: The subtitles are a fraction of a second off. How do I fix the timing?
**A:** You have two easy ways:
1. Inside KittyMov, press **`S`** to open Settings, go to **Playback**, and adjust **Subtitle Delay Offset** (`sub_delay`).
2. Inside `mpv` during playback, press **`z`** to decrease subtitle delay by 100ms or **`x`** to increase it by 100ms.

### Q: Why are episodes out of order for animated series (like Steven Universe)?
**A:** Cartoons often air on television in a completely different sequence than their intended production order (for example, Steven Universe Season 3 airing *Watermelon Island* first on TV while streaming services list *Steven's Birthday*). Press **`o`** in the series view or episode modal to switch between **Broadcast Order** and **Alternate / Streaming Order**. KittyMov will remember your preference.

### Q: What if a stream fails to load or buffers endlessly?
**A:** Sometimes a particular host is down or throttling requests. Press **`s`** before starting playback to open the **Provider & Quality** menu. From there, you can switch from `Auto` to a specific provider (like LookMovie or VixSrc) or select a specific stream resolution.

### Q: The poster artwork looks pixelated or doesn't show up. Why?
**A:** Make sure you have `chafa` installed on your system. KittyMov defaults to high-resolution Unicode 13 sextant and quadrant graphics. If your terminal font doesn't have these glyphs or you prefer a simpler look, press **`S`** -> **Appearance** -> **Poster Art Render Mode** and change it to **Half-Block** or **Off**.

### Q: Where does KittyMov store my settings and history?
**A:** It follows standard Linux XDG directories so nothing clutters your home folder:
- Configuration: `~/.config/kittymov/config.json`
- Watch history & bookmarks: `~/.config/kittymov/kittymov.db`
- Cached posters: `~/.cache/kittymov/posters/`
- Logs & diagnostics: `~/.local/state/kittymov/`

### Q: Can I pass custom flags to mpv (like hardware acceleration or custom shaders)?
**A:** Yes! Open `~/.config/kittymov/config.json` and look for the `mpv_extra_args` array. You can add any standard mpv arguments you want, such as:
```json
"mpv_extra_args": [
  "--hwdec=auto",
  "--save-position-on-quit",
  "--force-window=immediate",
  "--autofit=85%x85%"
]
```

### Q: How do I clear the cached poster files to free up disk space?
**A:** Press **`S`** to open Settings, navigate to **Storage**, and press **Enter** on **Poster Artwork Cache**. You can also run `rm -rf ~/.cache/kittymov/posters/*` from your terminal anytime.

---

## Configuration Reference

Here is a look at what `~/.config/kittymov/config.json` looks like with default options:

```json
{
  "player": "mpv",
  "auto_play_next": true,
  "audio_language": "eng",
  "sub_language": "eng",
  "audio_mode": "dub",
  "hwdec": "auto",
  "preferred_provider": "auto",
  "default_tab": "movies",
  "resume_on_startup": false,
  "subtitle_augmentation": true,
  "subtitle_source": "auto",
  "sub_delay": 0.0,
  "episode_order": "broadcast",
  "theme_name": "cyber",
  "poster_render_mode": "high-res",
  "tmdb_api_key": "653bb8af90162bd98fc7ee32bcbbfb3d",
  "mpv_extra_args": [
    "--save-position-on-quit",
    "--force-window=immediate",
    "--autofit=85%x85%"
  ]
}
```

> [!NOTE]
> If you have your own TMDB API key, you can either put it in `config.json` or export the `TMDB_API_KEY` environment variable in your shell profile (`~/.bashrc` or `~/.zshrc`).

---

## License

Distributed under the [MIT License](LICENSE).
