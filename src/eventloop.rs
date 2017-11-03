use arrayvec::ArrayVec;
use mutex::Mutex;
use fcpu::F_CPU;
use arduino::timer1;
use core::ptr::{read_volatile, write_volatile};

const DESIRED_HZ_TIM1: f64 = 1.0;
const TIM1_PRESCALER: u64 = 1024;
const INTERRUPT_EVERY_1_HZ_1024_PRESCALER: u16 =
    ((F_CPU as f64 / (DESIRED_HZ_TIM1 * TIM1_PRESCALER as f64)) as u64 - 1) as u16;

static mut TICKS: u16 = 0;

struct EventLoopCore {
    funcs: ArrayVec<[fn();8]>,
}

impl EventLoopCore {
    fn configure_timer(&mut self) {
        timer1::Timer::new()
            .waveform_generation_mode(
                timer1::WaveformGenerationMode::ClearOnTimerMatchOutputCompare,
            )
            .clock_source(timer1::ClockSource::Prescale1024)
            .output_compare_1(Some(INTERRUPT_EVERY_1_HZ_1024_PRESCALER))
            .configure();
    }
}

pub struct EventLoop {
    inner: Mutex<EventLoopCore>
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(EventLoopCore {
                funcs: ArrayVec::new()
            })
        }
    }

    pub fn run(&self) -> ! {
        let mut core = self.inner.lock();
        core.configure_timer();

        let mut last_tick = unsafe{read_volatile(&TICKS)};
        loop {
            let now_tick = unsafe{read_volatile(&TICKS)};

            if now_tick != last_tick {
                last_tick = now_tick;

                let core = self.inner.lock();
                for f in core.funcs.iter() {
                    (f)();
                }
            }
        }
    }

    pub fn add_callback(&self, f: fn()) {
        let mut core = self.inner.lock();
        core.funcs.push(f);
    }
}

#[no_mangle]
pub unsafe extern "avr-interrupt" fn _ivr_timer1_compare_a() {
    write_volatile(&mut TICKS, read_volatile(&TICKS) + 1);
}
