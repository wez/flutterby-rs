//! This module is helpful when running your firmware under the simavr
//! simulator.  It provides an implementation of a function that can
//! log to the simavr screen output.  The way this is implemented is
//! that build.rs selects a couple of unused addresses in the IO space
//! of the target system and maps the to the CONSOLE and COMMAND register
//! address functions in simavr.  These addresses are published in an
//! ELF section named `.mmcu` that the simavr loader will search
//! at execution time.   The target .json file needs to be amended
//! to reference one of those symbols to avoid having it be discarded
//! at the final linker stage.
use mcu;
use core::fmt::{Error, Write};
use core::mem;

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum Tag {
    TAG = 0,
    TAG_NAME,
    TAG_FREQUENCY,
    TAG_VCC,
    TAG_AVCC,
    TAG_AREF,
    TAG_LFUSE,
    TAG_HFUSE,
    TAG_EFUSE,
    TAG_SIGNATURE,
    TAG_SIMAVR_COMMAND,
    TAG_SIMAVR_CONSOLE,
    TAG_VCD_FILENAME,
    TAG_VCD_PERIOD,
    TAG_VCD_TRACE,
    TAG_VCD_PORTPIN,
    TAG_VCD_IRQ,
    TAG_PORT_EXTERNAL_PULL,
}

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum Cmd {
    SIMAVR_CMD_NONE = 0,
    SIMAVR_CMD_VCD_START_TRACE,
    SIMAVR_CMD_VCD_STOP_TRACE,
    SIMAVR_CMD_UART_LOOPBACK,
}

#[repr(C, packed)]
pub struct McuAddr {
    pub tag: Tag,
    pub len: u8,
    pub what: *const u8,
}
unsafe impl Sync for McuAddr {}

/// Console is a helper struct that is used to participate
/// in writing output to the console via `format!()`.
/// At the time of writing, the libcore implementation of
/// `core::fmt` is unavailable, so this does nothing.
pub struct Console {}
impl Write for Console {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        log_string(s);
        Ok(())
    }
}


pub fn log_data(data: &[u8]) {
    unsafe {
        let console = &(*mcu::simavr_regs::SIMAVR_CONSOLE.get());

        for c in data.iter() {
            console.write(*c);
            asm!("NOP"::::"volatile");
        }
    }
}

unsafe fn log_cstr(s: *const u8) {
    if s.is_null() {
        return;
    }
    let console = &(*mcu::simavr_regs::SIMAVR_CONSOLE.get());
    let mut s = s;
    loop {
        let c = *s;
        if c == 0 {
            return;
        }
        console.write(c);
        asm!("NOP"::::"volatile");

        s = ((s as usize) + 1) as *const u8;
    }
}

pub fn log_string(s: &str) {
    log_data(s.as_bytes())
}

pub trait ConsoleWrite {
    fn write_to_console(&self);
}

extern "C" {
    fn ultoa(val: u32, s: *mut u8, radix: i16) -> *mut u8;
    fn ltoa(val: i32, s: *mut u8, radix: i16) -> *mut u8;
}

fn write_number_i32(num: i32) {
    unsafe {
        let mut buf: [u8; 16] = mem::zeroed();
        log_cstr(ltoa(num, buf.as_mut_ptr(), 10));
    }
}

fn write_number_u32(num: u32) {
    unsafe {
        let mut buf: [u8; 16] = mem::zeroed();
        log_cstr(ultoa(num, buf.as_mut_ptr(), 10));
    }
}

impl ConsoleWrite for usize {
    fn write_to_console(&self) {
        write_number_u32(*self as u32);
    }
}

impl ConsoleWrite for u32 {
    fn write_to_console(&self) {
        write_number_u32(*self);
    }
}

impl ConsoleWrite for u16 {
    fn write_to_console(&self) {
        write_number_u32(*self as u32);
    }
}

impl ConsoleWrite for u8 {
    fn write_to_console(&self) {
        write_number_u32(*self as u32);
    }
}

impl ConsoleWrite for isize {
    fn write_to_console(&self) {
        write_number_i32(*self as i32);
    }
}

impl ConsoleWrite for i32 {
    fn write_to_console(&self) {
        write_number_i32(*self);
    }
}

impl ConsoleWrite for i16 {
    fn write_to_console(&self) {
        write_number_i32(*self as i32);
    }
}

impl ConsoleWrite for i8 {
    fn write_to_console(&self) {
        write_number_i32(*self as i32);
    }
}

impl ConsoleWrite for str {
    fn write_to_console(&self) {
        log_string(self);
    }
}


/// Log a line to the simavr console using core::fmt
/// core::fmt is nerfed in libcore at the moment, so
/// this does nothing useful right now.
/// https://github.com/avr-rust/libcore/issues/3
#[macro_export]
macro_rules! simavr_logln_fmt {
    ($($args:tt)*) => ({
        use $crate::simavr::Console;
        use core::fmt::Write;

        let file = file!();
        let line = line!();

        let mut writer = Console{};

        writer.write_fmt(format_args!("{}:{} ", file, line)).ok();
        writer.write_fmt(format_args!($($args)*)).ok();
        writer.write_str("\r").ok();
    })
}

/// Log a series of expressions to the simavr console.
/// This is a simplified debugging aide; it knows how
/// to render strings and some integral types.
#[macro_export]
macro_rules! simavr_logln {
    ($($arg:expr),*) => ({
        use $crate::simavr::ConsoleWrite;
        let file = file!();
        let line = line!();

        file.write_to_console();
        ":".write_to_console();
        line.write_to_console();
        " ".write_to_console();

        $(
            $arg.write_to_console();
        )*

        "\r".write_to_console();
    })
}
