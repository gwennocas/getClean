#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
if [ -f ./getclean ]; then
    binary=./getclean
else
    cargo build --release
    binary=target/release/getclean
fi
install -Dm755 "$binary" "$HOME/.local/bin/getclean"
install -Dm644 scripts/getclean.service "$HOME/.config/systemd/user/getclean.service"
systemctl --user daemon-reload
systemctl --user enable --now getclean.service
echo 'getClean installed as a systemd user service.'
