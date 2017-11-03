#![feature(lang_items, unwind_attributes)]
#![feature(asm)]
#![feature(optin_builtin_traits)]
#![feature(abi_avr_interrupt)]
#![no_std]

extern crate arrayvec;
extern crate arduino;

pub mod fcpu;
pub mod mutex;
pub mod eventloop;

// This lang item is present to satisfy the rust linking machinery
// that we've got an entry point.  It also provides us a way to insert
// code that runs before main.  This costs us a few bytes of instructions
// that get emitted in the bin crate, but improves ergonomics.
#[lang = "start"]
extern "C" fn __bin_crate_start(main: fn(), _argc: isize, _argv: *const *const u8) -> isize {
    main();
    0
}

#[lang = "eh_personality"]
#[no_mangle]
pub unsafe extern "C" fn rust_eh_personality(
    _state: (),
    _exception_object: *mut (),
    _context: *mut (),
) -> () {
}

#[lang = "panic_fmt"]
#[unwind]
pub extern "C" fn rust_begin_panic(_msg: (), _file: &'static str, _line: u32) -> ! {
    loop {}
}
