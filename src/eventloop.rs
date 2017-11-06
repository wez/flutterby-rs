use arrayvec::{ArrayVec, CapacityError};
use mutex::Mutex;
use fcpu::F_CPU;
use core::ptr::{read_volatile, write_volatile};
use timer1;
use sleep;
use heap::Box;
use core::cell::RefCell;
use futures::{Async, Future};
use futures::Stream;
use core::ops;

const DESIRED_HZ_TIM1: f64 = 50.0;
const TIM1_PRESCALER: u64 = 1024;
const INTERRUPT_EVERY_1_HZ_1024_PRESCALER: u16 =
    ((F_CPU as f64 / (DESIRED_HZ_TIM1 * TIM1_PRESCALER as f64)) as u64 - 1) as u16;
const TICKS_PER_MS: u16 = (1000.0 / DESIRED_HZ_TIM1) as u16;

#[derive(Copy, Clone, Default, Debug, PartialOrd, Ord, Eq, PartialEq)]
pub struct Ticks {
    ticks: u16,
}

impl Ticks {
    pub fn current() -> Self {
        unsafe {
            Self {
                // FIXME: interrupt_free() around this?
                ticks: read_volatile(&TICKS.ticks),
            }
        }
    }

    pub const fn milliseconds(ms: u16) -> Self {
        Self {
            ticks: ms * TICKS_PER_MS,
        }
    }

    pub const fn new(ticks: u16) -> Ticks {
        Self { ticks }
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.ticks == 0
    }
}

impl ops::Sub for Ticks {
    type Output = Ticks;
    fn sub(self, other: Ticks) -> Ticks {
        Ticks::new(self.ticks - other.ticks)
    }
}

impl ops::SubAssign for Ticks {
    fn sub_assign(&mut self, other: Ticks) {
        self.ticks -= other.ticks;
    }
}

impl ops::Add for Ticks {
    type Output = Ticks;
    fn add(self, other: Ticks) -> Ticks {
        Ticks::new(self.ticks + other.ticks)
    }
}

impl ops::AddAssign for Ticks {
    fn add_assign(&mut self, other: Ticks) {
        self.ticks += other.ticks;
    }
}

static mut TICKS: Ticks = Ticks { ticks: 0 };

/// Countdown is used to execute work after a delay.
/// A future iteration of this module will use the remaining_ticks
/// information to revise the clock settings so that longer sleep
/// periods and thus lower power utilization can be realized.
struct Countdown<F: FnMut(Ticks) + ?Sized> {
    remaining_ticks: Ticks,
    repeat: Ticks,
    func: RefCell<F>,
}

enum SlotEntry {
    /// Runs callback on every turn of the core
    Every(Box<RefCell<FnMut(Ticks, Ticks)>>),
    /// Polls the future on every turn of the core
    Future(Box<Future<Item = (), Error = ()>>),
    /// Polls the stream on every turn of the core
    Stream(Box<Stream<Item = (), Error = ()>>),
    Countdown(Box<Countdown<FnMut(Ticks)>>),
}

impl SlotEntry {
    fn run(&mut self, now: Ticks, elapsed: Ticks) -> bool {
        match self {
            &mut SlotEntry::Every(ref mut func) => {
                (*func.get_mut())(now, elapsed);
                false
            }
            &mut SlotEntry::Future(ref mut future) => match future.poll() {
                Ok(Async::Ready(())) | Err(()) => true,
                Ok(Async::NotReady) => false,
            },
            &mut SlotEntry::Stream(ref mut stream) => match stream.poll() {
                Ok(Async::Ready(None)) => true,
                _ => false,
            },
            &mut SlotEntry::Countdown(ref mut countdown) => {
                if elapsed > countdown.remaining_ticks {
                    (*countdown.func.get_mut())(now);

                    if !countdown.repeat.is_zero() {
                        countdown.remaining_ticks = countdown.repeat;
                        return false;
                    }
                    return true;
                }

                countdown.remaining_ticks -= elapsed;
                false
            }
        }
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
                logln!("idx is ", idx, " going to push a vacant entry");
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
        logln!("in run");
        self.configure();
        logln!("after configure");
        unsafe {
            // ensure that interrupts are enabled
            asm!("SEI"::::"volatile");
        }
        logln!("starting run");

        let mut last_tick = unsafe { read_volatile(&TICKS) };
        loop {
            let now_tick = unsafe { read_volatile(&TICKS) };
            let elapsed_ticks = now_tick - last_tick;
            last_tick = now_tick;

            self.turn(now_tick, elapsed_ticks);

            logln!("sleep");
            sleep::wait_for_event(sleep::SleepMode::Idle);
        }
    }

    fn turn(&self, current_tick: Ticks, elapsed_ticks: Ticks) {
        let num_slots = self.inner.lock().slots.len();

        for idx in 0..num_slots {
            let mut slot = match self.inner.lock().slots.get_mut(idx) {
                Some(&mut CoreSlot::Occupied(ref mut opt_slot)) if opt_slot.is_some() => {
                    opt_slot.take().unwrap()
                }
                _ => continue,
            };

            let completed = slot.run(current_tick, elapsed_ticks);

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

    /// Register a callback that will be called on every
    /// turn of the event loop core.  Please consider using
    /// either spawn() or spawn_stream() with `Future` or
    /// `Stream` instances that can be composed with other
    /// portions of work.
    pub fn add_callback<F>(&self, f: F) -> Result<(), ()>
    where
        F: FnMut(Ticks, Ticks) + 'static,
    {
        let mut core = self.inner.lock();
        let f = Box::try_new(RefCell::new(f))?;
        core.add_slot(SlotEntry::Every(f)).map_err(|_| ())?;
        Ok(())
    }

    /// Drive a Future to completion asynchronously.
    /// The even loop takes ownership and will poll the
    /// Future continuously until it completes.
    pub fn spawn<F>(&self, f: F) -> Result<(), ()>
    where
        F: Future<Item = (), Error = ()> + 'static,
    {
        let mut core = self.inner.lock();
        let f = Box::try_new(f)?;
        core.add_slot(SlotEntry::Future(f)).map_err(|_| ())?;
        Ok(())
    }

    /// Drive a Stream to completion asynchronously.
    /// The even loop takes ownership and will poll the
    /// Stream continuously until it completes.
    pub fn spawn_stream<S>(&self, s: S) -> Result<(), ()>
    where
        S: Stream<Item = (), Error = ()> + 'static,
    {
        let mut core = self.inner.lock();
        let s = Box::try_new(s)?;
        core.add_slot(SlotEntry::Stream(s)).map_err(|_| ())?;
        Ok(())
    }

    pub fn spawn_after<F>(&self, f: F, after: Ticks) -> Result<(), ()>
    where
        F: FnMut(Ticks) + 'static,
    {
        let mut core = self.inner.lock();
        let s = Box::try_new(Countdown {
            remaining_ticks: after,
            repeat: Ticks::default(),
            func: RefCell::new(f),
        })?;
        core.add_slot(SlotEntry::Countdown(s)).map_err(|_| ())?;
        Ok(())
    }

    pub fn spawn_repeating<F>(&self, f: F, every: Ticks) -> Result<(), ()>
    where
        F: FnMut(Ticks) + 'static,
    {
        let mut core = self.inner.lock();
        let s = Box::try_new(Countdown {
            remaining_ticks: every,
            repeat: every,
            func: RefCell::new(f),
        })?;
        core.add_slot(SlotEntry::Countdown(s)).map_err(|_| ())?;
        logln!("made it down here");
        Ok(())
    }
}

fn timer1_compare_a() {
    unsafe {
        write_volatile(&mut TICKS.ticks, read_volatile(&TICKS.ticks) + 1);
        sleep::set_event_pending();
    }
}

irq_handler!(TIMER1_COMPA, timer1_compare_a);
