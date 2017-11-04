use mcu::{TC1, Tc1Tccr1aFlags, Tc1Tccr1bFlags, Tc1Tccr1cFlags, Tc1Timsk1Flags};
use mutex::interrupt_free;

pub enum ClockSource {
    None,
    Prescale1,
    Prescale8,
    Prescale64,
    Prescale256,
    Prescale1024,
    ExternalFalling,
    ExternalRising,
}

impl ClockSource {
    #[inline]
    fn bits(&self) -> Tc1Tccr1bFlags {
        use self::ClockSource::*;
        match *self {
            None => Tc1Tccr1bFlags::CLK_SEL_3BIT_EXT_NO_CLOCK_SOURCE_STOPPED,
            Prescale1 => Tc1Tccr1bFlags::CLK_SEL_3BIT_EXT_RUNNING_NO_PRESCALING,
            Prescale8 => Tc1Tccr1bFlags::CLK_SEL_3BIT_EXT_RUNNING_CLK8,
            Prescale64 => Tc1Tccr1bFlags::CLK_SEL_3BIT_EXT_RUNNING_CLK64,
            Prescale256 => Tc1Tccr1bFlags::CLK_SEL_3BIT_EXT_RUNNING_CLK256,
            Prescale1024 => Tc1Tccr1bFlags::CLK_SEL_3BIT_EXT_RUNNING_CLK1024,
            ExternalFalling => Tc1Tccr1bFlags::CLK_SEL_3BIT_EXT_RUNNING_EXTCLK_TX_FALLING_EDGE,
            ExternalRising => Tc1Tccr1bFlags::CLK_SEL_3BIT_EXT_RUNNING_EXTCLK_TX_RISING_EDGE,
        }
    }
}

pub enum WaveformGenerationMode {
    Normal,
    PwmPhaseCorrect8Bit,
    PwmPhaseCorrect9Bit,
    PwmPhaseCorrect10Bit,
    ClearOnTimerMatchOutputCompare,
    FastPwm8Bit,
    FastPwm9Bit,
    FastPwm10Bit,
    PwmPhaseAndFrequencyCorrectInputCapture,
    PwmPhaseAndFrequencyCorrectOutputCompare,
    PwmPhaseCorrectInputCapture,
    PwmPhaseCorrectOutputCompare,
    ClearOnTimerMatchInputCapture,
    FastPwmInputCapture,
    FastPwmOutputCompare,
}

const WGM10: Tc1Tccr1aFlags = Tc1Tccr1aFlags::from_bits(1 << 0);
const WGM11: Tc1Tccr1aFlags = Tc1Tccr1aFlags::from_bits(1 << 1);

const WGM12: Tc1Tccr1bFlags = Tc1Tccr1bFlags::from_bits(1 << 3);
const WGM13: Tc1Tccr1bFlags = Tc1Tccr1bFlags::from_bits(1 << 4);

impl WaveformGenerationMode {
    #[inline]
    fn bits(&self) -> (Tc1Tccr1bFlags, Tc1Tccr1aFlags) {
        use self::WaveformGenerationMode::*;
        match *self {
            Normal => (Tc1Tccr1bFlags::empty(), Tc1Tccr1aFlags::empty()),
            PwmPhaseCorrect8Bit => (Tc1Tccr1bFlags::empty(), WGM10),
            PwmPhaseCorrect9Bit => (Tc1Tccr1bFlags::empty(), WGM11),
            PwmPhaseCorrect10Bit => (Tc1Tccr1bFlags::empty(), WGM11 | WGM10),
            ClearOnTimerMatchOutputCompare => (WGM12, Tc1Tccr1aFlags::empty()),
            FastPwm8Bit => (WGM12, WGM10),
            FastPwm9Bit => (WGM12, WGM11),
            FastPwm10Bit => (WGM12, WGM11 | WGM10),
            PwmPhaseAndFrequencyCorrectInputCapture => (WGM13, Tc1Tccr1aFlags::empty()),
            PwmPhaseAndFrequencyCorrectOutputCompare => (WGM13, WGM10),
            PwmPhaseCorrectInputCapture => (WGM13, WGM11),
            PwmPhaseCorrectOutputCompare => (WGM13, WGM11 | WGM10),
            ClearOnTimerMatchInputCapture => (WGM13 | WGM12, Tc1Tccr1aFlags::empty()),
            // Reserved                              => (WGM13 | WGM12, WGM10),
            FastPwmInputCapture => (WGM13 | WGM12, WGM11),
            FastPwmOutputCompare => (WGM13 | WGM12, WGM11 | WGM10),
        }
    }
}

pub struct Timer {
    a: Tc1Tccr1aFlags,
    b: Tc1Tccr1bFlags,
    c: Tc1Tccr1cFlags,
    compare: Option<u16>,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            a: Tc1Tccr1aFlags::empty(),
            b: Tc1Tccr1bFlags::empty(),
            c: Tc1Tccr1cFlags::empty(),
            compare: None,
        }
    }

    pub fn waveform_generation_mode(mut self, wgm: WaveformGenerationMode) -> Self {
        let (b, a) = wgm.bits();

        self.a |= a;
        self.b |= b;

        self
    }

    pub fn clock_source(mut self, src: ClockSource) -> Self {
        self.b |= src.bits();
        self
    }

    pub fn output_compare_1(mut self, value: u16) -> Self {
        self.compare = Some(value);
        self
    }

    pub fn configure(self) {
        unsafe {
            interrupt_free(|_cs| {
                let tc1 = &(*TC1.get());
                tc1.tccr1a.write(self.a);
                tc1.tccr1b.write(self.b);
                tc1.tccr1c.write(self.c);
                tc1.tcnt1.write(0);

                if let Some(compare) = self.compare {
                    tc1.ocr1a.write(compare);
                    tc1.timsk1.write(Tc1Timsk1Flags::OCIE1A);
                }
            });
        }
    }
}
