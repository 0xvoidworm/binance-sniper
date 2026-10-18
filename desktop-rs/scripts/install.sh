#!/usr/bin/env bash
set -euo pipefail

desk_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
desk_binary="$desk_root/target/release/binance-desk"
desk_bin_dir="$HOME/.local/bin"
desk_share_dir="${XDG_DATA_HOME:-$HOME/.local/share}"

if [[ ! -x "$desk_binary" ]]; then
  printf 'Build dulu: cargo build --locked --release\n' >&2
  exit 1
fi

mkdir -p "$desk_bin_dir" "$desk_share_dir/applications" "$desk_share_dir/icons/hicolor/scalable/apps"
install -m 755 "$desk_binary" "$desk_bin_dir/.binance-desk.new"
mv "$desk_bin_dir/.binance-desk.new" "$desk_bin_dir/binance-desk"
install -m 644 "$desk_root/assets/io.yuri.BinanceDesk.svg" "$desk_share_dir/icons/hicolor/scalable/apps/io.yuri.BinanceDesk.svg"

cat > "$desk_share_dir/applications/io.yuri.BinanceDesk.desktop" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Binance Desk
GenericName=Trading workspace
Comment=Mesin Futures Binance dengan strategi, batas risiko dan pengelolaan order otomatis
Exec="$desk_bin_dir/binance-desk"
Icon=io.yuri.BinanceDesk
Terminal=false
Categories=Office;Finance;
StartupNotify=true
StartupWMClass=io.yuri.BinanceDesk
Keywords=trading;binance;futures;rust;
EOF

desktop-file-validate "$desk_share_dir/applications/io.yuri.BinanceDesk.desktop"
update-desktop-database "$desk_share_dir/applications"
printf 'Terpasang: %s\nMenu aplikasi: Binance Desk\n' "$desk_bin_dir/binance-desk"
