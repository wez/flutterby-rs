#![no_std]

extern crate flutterby;

// this is the red LED on most adafruit 32u4 boards
// (the feather product line)
use flutterby::mcu::{PortcSignalFlags, PORTC};

pub fn toggle_led(_ticks: flutterby::eventloop::Ticks) {
    unsafe {
        (*PORTC.get()).portc.modify(|x| x ^ PortcSignalFlags::PC7);
    }
}

fn main() {
    unsafe {
        // Configure LED and turn it off
        (*PORTC.get()).ddrc.modify(|x| x | PortcSignalFlags::PC7);
        (*PORTC.get()).portc.write(PortcSignalFlags::empty());
    }

    let events = flutterby::eventloop::EventLoop::new();

    events.add_callback(toggle_led).expect("add led callback");

    events.run();
}
