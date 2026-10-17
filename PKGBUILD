# Maintainer: realseventy2-svg <realseventy2@gmail.com>
pkgname=kittymov
pkgver=2.0.0
pkgrel=1
pkgdesc="Modern Cyber-Minimalist Terminal Movie & TV Streaming Suite"
arch=('x86_64')
url="https://github.com/realseventy2-svg/kittymov"
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
    install -Dm755 "target/release/kittymov" "${pkgdir}/usr/bin/kittymov"
    
    # Desktop entry
    install -Dm644 /dev/stdin "${pkgdir}/usr/share/applications/kittymov.desktop" << 'EOF'
[Desktop Entry]
Name=KittyMov
Comment=Cyber-Minimalist Terminal Movie and TV Series Streamer
Exec=kittymov
Icon=video-television
Terminal=true
Type=Application
Categories=AudioVideo;Player;Video;
Keywords=streaming;movie;tv;series;anime;mpv;ratatui;kittymov;
EOF
}
