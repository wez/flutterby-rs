
// Suitable for adafruit 32u4 boards @ 8MHz
#[cfg(feature = "clock_8mhz")]
const F_CPU: u32 = 8_000_000;

// Suitable for ergodox @ 16MHz
#[cfg(feature = "clock_16mhz")]
const F_CPU: u32 = 16_000_000;

/// Busy wait for the specified number of ms
// borrowed from https://github.com/shepmaster/rust-arduino-blink-led-no-core/blob/part3/hello.rs#L18
pub fn busy_wait_ms(duration_ms: u16) {
    const CYCLES_PER_MS: u16 = (F_CPU / 1000) as u16;
    const CYCLES_PER_INNER_LOOP: u16 = 6; // From the disassembly
    const INNER_LOOP_ITERATIONS: u16 = CYCLES_PER_MS / CYCLES_PER_INNER_LOOP;

    let mut outer = 0;
    while outer < duration_ms {
        let mut inner = 0;
        while inner < INNER_LOOP_ITERATIONS {
            unsafe {
                asm!("");
            }
            inner += 1;
        }
        outer += 1;
    }
}
