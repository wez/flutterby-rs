use mcu;
use mutex;

pub enum Duration {
    Approx15ms = 15,
    Approx30ms = 30,
    Approx60ms = 60,
    Approx120ms = 120,
    Approx250ms = 250,
    Approx500ms = 500,
    Approx1s = 1000,
    Approx2s = 2000,
    Approx4s = 4000,
    Approx8s = 8000,
}

/// Called by the main startup code, so you won't generally need to call this.
/// This function re-initializes the watchdog timer and disables it.
pub fn initialize_disabled() {
    unsafe {
        let cpu = &(*mcu::CPU.get());
        cpu.mcusr.modify(|x| x - mcu::CpuMcusrFlags::WDRF);
        cpu.clkpr
            .write(mcu::CpuClkprFlags::CPU_CLK_PRESCALE_4_BITS_SMALL_1);

        // Disable watchdog resets
        asm!("WDR"::::"volatile");
        let wdt = &(*mcu::WDT.get());
        wdt.wdtcsr
            .modify(|x| x | mcu::WdtWdtcsrFlags::WDCE | mcu::WdtWdtcsrFlags::WDE);
        wdt.wdtcsr.write(mcu::WdtWdtcsrFlags::empty());
    }
}

/// Disable the watchdog timer
pub fn disable() {
    mutex::interrupt_free(|_cs| unsafe {
        asm!("WDR"::::"volatile");
        let wdt = &(*mcu::WDT.get());
        wdt.wdtcsr
            .modify(|x| x | mcu::WdtWdtcsrFlags::WDCE | mcu::WdtWdtcsrFlags::WDE);
        wdt.wdtcsr.write(mcu::WdtWdtcsrFlags::empty());
    });
}

/// Reset the watchdog counter.  If the watchdog is enabled and the counter is
/// not reset before the watchdog duration expires, an interrupt will be generated.
pub fn reset() {
    unsafe {
        asm!("WDR"::::"volatile");
    }
}

/// Enable the watchdog and set the timer interval
pub fn enable(duration: Duration) {
    use self::Duration::*;
    let mask = match duration {
        Approx15ms => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_2K,
        Approx30ms => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_4K,
        Approx60ms => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_8K,
        Approx120ms => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_16K,
        Approx250ms => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_32K,
        Approx500ms => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_64K,
        Approx1s => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_128K,
        Approx2s => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_256K,
        Approx4s => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_512K,
        Approx8s => mcu::WdtWdtcsrFlags::WDOG_TIMER_PRESCALE_4BITS_OSCILLATOR_CYCLES_1024K,
    };

    mutex::interrupt_free(|_cs| unsafe {
        asm!("WDR"::::"volatile");
        let wdt = &(*mcu::WDT.get());
        wdt.wdtcsr
            .modify(|x| x | mcu::WdtWdtcsrFlags::WDCE | mcu::WdtWdtcsrFlags::WDE);
        wdt.wdtcsr.write(mask);
    });
}
