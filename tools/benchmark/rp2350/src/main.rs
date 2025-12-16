#![no_std]
#![no_main]

mod bench;

use core::hint::black_box;
use rp235x_hal::entry;
use rp235x_hal::{Clock, clocks::init_clocks_and_plls, pac, watchdog::Watchdog};
use rtt_target::ChannelMode;

use panic_rtt_target as _;
use rtt_target::rprintln;

use bench::{
    BENCH_ITERATIONS, BenchLed, BenchTimeSource, HardwareTimer, Instant, Microseconds, RpTimer,
    WARMUP_ITERATIONS, cycles_to_micros,
};

use rgb_sequencer::{LoopCount, RgbSequence, RgbSequencer, TransitionStyle};

// RP2350 boot block - required for the chip to boot
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: rp235x_hal::block::ImageDef = rp235x_hal::block::ImageDef::secure_exe();

// Crystal frequency for RP2350 (12 MHz)
const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

const STEP_DURATION_MICROS: u64 = 1_000_000;

#[entry]
fn main() -> ! {
    rtt_target::rtt_init_print!(ChannelMode::NoBlockSkip);

    rprintln!("");
    rprintln!("RGB Sequencer Benchmark");
    rprintln!("=======================");
    rprintln!("");

    // Get peripherals
    let mut pac = pac::Peripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let cpu_freq_hz = clocks.system_clock.freq().to_Hz();
    let timer = rp235x_hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);
    let timer_freq_hz = RpTimer::calibrate(&timer, cpu_freq_hz);

    rprintln!("Platform: RP2350 (Cortex-M33F with FPU)");
    rprintln!("CPU Frequency: {} MHz", cpu_freq_hz / 1_000_000);
    rprintln!("");
    rprintln!("service() Performance");
    rprintln!("---------------------");
    rprintln!("Test Configuration: Time position at last step midpoint");
    rprintln!("");
    rprintln!("Transition       N=4          N=8          N=16         N=32");
    rprintln!("Style          cycles/µs    cycles/µs    cycles/µs    cycles/µs");
    rprintln!("============  ===========  ===========  ===========  ===========");

    fn bench_capacity<const N: usize>(
        transition: TransitionStyle,
        timer_freq_hz: u32,
        cpu_freq_hz: u32,
    ) -> u32 {
        let time_source = BenchTimeSource::new();
        let led = BenchLed::new();
        let mut sequencer =
            RgbSequencer::<Instant, BenchLed, BenchTimeSource, N>::new(led, &time_source);
        let mut builder = RgbSequence::<Microseconds, N>::builder();

        for i in 0..N {
            let hue = (i as f32 / N as f32) * 360.0;
            let color = rgb_sequencer::colors::hue(hue);
            builder = builder
                .step(
                    color,
                    Microseconds::from_micros(STEP_DURATION_MICROS),
                    transition,
                )
                .unwrap();
        }

        let sequence = builder.loop_count(LoopCount::Infinite).build().unwrap();
        sequencer.load_and_start(sequence).unwrap();

        let last_step_midpoint = Microseconds::from_micros(
            (((N - 1) as u64) * STEP_DURATION_MICROS) + (STEP_DURATION_MICROS / 2),
        );
        time_source.advance(last_step_midpoint);

        let mut samples = [0u32; BENCH_ITERATIONS as usize];
        for _ in 0..WARMUP_ITERATIONS {
            let _ = black_box(sequencer.service());
        }
        for sample in &mut samples {
            let timer = RpTimer::start(timer_freq_hz);
            let _ = black_box(sequencer.service());
            *sample = timer.elapsed_cycles(cpu_freq_hz);
        }

        let sum: u64 = samples.iter().map(|&x| x as u64).sum();
        (sum / samples.len() as u64) as u32
    }

    fn bench_row(name: &str, transition: TransitionStyle, timer_freq_hz: u32, cpu_freq_hz: u32) {
        let n4 = bench_capacity::<4>(transition, timer_freq_hz, cpu_freq_hz);
        let n8 = bench_capacity::<8>(transition, timer_freq_hz, cpu_freq_hz);
        let n16 = bench_capacity::<16>(transition, timer_freq_hz, cpu_freq_hz);
        let n32 = bench_capacity::<32>(transition, timer_freq_hz, cpu_freq_hz);

        let n4_us = cycles_to_micros(n4, cpu_freq_hz);
        let n8_us = cycles_to_micros(n8, cpu_freq_hz);
        let n16_us = cycles_to_micros(n16, cpu_freq_hz);
        let n32_us = cycles_to_micros(n32, cpu_freq_hz);

        rprintln!(
            "{:<12}    {:>4}/{:<4}    {:>4}/{:<4}   {:>5}/{:<3}    {:>5}/{:<3}",
            name,
            n4,
            n4_us,
            n8,
            n8_us,
            n16,
            n16_us,
            n32,
            n32_us
        );
    }

    bench_row("Step", TransitionStyle::Step, timer_freq_hz, cpu_freq_hz);
    bench_row(
        "Linear",
        TransitionStyle::Linear,
        timer_freq_hz,
        cpu_freq_hz,
    );
    bench_row(
        "EaseIn",
        TransitionStyle::EaseIn,
        timer_freq_hz,
        cpu_freq_hz,
    );
    bench_row(
        "EaseOut",
        TransitionStyle::EaseOut,
        timer_freq_hz,
        cpu_freq_hz,
    );
    bench_row(
        "EaseInOut",
        TransitionStyle::EaseInOut,
        timer_freq_hz,
        cpu_freq_hz,
    );
    bench_row(
        "EaseOutIn",
        TransitionStyle::EaseOutIn,
        timer_freq_hz,
        cpu_freq_hz,
    );

    rprintln!("");
    rprintln!("Benchmark complete.");

    loop {
        cortex_m::asm::wfi();
    }
}
