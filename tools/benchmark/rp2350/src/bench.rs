use core::hint::black_box;
use palette::Srgb;
use rgb_sequencer::{RgbLed, TimeDuration, TimeInstant, TimeSource};

// Benchmark configuration
pub const WARMUP_ITERATIONS: u32 = 100;
pub const BENCH_ITERATIONS: u32 = 1000;

/// Duration type using microseconds for precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Microseconds(pub u64);

impl TimeDuration for Microseconds {
    const ZERO: Self = Microseconds(0);

    fn as_millis(&self) -> u64 {
        self.0 / 1000
    }

    fn from_millis(millis: u64) -> Self {
        Microseconds(millis * 1000)
    }

    fn saturating_sub(self, other: Self) -> Self {
        Microseconds(self.0.saturating_sub(other.0))
    }
}

impl Microseconds {
    pub const fn from_micros(micros: u64) -> Self {
        Microseconds(micros)
    }

    #[allow(dead_code)]
    pub const fn as_micros(&self) -> u64 {
        self.0
    }
}

/// Instant type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(u64);

impl TimeInstant for Instant {
    type Duration = Microseconds;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        Microseconds(self.0.saturating_sub(earlier.0))
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        Some(Instant(self.0.saturating_add(duration.0)))
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_sub(duration.0).map(Instant)
    }
}

/// Minimal LED that just accepts colors (no actual hardware)
pub struct BenchLed {
    current_color: core::cell::Cell<Srgb>,
}

impl BenchLed {
    pub fn new() -> Self {
        Self {
            current_color: core::cell::Cell::new(Srgb::new(0.0, 0.0, 0.0)),
        }
    }
}

impl RgbLed for BenchLed {
    fn set_color(&mut self, color: Srgb) {
        black_box(self.current_color.set(color));
    }
}

/// Benchmark time source
pub struct BenchTimeSource {
    current_time: core::cell::Cell<u64>,
}

impl BenchTimeSource {
    pub fn new() -> Self {
        Self {
            current_time: core::cell::Cell::new(0),
        }
    }

    pub fn advance(&self, duration: Microseconds) {
        let new_time = self.current_time.get() + duration.0;
        self.current_time.set(new_time);
    }
}

impl TimeSource<Instant> for BenchTimeSource {
    fn now(&self) -> Instant {
        Instant(self.current_time.get())
    }
}

pub fn cycles_to_micros(cycles: u32, cpu_freq_hz: u32) -> u32 {
    cycles / (cpu_freq_hz / 1_000_000)
}

/// Hardware timer trait - implemented per-platform
pub trait HardwareTimer {
    /// Start timing with calibrated timer frequency (in Hz)
    fn start(timer_freq_hz: u32) -> Self;
    /// Get elapsed cycles
    fn elapsed_cycles(&self, cpu_freq_hz: u32) -> u32;
}

/// RP2040/RP2350 hardware timer
/// Auto-calibrates timer frequency at runtime
pub struct RpTimer {
    start_ticks: u64,
    timer_freq_hz: u32, // Actual timer frequency in Hz (measured at runtime)
}

impl RpTimer {
    #[inline(always)]
    fn read_timer_raw() -> u64 {
        unsafe {
            const TIMERAWH: u32 = 0x400B0024;
            const TIMERAWL: u32 = 0x400B0028;

            let mut hi0 = core::ptr::read_volatile(TIMERAWH as *const u32) as u64;
            loop {
                let low = core::ptr::read_volatile(TIMERAWL as *const u32) as u64;
                let hi1 = core::ptr::read_volatile(TIMERAWH as *const u32) as u64;
                if hi0 == hi1 {
                    break (hi0 << 32) | low;
                }
                hi0 = hi1;
            }
        }
    }

    pub fn calibrate<D>(_timer: &rp235x_hal::Timer<D>, cpu_freq_hz: u32) -> u32
    where
        D: rp235x_hal::timer::TimerDevice,
    {
        let mut core_peripherals = cortex_m::Peripherals::take().unwrap();
        core_peripherals.DCB.enable_trace();
        core_peripherals.DWT.enable_cycle_counter();

        let calibration_cycles = cpu_freq_hz / 100;

        unsafe { core_peripherals.DWT.cyccnt.write(0) };
        let timer_start = Self::read_timer_raw();

        while core_peripherals.DWT.cyccnt.read() < calibration_cycles {}

        let timer_end = Self::read_timer_raw();
        let timer_ticks = timer_end.wrapping_sub(timer_start);

        (timer_ticks * 100) as u32
    }
}

impl HardwareTimer for RpTimer {
    #[inline(never)]
    fn start(timer_freq_hz: u32) -> Self {
        cortex_m::asm::dmb();
        let start_ticks = Self::read_timer_raw();
        cortex_m::asm::dmb();

        Self {
            start_ticks,
            timer_freq_hz,
        }
    }

    #[inline(never)]
    fn elapsed_cycles(&self, cpu_freq_hz: u32) -> u32 {
        cortex_m::asm::dmb();
        let end_ticks = Self::read_timer_raw();
        cortex_m::asm::dmb();

        let elapsed_ticks = end_ticks.wrapping_sub(self.start_ticks);
        ((elapsed_ticks * cpu_freq_hz as u64) / self.timer_freq_hz as u64) as u32
    }
}
