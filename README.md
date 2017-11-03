# flutterby-rs
Keyboard firmware implemented in Rust.

*It doesn't do anything useful yet!*

## Building for atmega32u4 devices (ergodox-ez, feather32u4)

* First build the cross compiler per these instructions: https://github.com/avr-rust/rust (this will take a couple of hours, but you only need to do it once)

* Then build the examples from this repo:

```
$ XARGO_RUST_SRC=$HOME/avr-rust rustup run avr-toolchain \
     xargo build --target avr-atmega32u4 --release --verbose --examples
```

To flash it to the target device:

```
$ avr-objcopy target/avr-atmega32u4/release/examples/blink.elf -O ihex target/target.hex
$ avrdude -p atmega32u4 -U flash:w:target/target.hex:i -cavr109 -b57600 -D
```
