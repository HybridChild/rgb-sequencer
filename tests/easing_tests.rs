//! Integration tests for transition curves and within-step progress.
//!
//! The Q0.15 arithmetic behind both is private, so these tests observe it through the
//! public API: a BLACK -> WHITE step makes the red channel a direct readout of eased
//! progress, at finer resolution than the value being checked.

mod common;

use common::{TestDuration, TestInstant};
use rgb_sequencer::sequence::RgbSequence;
use rgb_sequencer::types::TransitionStyle;
use rgb_sequencer::{BLACK, FULL, WHITE};

/// 1.0 in the Q0.15 format the library uses for progress internally.
const ONE: u32 = 32768;
const HALF: u32 = ONE / 2;

const ALL_STYLES: [TransitionStyle; 5] = [
    TransitionStyle::Step,
    TransitionStyle::Linear,
    TransitionStyle::EaseIn,
    TransitionStyle::EaseOut,
    TransitionStyle::EaseInOut,
];

/// Builds a BLACK -> WHITE step whose duration matches the Q0.15 scale, so elapsed
/// milliseconds and progress units are the same number.
fn curve(style: TransitionStyle) -> RgbSequence<TestDuration, 4> {
    RgbSequence::<TestDuration, 4>::builder()
        .start_color(BLACK)
        .step(WHITE, TestDuration(ONE as u64), style)
        .unwrap()
        .build()
        .unwrap()
}

/// Reads eased progress back out of the interpolated color, in Q0.15 units.
fn eased_at(sequence: &RgbSequence<TestDuration, 4>, t: u32) -> u32 {
    let color = sequence.evaluate(TestDuration(t as u64)).0;
    // Invert lerp(0, 65535, eased): channel = (65535 * eased) >> 15.
    (color.r as u32 * ONE + 32767) / 65535
}

/// The f32 easing curves the fixed-point implementation replaced, kept as its reference.
fn reference(t: f32, transition: TransitionStyle) -> f32 {
    match transition {
        TransitionStyle::Step => t,
        TransitionStyle::Linear => t,
        TransitionStyle::EaseIn => t * t,
        TransitionStyle::EaseOut => t * (2.0 - t),
        TransitionStyle::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                -1.0 + (4.0 - 2.0 * t) * t
            }
        }
    }
}

#[test]
fn every_curve_matches_the_f32_reference_across_its_whole_domain() {
    // `Step` holds its target for the whole duration rather than following a curve,
    // so it is exercised separately below.
    for style in ALL_STYLES.into_iter().skip(1) {
        let sequence = curve(style);
        for t in 0..ONE {
            let expected = (reference(t as f32 / ONE as f32, style) * ONE as f32) as i64;
            let actual = eased_at(&sequence, t) as i64;
            assert!(
                (expected - actual).abs() <= 3,
                "{style:?} at t={t}: got {actual}, expected ~{expected}"
            );
        }
    }
}

#[test]
fn every_curve_starts_at_the_source_color() {
    for style in ALL_STYLES.into_iter().skip(1) {
        assert_eq!(curve(style).evaluate(TestDuration(0)).0, BLACK, "{style:?}");
    }
}

#[test]
fn every_curve_reaches_the_target_color() {
    // One millisecond before the step ends the color is within a hair of WHITE, and
    // the interpolation must not overshoot past it.
    for style in ALL_STYLES.into_iter().skip(1) {
        let sequence = curve(style);
        let color = sequence.evaluate(TestDuration(ONE as u64 - 1)).0;
        assert!(
            color.r >= 65535 - 8,
            "{style:?} stopped short at {}",
            color.r
        );
    }
}

#[test]
fn every_curve_is_monotonic_and_stays_in_range() {
    for style in ALL_STYLES.into_iter().skip(1) {
        let sequence = curve(style);
        let mut previous = 0u16;
        for t in 0..ONE {
            let value = sequence.evaluate(TestDuration(t as u64)).0.r;
            assert!(value >= previous, "{style:?} went backwards at t={t}");
            previous = value;
        }
    }
}

#[test]
fn easing_curves_pass_through_the_midpoint_as_expected() {
    let midpoint = |style| eased_at(&curve(style), HALF) as i64;

    // EaseInOut is symmetric about the centre by construction.
    assert!((midpoint(TransitionStyle::EaseInOut) - HALF as i64).abs() <= 3);
    // EaseIn lags the midpoint, EaseOut leads it.
    assert!(midpoint(TransitionStyle::EaseIn) < HALF as i64);
    assert!(midpoint(TransitionStyle::EaseOut) > HALF as i64);
}

#[test]
fn step_transition_holds_its_target_for_the_whole_duration() {
    // No start_color here: the builder rejects pairing it with Step, since there is
    // nothing to interpolate from.
    let sequence = RgbSequence::<TestDuration, 4>::builder()
        .step(WHITE, TestDuration(ONE as u64), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    for t in [0, 1, HALF, ONE - 1] {
        assert_eq!(
            sequence.evaluate(TestDuration(t as u64)).0,
            WHITE,
            "Step should not interpolate, but moved at t={t}"
        );
    }
}

// Within-step progress. The library reduces both operands of elapsed / duration to 17
// bits with a shared shift so a 32-bit divide always suffices; the tests below exercise
// both the unshifted range and durations wide enough to need it.

/// Builds a linear BLACK -> WHITE step of an arbitrary duration.
fn ramp(duration_ms: u64) -> RgbSequence<TestDuration, 4> {
    RgbSequence::<TestDuration, 4>::builder()
        .start_color(BLACK)
        .step(WHITE, TestDuration(duration_ms), TransitionStyle::Linear)
        .unwrap()
        .build()
        .unwrap()
}

#[test]
fn progress_matches_the_elapsed_fraction() {
    for duration in [1u64, 2, 7, 100, 1000, 16_384, 131_071] {
        let sequence = ramp(duration);
        for n in 0..=32u64 {
            let elapsed = duration / 32 * n;
            if elapsed >= duration {
                continue;
            }
            let expected = (elapsed as f64 / duration as f64 * 65535.0) as i64;
            let actual = sequence.evaluate(TestDuration(elapsed)).0.r as i64;
            assert!(
                (expected - actual).abs() <= 4,
                "duration {duration}, elapsed {elapsed}: got {actual}, expected ~{expected}"
            );
        }
    }
}

#[test]
fn a_slow_fade_advances_in_even_steps() {
    // Banding on hardware is a fade that stalls or jumps, not one that ends up in the
    // wrong place, so bound the step rather than the value. Real fade durations are not
    // powers of two, which leaves the progress divide a remainder that the exact
    // 32768 ms case above never exercises.
    //
    // Elapsed truncates to progress and progress truncates to a channel value, and each
    // rounding can land a single step a count either side of the ideal. Three counts of
    // slack absorbs that; a stall or a doubled step is far larger.
    for duration in [2_000u64, 5_000, 10_000, 30_000] {
        let sequence = ramp(duration);
        let ideal = 65535.0 / duration as f64;

        let mut previous = 0u16;
        for t in 1..duration {
            let value = sequence.evaluate(TestDuration(t)).0.r;
            let step = value as f64 - previous as f64;
            assert!(
                step >= 1.0 && (step - ideal).abs() < 3.0,
                "{duration} ms fade stepped by {step} at t={t}, expected ~{ideal:.2}"
            );
            previous = value;
        }
    }
}

#[test]
fn progress_holds_up_for_durations_that_need_reduction() {
    // Durations above 131_071 ms force the shared-shift path. The boundary values
    // and the top of u64 are where an off-by-one in that reduction would show up.
    for duration in [131_071u64, 131_072, 131_073, 1 << 30, 1 << 40, u64::MAX] {
        let sequence = ramp(duration);

        assert_eq!(
            sequence.evaluate(TestDuration(0)).0,
            BLACK,
            "start of {duration}"
        );

        let mid = sequence.evaluate(TestDuration(duration / 2)).0.r as i64;
        assert!(
            (mid - 32767).abs() <= 8,
            "midpoint of {duration} was {mid}, expected ~32767"
        );

        let quarter = sequence.evaluate(TestDuration(duration / 4)).0.r as i64;
        assert!(
            (quarter - 16383).abs() <= 8,
            "quarter of {duration} was {quarter}, expected ~16383"
        );
    }
}

#[test]
fn progress_never_overshoots_its_target() {
    // The shared shift can make the reduced numerator equal the denominator. If that
    // produced a progress value above fully-complete, the interpolation would compute
    // a channel past full scale - which panics on overflow in debug builds and wraps
    // to near-black in release. Landing just under WHITE is the evidence it clamped.
    //
    // Only durations past 131_071 ms take the shift path; below that the operands are
    // used as-is.
    for duration in [131_071u64, 131_073, 1 << 30, 1 << 40, u64::MAX] {
        let color = ramp(duration).evaluate(TestDuration(duration - 1)).0;
        assert!(
            color.r >= 65535 - 8,
            "duration {duration} wrapped instead of clamping: {color:?}"
        );
    }

    // Very short steps cannot reach "just before the end" - one millisecond before a
    // one-millisecond step is its start - so they are only checked for sane bounds.
    for duration in [1u64, 2, 3] {
        let color = ramp(duration).evaluate(TestDuration(duration - 1)).0;
        assert!(color.g == color.r && color.b == color.r, "{duration}");
    }
}

#[test]
fn zero_duration_step_reports_complete_rather_than_dividing() {
    // A zero-duration step must short-circuit before the progress division.
    let sequence = RgbSequence::<TestDuration, 4>::builder()
        .step(WHITE, TestDuration(0), TransitionStyle::Step)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(sequence.evaluate(TestDuration(0)).0, WHITE);
}

/// Loads a single Step to WHITE, so the color reaching the LED is the brightness
/// factor's effect on its own - no easing or interpolation in play.
fn white_at_brightness(
    sequencer: &mut rgb_sequencer::RgbSequencer<
        TestInstant,
        common::MockLed,
        common::MockTimeSource,
        4,
    >,
    brightness: u16,
) -> rgb_sequencer::Rgb {
    sequencer.load(
        RgbSequence::<TestDuration, 4>::builder()
            .step(WHITE, TestDuration(1000), TransitionStyle::Step)
            .unwrap()
            .build()
            .unwrap(),
    );
    sequencer.set_brightness(brightness);
    sequencer.start().unwrap();
    sequencer.service().unwrap();
    sequencer.current_color()
}

#[test]
fn full_brightness_bypasses_scaling() {
    // service() short-circuits at unity rather than multiplying, so this pins the
    // bypass. The arithmetic path at unity is covered separately, in color_tests.
    use rgb_sequencer::{RgbSequencer, SequencerState};

    let led = common::MockLed::new();
    let timer = common::MockTimeSource::new();
    let mut sequencer =
        RgbSequencer::<TestInstant, common::MockLed, common::MockTimeSource, 4>::new(led, &timer);

    assert_eq!(white_at_brightness(&mut sequencer, FULL), WHITE);
    assert_eq!(sequencer.state(), SequencerState::Running);
}

#[test]
fn partial_brightness_scales_exactly() {
    // Below unity the bypass does not apply, so this is the multiply path itself.
    // Exact equality rather than a tolerance - the factor divides 65535 evenly here.
    use rgb_sequencer::{Rgb, RgbSequencer};

    let led = common::MockLed::new();
    let timer = common::MockTimeSource::new();
    let mut sequencer =
        RgbSequencer::<TestInstant, common::MockLed, common::MockTimeSource, 4>::new(led, &timer);

    let half = white_at_brightness(&mut sequencer, 32768);
    assert_eq!(half, Rgb::new(32768, 32768, 32768));
}
