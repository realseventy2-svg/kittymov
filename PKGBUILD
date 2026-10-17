# Maintainer: realseventy2-svg <realseventy2@gmail.com>
pkgname=cachyflix
pkgver=2.0.0
pkgrel=1
pkgdesc="Modern Cyber-Minimalist Terminal Streaming Suite for CachyOS"
arch=('x86_64')
url="https://github.com/realseventy2-svg/cachyflix"
license=('MIT')
depends=('mpv' 'chafa')
makedepends=('cargo' 'rust')
source=()

build() {
    cd "${srcdir}/.."
    cargo build --release --locked
}

package() {
    cd "${srcdir}/.."
    install -Dm755 "target/release/cachyflix" "${pkgdir}/usr/bin/cachyflix"
    
    # Desktop entry
    install -Dm644 /dev/stdin "${pkgdir}/usr/share/applications/cachyflix.desktop" << 'EOF'
[Desktop Entry]
Name=CachyFlix
Comment=Cyber-Minimalist Terminal Movie and TV Series Streamer
Exec=cachyflix
Icon=video-television
Terminal=true
Type=Application
Categories=AudioVideo;Player;Video;
Keywords=streaming;movie;tv;series;anime;mpv;ratatui;
EOF
}
