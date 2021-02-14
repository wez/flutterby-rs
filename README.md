# flutterby-rs
Keyboard firmware implemented in Rust.

*It doesn't do anything useful yet!*

## Building for atmega32u4 devices (ergodox-ez, feather32u4)

* First build the cross compiler (you only need to do it once) per these instructions: https://book.avr-rust.com/
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

There's a helper script for building and running a given example:

```
$ ./run.sh blink
```

This makes some assumptions about the location of the `avr-rust` source and which port
my device is attached to.

## Debugging Using simavr

You need to install simavr and avr-gdb for yourself.  On the mac you can do this as a one-time setup:

```
$ brew install avr-gdb
$ brew install --HEAD simavr
```

(this may take a while as it may need to build gcc)

Then you can use the `sim.sh` script to run an example under the simulator.  There's no GUI
or other visual output for this, but it does start paused and waiting for you to attach
with gdb.

In one window:

```
$ ./sim.sh blink
    Finished release [optimized + debuginfo] target(s) in 0.0 secs
 * You can connect to the sim using
avr-gdb target/avr-atmega32u4/release/examples/blink.elf -ex "target remote :1234" -tui
+ simavr -g -m atmega32u4 -f 8000000 -v -v -v -t target/avr-atmega32u4/release/examples/blink.elf
Loaded 2486 .text at address 0x0
Loaded 110 .data
avr_gdb_init listening on port 1234
```

Then in another window run the gdb command that it printed above:

```
$ avr-gdb target/avr-atmega32u4/release/examples/blink.elf -ex "target remote :1234" -tui
```
