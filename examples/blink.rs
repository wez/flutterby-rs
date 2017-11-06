#![feature(naked_functions)]
#![no_std]
#![no_main]

#[macro_use]
extern crate flutterby;

use flutterby::eventloop::Ticks;

// this is the red LED on most adafruit 32u4 boards
// (the feather product line)
use flutterby::mcu::{PortcSignalFlags, PORTC};

pub fn toggle_led(_now: flutterby::eventloop::Ticks) {
    unsafe {
        (*PORTC.get()).portc.modify(|mut x| {
            x ^= PortcSignalFlags::PC7;
            logln!("LED ", x.bits());
            x
        });
    }
}

#[no_mangle]
pub extern "C" fn main() {
    flutterby::reset_peripherals();

    unsafe {
        // Configure LED and turn it off
        (*PORTC.get()).ddrc.modify(|x| x | PortcSignalFlags::PC7);
        (*PORTC.get()).portc.write(PortcSignalFlags::empty());
    }

    let events = flutterby::eventloop::EventLoop::new();

    events
        .spawn_repeating(toggle_led, Ticks::milliseconds(1000))
        .expect("add led callback");

    events.run();
}
