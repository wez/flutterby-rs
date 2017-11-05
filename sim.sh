#!/bin/bash
example=$1
export XARGO_RUST_SRC=$HOME/src/oss/avr-rust
set -e

rustup run avr-toolchain xargo build --target avr-atmega32u4 --release --example $example

elf=target/avr-atmega32u4/release/examples/$example.elf

echo " * You can connect to the sim using"
echo "avr-gdb $elf -ex \"target remote :1234\" -tui"
set -x

simavr -g -m atmega32u4 -f 8000000 -v -v -v -t $elf

