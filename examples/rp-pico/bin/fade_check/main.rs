#![no_std]
#![no_main]

//! Visual banding check for the fixed-point color pipeline.
//!
//! Unlike the other examples this one runs PWM at the full 16-bit range, so a channel
//! value maps 1:1 onto a duty cycle and the LED shows exactly what the library computed.
//! At the 1000-count top the other examples use, the PWM is 32x coarser than the Q0.15
//! progress behind a fade and would hide any banding in its own quantization.
//!
//! Fades black to red and black to amber, ten seconds each, and reports the per-frame
//! step size over RTT. Watch the first second of each ramp: PWM duty is linear in light
//! while the channel values are gamma-encoded, so the dark end is where the eye is most
//! sensitive to stepping.

use cortex_m::delay::Delay;
use panic_halt as _;
use rp_pico::entry;
use rp_pico::hal::{
    Clock, Sio, Timer,
    clocks::init_clocks_and_plls,
    pac,
    pwm::{A, B, Channel, FreeRunning, Pwm1, Pwm2, Slice},
    watchdog::Watchdog,
};
use rtt_target::{rprintln, rtt_init_print};

use rp_pico_examples::rgb_led::PwmRgbLed;
use rp_pico_examples::time::{Duration, HardwareTimer, Instant};

use rgb_sequencer::{
    BLACK, DEFAULT_COLOR_EPSILON, LoopCount, RED, Rgb, RgbSequence8, RgbSequencer8, ServiceTiming,
    TimeDuration, TransitionStyle,
};

pub const FRAME_RATE_MS: u64 = 16;

/// Ten seconds keeps the per-frame step well above the epsilon threshold. Past about 16
/// seconds a full-range fade moves less than [`DEFAULT_COLOR_EPSILON`] per frame, and
/// the sequencer starts skipping LED writes - stepping that comes from the epsilon
/// rather than from the color math.
const FADE_MS: u64 = 10_000;

/// Zero rather than [`DEFAULT_COLOR_EPSILON`], so that every frame reaches the LED and
/// the statistics below measure the color pipeline instead of the write-suppression
/// policy layered over it.
///
/// With the default 64 the loop reports its own sampling as banding. Where the fade
/// reverses direction, the color genuinely moves less than the threshold and the write
/// is correctly suppressed - but `current_color()` then lags a frame, and the frame
/// after the turnaround measures a span of `32 - 2y` ms rather than 16, where `y` is how
/// far the boundary fell before the frame tick. That single parameter accounts for every
/// anomaly observed on hardware, high and low, including the occasional zero when the
/// following frame is suppressed as well.
const EPSILON: u16 = 0;

/// Print every frame as CSV for offline plotting. The summary below is computed on the
/// device and survives dropped RTT output; this does not.
const VERBOSE: bool = false;

/// Two channels moving at different rates, unlike a single primary.
const AMBER: Rgb = Rgb::from_u8(255, 128, 0);

/// Type alias for the RGB LED using PWM channels
pub type Led = PwmRgbLed<
    Channel<Slice<Pwm1, FreeRunning>, A>,
    Channel<Slice<Pwm1, FreeRunning>, B>,
    Channel<Slice<Pwm2, FreeRunning>, A>,
>;

/// Largest per-channel movement between two colors.
fn channel_step(from: Rgb, to: Rgb) -> u16 {
    from.r
        .abs_diff(to.r)
        .max(from.g.abs_diff(to.g))
        .max(from.b.abs_diff(to.b))
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== RP Pico Fade Banding Check ===");

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
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

    let sio = Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Full 16-bit PWM: no divider, top at 65535. 125 MHz / 65536 gives a 1.9 kHz carrier,
    // well clear of flicker fusion, and makes `max_duty_cycle` 65535 so the LED driver's
    // scaling is the identity. Phase-correct mode is left off because it would halve the
    // carrier for no benefit here.
    let mut pwm_slices = rp_pico::hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    pwm_slices.pwm1.set_div_int(1u8);
    pwm_slices.pwm1.set_top(u16::MAX);
    pwm_slices.pwm1.enable();

    pwm_slices.pwm2.set_div_int(1u8);
    pwm_slices.pwm2.set_top(u16::MAX);
    pwm_slices.pwm2.enable();

    let mut red_channel = pwm_slices.pwm1.channel_a;
    let mut green_channel = pwm_slices.pwm1.channel_b;
    let mut blue_channel = pwm_slices.pwm2.channel_a;

    red_channel.output_to(pins.gpio2);
    green_channel.output_to(pins.gpio3);
    blue_channel.output_to(pins.gpio4);

    let led = PwmRgbLed::new(red_channel, green_channel, blue_channel, true);

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let time_source = HardwareTimer::new(timer);

    let mut sequencer: RgbSequencer8<Instant, Led, HardwareTimer> =
        RgbSequencer8::with_epsilon(led, &time_source, EPSILON);

    let fade = Duration::from_millis(FADE_MS);
    let sequence = RgbSequence8::<Duration>::builder()
        .start_color(BLACK)
        .step(RED, fade, TransitionStyle::Linear)
        .unwrap()
        .step(BLACK, fade, TransitionStyle::Linear)
        .unwrap()
        .step(AMBER, fade, TransitionStyle::Linear)
        .unwrap()
        .step(BLACK, fade, TransitionStyle::Linear)
        .unwrap()
        .loop_count(LoopCount::Infinite)
        .build()
        .unwrap();

    sequencer.load_and_start(sequence).unwrap();

    // rtt-target's single-argument arm writes the string verbatim rather than
    // formatting it, so every placeholder below needs a positional argument.
    let expected = 65535 / (FADE_MS / FRAME_RATE_MS);
    rprintln!(
        "PWM top 65535, epsilon {} (library default {}), {} ms ramps",
        EPSILON,
        DEFAULT_COLOR_EPSILON,
        FADE_MS
    );
    rprintln!(
        "Expect every step at ~{} counts, at both ends of every ramp.",
        expected
    );
    rprintln!("A minimum of 0 means a frame produced no change - that is banding.");
    if VERBOSE {
        rprintln!("frame,r,g,b");
    }

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let mut previous = sequencer.current_color();
    let mut step_index = 0usize;
    let mut frames = 0u32;
    let mut updates = 0u32;
    let mut smallest = u16::MAX;
    let mut largest = 0u16;
    // Which frame the extremes landed on. A ramp is roughly 625 frames, so an anomaly
    // at either end came from the boundary and one in the middle did not.
    let mut smallest_at = 0u32;
    let mut largest_at = 0u32;

    loop {
        let timing = sequencer.service().unwrap();

        let color = sequencer.current_color();

        // Report once per ramp rather than per frame, so the numbers survive RTT drops.
        // Closing the previous ramp before this frame is counted keeps the boundary,
        // where the color barely moves, out of both ramps' statistics.
        if let Some(position) = sequencer.current_position()
            && position.step_index != step_index
        {
            rprintln!(
                "ramp {}: {} frames, {} updates, step {}@{}..{}@{}",
                step_index,
                frames,
                updates,
                smallest,
                smallest_at,
                largest,
                largest_at
            );
            step_index = position.step_index;
            frames = 0;
            updates = 0;
            smallest = u16::MAX;
            largest = 0;
            smallest_at = 0;
            largest_at = 0;
        }

        frames += 1;

        // The opening frame of a ramp only establishes the reference color.
        if frames > 1 {
            let moved = channel_step(previous, color);
            if moved > 0 {
                updates += 1;
            }
            if moved < smallest {
                smallest = moved;
                smallest_at = frames;
            }
            if moved > largest {
                largest = moved;
                largest_at = frames;
            }
        }
        previous = color;

        if VERBOSE {
            rprintln!("{},{},{},{}", frames, color.r, color.g, color.b);
        }

        match timing {
            ServiceTiming::Continuous => delay.delay_ms(FRAME_RATE_MS as u32),
            ServiceTiming::Delay(duration) => delay.delay_ms(duration.as_millis() as u32),
            ServiceTiming::Complete => break,
        }
    }

    rprintln!("Sequence complete");

    loop {
        cortex_m::asm::wfi();
    }
}
