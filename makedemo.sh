#!/usr/bin/env bash

set -e

rm -f demo/*

font="../material-design-icons/font/MaterialIconsOutlined-Regular.otf"
cargo build --release
awk '{ print "--codepoint 0x"$2" --animation pulse-whole --out-file demo/"$1"-pulse.json" } ' samples.txt | xargs -L1 echo target/release/iconimation --font $font
awk '{ print "--codepoint 0x"$2" --animation pulse-parts --out-file demo/"$1"-pulse-parts.json" } ' samples.txt | xargs -L1 echo target/release/iconimation --font $font

python3 makedemo.py

echo "Try demo/demo.html"