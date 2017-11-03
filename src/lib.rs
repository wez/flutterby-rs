#![feature(lang_items, unwind_attributes)]
#![feature(asm)]
#![no_std]

pub mod fcpu;

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
