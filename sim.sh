#!/bin/bash
example=$1
export XARGO_RUST_SRC=$HOME/src/oss/avr-rust
set -e

rustup run avr-toolchain xargo build --target avr-atmega32u4 --example $example --features simavr

elf=target/avr-atmega32u4/debug/examples/$example.elf
avr-size --format=avr --mcu=atmega32u4 $elf

echo " * You can connect to the sim using"
echo "avr-gdb $elf -ex \"target remote :1234\" -tui"
set -x

simavr -m atmega32u4 -f 8000000 -v -v -v -v -v -v $elf

