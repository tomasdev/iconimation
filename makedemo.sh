#!/usr/bin/env bash

set -e

function generate() {
    local sample_file="$1"
    local animation="$2"
    local font="$3"
    # strip comments
    sed 's/#.*//g' "$sample_file" \
    | grep -v "^\S*$" \
    | awk "{ print \"--codepoint 0x\"\$2\" --animation $animation --out-file demo/\"\$1\"-$animation.json\" } " \
    | xargs -L1 target/release/iconimation --font "$font" --debug
}

rm -f demo/*

font='../material-design-icons/variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf'
sample_file=samples2.txt
cargo build --release

#generate samples2.txt pulse-whole "$font"
generate samples2.txt pulse-parts "$font"

python3 makedemo.py

echo "Try demo/demo.html"