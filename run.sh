#!/bin/bash
example=$1
export XARGO_RUST_SRC=$HOME/src/oss/avr-rust
PORT=/dev/cu.usbmodem1431
set -e

elf=target/avr-atmega32u4/release/examples/$example.elf

rustup run avr-toolchain xargo build --target avr-atmega32u4 --release --verbose --example $example
avr-size --format=avr --mcu=atmega32u4 $elf
avr-objcopy $elf -O ihex target/$example.hex

if [[ ! -e $PORT ]] ; then
  echo "$PORT is not present."
  echo "Press the reset button on the board?"
  while [[ ! -e $PORT ]] ; do
    sleep 1
  done
fi

if [[ -e $PORT ]]; then
  case $(uname) in
    Darwin)
      stty -f $PORT ispeed 1200 ospeed 1200
      ;;
    Linux)
      stty -F $PORT ispeed 1200 ospeed 1200
      ;;
  esac
fi

avrdude -p atmega32u4 -U flash:w:target/$example.hex:i -cavr109 -b57600 -D -P $PORT
