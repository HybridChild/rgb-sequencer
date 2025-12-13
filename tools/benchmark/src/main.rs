#![no_std]
#![no_main]

use core::hint::black_box;
use panic_halt as _;
use palette::Srgb;
use rp_pico::entry;
use rp_pico::hal::{Clock, clocks::init_clocks_and_plls, pac, watchdog::Watchdog};
use rtt_target::{rprintln, rtt_init_print};

use rgb_sequencer::{
    LoopCount, RgbLed, RgbSequence, RgbSequencer, TimeDuration, TimeInstant, TimeSource,
    TransitionStyle,
};

// Constants
const WARMUP_ITERATIONS: u32 = 100;
const BENCH_ITERATIONS: u32 = 1000;

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
    current_color: Srgb,
}

impl BenchLed {
    pub fn new() -> Self {
        Self {
            current_color: Srgb::new(0.0, 0.0, 0.0),
        }
    }
}

impl RgbLed for BenchLed {
    fn set_color(&mut self, color: Srgb) {
        // Prevent optimizer from removing this
        self.current_color = black_box(color);
    }
}

/// Time source for benchmarking
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
        let current = self.current_time.get();
        self.current_time.set(current + duration.as_micros());
    }
}

impl TimeSource<Instant> for BenchTimeSource {
    fn now(&self) -> Instant {
        Instant(self.current_time.get())
    }
}

/// Hardware timer wrapper for benchmarking
/// Uses RP2040's TIMER peripheral (1 MHz, microsecond precision)
pub struct BenchTimer {
    start: u64,
}

impl BenchTimer {
    /// Start timing
    #[inline(never)]
    pub fn start() -> Self {
        cortex_m::asm::dmb();
        // Read RP2040 TIMER (TIMELR + TIMEHR at 0x40054000)
        let start = unsafe {
            let low = core::ptr::read_volatile(0x4005400C as *const u32) as u64;
            let high = core::ptr::read_volatile(0x40054010 as *const u32) as u64;
            (high << 32) | low
        };
        cortex_m::asm::dmb();
        Self { start }
    }

    /// Get elapsed time in nanoseconds (at 125 MHz, 1 cycle = 8ns)
    /// We measure in microseconds and convert to cycles
    #[inline(never)]
    pub fn elapsed_cycles(&self) -> u32 {
        cortex_m::asm::dmb();
        let end = unsafe {
            let low = core::ptr::read_volatile(0x4005400C as *const u32) as u64;
            let high = core::ptr::read_volatile(0x40054010 as *const u32) as u64;
            (high << 32) | low
        };
        cortex_m::asm::dmb();

        let micros = end.wrapping_sub(self.start);
        // Convert microseconds to cycles at 125 MHz (125 cycles per microsecond)
        (micros * 125) as u32
    }
}

/// Statistics for benchmark results
pub struct BenchStats {
    pub min: u32,
    pub max: u32,
    pub median: u32,
    pub mean: u32,
}

impl BenchStats {
    pub fn from_samples(samples: &mut [u32]) -> Self {
        samples.sort_unstable();

        let min = samples[0];
        let max = samples[samples.len() - 1];
        let median = samples[samples.len() / 2];
        let mean = {
            let sum: u64 = samples.iter().map(|&x| x as u64).sum();
            (sum / samples.len() as u64) as u32
        };

        Self { min, max, median, mean }
    }

    /// Convert cycles to microseconds at 125 MHz (125 cycles per microsecond)
    pub fn cycles_to_micros(cycles: u32) -> u32 {
        cycles / 125
    }

    /// Convert cycles to nanoseconds at 125 MHz (8 nanoseconds per cycle)
    pub fn cycles_to_nanos(cycles: u32) -> u32 {
        cycles * 8
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== RGB Sequencer Benchmark ===");

    // Get peripherals
    let mut pac = pac::Peripherals::take().unwrap();

    // Set up watchdog and clocks
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let freq = clocks.system_clock.freq().to_Hz();
    rprintln!("System clock: {} Hz", freq);
    rprintln!("Warmup iterations: {}", WARMUP_ITERATIONS);
    rprintln!("Benchmark iterations: {}", BENCH_ITERATIONS);
    rprintln!("");
    rprintln!("=== service() Performance (Time Position: Last Step) ===");
    rprintln!("");
    rprintln!("                   N=4        N=8        N=16       N=32");
    rprintln!("              cycles/µs  cycles/µs  cycles/µs  cycles/µs");
    rprintln!("  -----------------------------------------------------------");

    // Macro to benchmark a specific capacity N at last step
    macro_rules! bench_capacity {
        ($n:expr, $transition:expr) => {{
            let time_source = BenchTimeSource::new();
            let led = BenchLed::new();
            let mut sequencer = RgbSequencer::<Instant, BenchLed, BenchTimeSource, $n>::new(led, &time_source);
            let mut builder = RgbSequence::<Microseconds, $n>::builder();

            // Build sequence with $n steps
            for i in 0..$n {
                let hue = (i as f32 / $n as f32) * 360.0;
                let color = rgb_sequencer::colors::hue(hue);
                builder = builder
                    .step(color, Microseconds::from_micros(1_000_000), $transition)
                    .unwrap();
            }

            let sequence = builder.loop_count(LoopCount::Infinite).build().unwrap();
            sequencer.load_and_start(sequence).unwrap();

            // Advance to 50% through LAST step
            // Last step (step N-1) starts at (N-1) * 1000ms
            // 50% through last step = (N-1) * 1000ms + 500ms
            let last_step_midpoint = Microseconds::from_micros((($n - 1) * 1_000_000) + 500_000);
            time_source.advance(last_step_midpoint);

            // Benchmark service() at this position
            let mut samples = [0u32; BENCH_ITERATIONS as usize];
            for _ in 0..WARMUP_ITERATIONS {
                let _ = black_box(sequencer.service());
            }
            for sample in &mut samples {
                let timer = BenchTimer::start();
                let _ = black_box(sequencer.service());
                *sample = timer.elapsed_cycles();
            }

            let mean = {
                let sum: u64 = samples.iter().map(|&x| x as u64).sum();
                (sum / samples.len() as u64) as u32
            };
            mean
        }};
    }

    // Benchmark each transition style across all capacities
    macro_rules! bench_row {
        ($name:expr, $transition:expr) => {{
            let n4 = bench_capacity!(4, $transition);
            let n8 = bench_capacity!(8, $transition);
            let n16 = bench_capacity!(16, $transition);
            let n32 = bench_capacity!(32, $transition);

            let n4_us = BenchStats::cycles_to_micros(n4);
            let n8_us = BenchStats::cycles_to_micros(n8);
            let n16_us = BenchStats::cycles_to_micros(n16);
            let n32_us = BenchStats::cycles_to_micros(n32);

            rprintln!(
                "  {:<10}  {:>6}/{:<3} {:>6}/{:<3} {:>6}/{:<3} {:>6}/{:<3}",
                $name,
                n4, n4_us,
                n8, n8_us,
                n16, n16_us,
                n32, n32_us
            );
        }};
    }

    bench_row!("Step", TransitionStyle::Step);
    bench_row!("Linear", TransitionStyle::Linear);
    bench_row!("EaseIn", TransitionStyle::EaseIn);
    bench_row!("EaseOut", TransitionStyle::EaseOut);
    bench_row!("EaseInOut", TransitionStyle::EaseInOut);

    rprintln!("");
    rprintln!("=== Benchmark Complete - {} MHz ===", freq / 1_000_000);
    rprintln!("All benchmarks finished successfully!");

    loop {
        cortex_m::asm::wfi();
    }
}
