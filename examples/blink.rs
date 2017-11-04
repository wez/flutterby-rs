#![no_std]

extern crate flutterby;

// this is the red LED on most adafruit 32u4 boards
// (the feather product line)
use flutterby::mcu::{PortcSignalFlags, PORTC};

pub fn toggle_led() {
    unsafe {
        (*PORTC.get()).portc.modify(|x| x ^ PortcSignalFlags::PC7);
    }
}

fn main() {
    unsafe {
        (*PORTC.get()).ddrc.modify(|x| x | PortcSignalFlags::PC7);
    }

    /*
    let events = flutterby::eventloop::EventLoop::new();

    events.add_callback(toggle);

    events.run();
    */

    loop {
        toggle_led();
        flutterby::fcpu::busy_wait_ms(1000);
    }
}
