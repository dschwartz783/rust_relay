#!/bin/zsh

cd /home/david/rust_scripts/rust_relay

/home/david/.cargo/bin/cargo install --root /home/david_local --path .

sudo setcap cap_net_raw+p /home/david_local/bin/rust_relay
sudo systemctl restart relay
