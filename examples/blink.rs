#![no_std]
#![no_main]
#![feature(asm)]

extern crate avrd;
extern crate flutterby;

use avrd::atmega32u4::*;
// this is the red LED on most adafruit 32u4 boards
// (the feather product line)
const PC7: u8 = 1 << 7;

use core::ptr::{read_volatile, write_volatile};

#[no_mangle]
pub extern "C" fn main() {
    unsafe { write_volatile(DDRC, read_volatile(DDRC) | PC7) }

    loop {
        unsafe { write_volatile(PORTC, read_volatile(PORTC) | PC7) }
        flutterby::fcpu::busy_wait_ms(1000);

        unsafe { write_volatile(PORTC, read_volatile(PORTC) & !PC7) }
        flutterby::fcpu::busy_wait_ms(1000);
    }
}
