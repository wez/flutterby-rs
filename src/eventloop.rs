use arrayvec::{ArrayVec, CapacityError};
use mutex::Mutex;
use fcpu::F_CPU;
use core::ptr::{read_volatile, write_volatile};
use timer1;
use sleep;
use heap::Box;
use core::cell::RefCell;

const DESIRED_HZ_TIM1: f64 = 1.0;
const TIM1_PRESCALER: u64 = 1024;
const INTERRUPT_EVERY_1_HZ_1024_PRESCALER: u16 =
    ((F_CPU as f64 / (DESIRED_HZ_TIM1 * TIM1_PRESCALER as f64)) as u64 - 1) as u16;

pub type Ticks = u16;
static mut TICKS: Ticks = 0;

enum SlotEntry {
    Every(Box<RefCell<FnMut(Ticks)>>),
}

impl SlotEntry {
    fn run(&mut self, ticks: Ticks) -> bool {
        match self {
            &mut SlotEntry::Every(ref mut func) => {
                (*func.get_mut())(ticks);
            }
        }
        false
    }
}

enum CoreSlot {
    Vacant { next_vacant: usize },
    Occupied(Option<SlotEntry>),
}

struct EventLoopCore {
    slots: ArrayVec<[CoreSlot; 8]>,
    next_slot: usize,
}

impl EventLoopCore {
    fn configure_timer(&mut self) {
        timer1::Timer::new()
            .waveform_generation_mode(
                timer1::WaveformGenerationMode::ClearOnTimerMatchOutputCompare,
            )
            .clock_source(timer1::ClockSource::Prescale1024)
            .output_compare_1(INTERRUPT_EVERY_1_HZ_1024_PRESCALER)
            .configure();
    }

    fn add_slot(&mut self, slot: SlotEntry) -> Result<(), CapacityError<CoreSlot>> {
        let idx = self.next_slot;
        match self.slots.get_mut(idx) {
            Some(&mut CoreSlot::Vacant { next_vacant }) => {
                self.next_slot = next_vacant;
            }
            Some(_) => panic!("vacant points to running item"),
            None => {
                assert_eq!(idx, self.slots.len());
                self.slots.try_push(CoreSlot::Vacant { next_vacant: 0 })?;
                self.next_slot = idx + 1;
            }
        }
        self.slots[idx] = CoreSlot::Occupied(Some(slot));
        Ok(())
    }
}

pub struct EventLoop {
    inner: Mutex<EventLoopCore>,
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(EventLoopCore {
                slots: ArrayVec::new(),
                next_slot: 0,
            }),
        }
    }

    fn configure(&self) {
        let mut core = self.inner.lock();
        core.configure_timer();
    }

    pub fn run(&self) -> ! {
        self.configure();
        unsafe {
            // ensure that interrupts are enabled
            asm!("SEI"::::"volatile");
        }

        let mut last_tick = unsafe { read_volatile(&TICKS) };
        loop {
            let now_tick = unsafe { read_volatile(&TICKS) };
            let _elapsed_ticks = now_tick - last_tick;
            last_tick = now_tick;

            self.turn(now_tick);

            sleep::wait_for_event(sleep::SleepMode::Idle);
        }
    }

    fn turn(&self, current_tick: Ticks) {
        let num_slots = self.inner.lock().slots.len();

        for idx in 0..num_slots {
            let mut slot = match self.inner.lock().slots.get_mut(idx) {
                Some(&mut CoreSlot::Occupied(ref mut opt_slot)) if opt_slot.is_some() => {
                    opt_slot.take().unwrap()
                }
                _ => continue,
            };

            let completed = slot.run(current_tick);

            {
                let mut core = self.inner.lock();
                if completed {
                    core.slots[idx] = CoreSlot::Vacant {
                        next_vacant: core.next_slot,
                    };
                    core.next_slot = idx;
                } else {
                    core.slots[idx] = CoreSlot::Occupied(Some(slot));
                }
            }
        }
    }

    pub fn add_callback<F>(&self, f: F) -> Result<(), ()>
    where
        F: FnMut(Ticks) + 'static,
    {
        let mut core = self.inner.lock();
        let f = Box::try_new(RefCell::new(f))?;
        core.add_slot(SlotEntry::Every(f)).map_err(|_| ())?;
        Ok(())
    }
}

fn timer1_compare_a() {
    unsafe {
        write_volatile(&mut TICKS, read_volatile(&TICKS) + 1);
        sleep::set_event_pending();
    }
}

irq_handler!(TIMER1_COMPA, timer1_compare_a);
