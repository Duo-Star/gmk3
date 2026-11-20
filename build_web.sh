#!/usr/bin/env sh
set -eu

export RUSTUP_HOME=/tmp/rustup
export CARGO_HOME=/tmp/cargo
export PATH="/tmp/cargo/bin:$PATH"

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
. "/tmp/cargo/env"
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk build --release