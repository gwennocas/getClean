#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
cargo build --release
install -Dm755 target/release/getclean "$HOME/.local/bin/getclean"
install -Dm644 scripts/getclean.service "$HOME/.config/systemd/user/getclean.service"
systemctl --user daemon-reload
systemctl --user enable --now getclean.service
echo 'getClean installed as a systemd user service.'
