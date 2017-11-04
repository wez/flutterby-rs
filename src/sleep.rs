use mcu;
use core::ptr;

/// http://microchipdeveloper.com/8avr:avrsleep has more information on sleep modes
pub enum SleepMode {
    Idle,
    ADCNoiseReduction,
    PowerDown,
    PowerSave,
    StandyBy,
    ExtendedStandBy,
}

pub fn set_sleep_mode(mode: SleepMode) {
    use self::SleepMode::*;
    let flags = match mode {
        Idle => mcu::CpuSmcrFlags::CPU_SLEEP_MODE_3BITS_IDLE,
        ADCNoiseReduction => {
            mcu::CpuSmcrFlags::CPU_SLEEP_MODE_3BITS_ADC_NOISE_REDUCTION_IF_AVAILABLE
        }
        PowerDown => mcu::CpuSmcrFlags::CPU_SLEEP_MODE_3BITS_POWER_DOWN,
        PowerSave => mcu::CpuSmcrFlags::CPU_SLEEP_MODE_3BITS_POWER_SAVE,
        StandyBy => mcu::CpuSmcrFlags::CPU_SLEEP_MODE_3BITS_STANDBY,
        ExtendedStandBy => mcu::CpuSmcrFlags::CPU_SLEEP_MODE_3BITS_EXTENDED_STANDBY,
    };

    unsafe {
        // Dont flip the sleep enable bit; just set the mode flags
        (*mcu::CPU.get())
            .smcr
            .modify(|x| (x & mcu::CpuSmcrFlags::SE) | flags);
    }
}

pub fn sleep_enable() {
    unsafe {
        (*mcu::CPU.get()).smcr.modify(|x| x | mcu::CpuSmcrFlags::SE);
    }
}

pub fn sleep_disable() {
    unsafe {
        (*mcu::CPU.get()).smcr.modify(|x| x - mcu::CpuSmcrFlags::SE);
    }
}

pub fn sleep_cpu() {
    unsafe {
        asm!("SLEEP"::::"volatile");
    }
}

static mut PENDING: bool = false;

/// Intended to be called from a ISR that is queuing up work or otherwise
/// setting a flag for work to be done in the main non-interrupt context.
/// Setting pending status will avoid a race between the start of the
/// decision to initiate a sleep and an interrupt coming in while we
/// are setting up to sleep.
pub fn set_event_pending() {
    unsafe {
        ptr::write_volatile(&mut PENDING, true);
    }
}

/// Put the CPU into sleep mode, blocking until an interrupt occurs.
/// Clears any pending event state that may have been set by set_event_pending().
pub fn wait_for_event(mode: SleepMode) {
    unsafe {
        set_sleep_mode(mode);
        asm!("CLI" :::: "volatile");
        if !ptr::read_volatile(&PENDING) {
            sleep_enable();
            asm!("SEI" :::: "volatile");
            sleep_cpu();
            sleep_disable();
        }
        ptr::write_volatile(&mut PENDING, false);
        asm!("SEI"::::"volatile");
    }
}
