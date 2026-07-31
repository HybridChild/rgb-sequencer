//! Integration tests for colors module

mod common;

use common::{channel, colors_equal, rgb_f};

use rgb_sequencer::colors::{self, degrees};
use rgb_sequencer::{BLACK, BLUE, GREEN, RED, Rgb, WHITE};

/// Full saturation / full value, in the module's 0..=65535 scale.
const FULL: u16 = 65535;

#[test]
fn hsv_creates_primary_colors() {
    // Red (hue = 0) is the one sector boundary that lands exactly on a
    // representable hue; green and blue sit a part or two off it.
    let red = colors::hsv(0, FULL, FULL);
    assert!(colors_equal(red, rgb_f(1.0, 0.0, 0.0)));

    let green = colors::hsv(degrees(120), FULL, FULL);
    assert!(colors_equal(green, rgb_f(0.0, 1.0, 0.0)));

    let blue = colors::hsv(degrees(240), FULL, FULL);
    assert!(colors_equal(blue, rgb_f(0.0, 0.0, 1.0)));
}

#[test]
fn hsv_handles_saturation() {
    // Full saturation
    let full = colors::hsv(0, FULL, FULL);
    assert!(full.r > channel(0.99));

    // Zero saturation (gray)
    let gray = colors::hsv(0, 0, channel(0.5));
    assert!(colors_equal(gray, rgb_f(0.5, 0.5, 0.5)));
}

#[test]
fn hsv_handles_value() {
    // Full value
    let bright = colors::hsv(0, FULL, FULL);
    assert!(bright.r > channel(0.99));

    // Half value
    let dim = colors::hsv(0, FULL, channel(0.5));
    assert!(dim.r > channel(0.49) && dim.r < channel(0.51));

    // Zero value (black)
    let black = colors::hsv(0, FULL, 0);
    assert!(colors_equal(black, rgb_f(0.0, 0.0, 0.0)));
}

#[test]
fn hue_creates_fully_saturated_colors() {
    let red = colors::hue(0);
    assert!(colors_equal(red, rgb_f(1.0, 0.0, 0.0)));

    let cyan = colors::hue(degrees(180));
    assert!(colors_equal(cyan, rgb_f(0.0, 1.0, 1.0)));

    let yellow = colors::hue(degrees(60));
    assert!(colors_equal(yellow, rgb_f(1.0, 1.0, 0.0)));
}

#[test]
fn hue_wraps_around_360() {
    // Hue should wrap, so 360 == 0
    assert!(colors_equal(
        colors::hue(degrees(0)),
        colors::hue(degrees(360))
    ));
}

#[test]
fn hue_wraps_by_plain_addition() {
    // The wheel maps exactly onto u16, so rotation needs no range check: adding
    // past the top wraps to the equivalent hue on the other side.
    let quarter = degrees(90);
    let three_quarters = degrees(270);
    assert!(colors_equal(
        colors::hue(three_quarters.wrapping_add(quarter)),
        colors::hue(0)
    ));
}

#[test]
fn red_is_exact_and_other_primaries_are_within_a_hair() {
    // Hue 0 is the one sector boundary that lands exactly on a representable value;
    // the rest are off by a part or two in 65535, which vanishes at 8 bits.
    assert_eq!(colors::hue(0), RED);
    assert!(colors_equal(colors::hue(degrees(120)), GREEN));
    assert!(colors_equal(colors::hue(degrees(240)), BLUE));
    assert_eq!(colors::hue(degrees(120)).to_u8(), (0, 255, 0));
    assert_eq!(colors::hue(degrees(240)).to_u8(), (0, 0, 255));
}

#[test]
fn secondary_hues_are_close() {
    assert!(
        colors_equal(colors::hue(degrees(60)), Rgb::new(FULL, FULL, 0)),
        "yellow"
    );
    assert!(
        colors_equal(colors::hue(degrees(180)), Rgb::new(0, FULL, FULL)),
        "cyan"
    );
    assert!(
        colors_equal(colors::hue(degrees(300)), Rgb::new(FULL, 0, FULL)),
        "magenta"
    );
}

#[test]
fn zero_saturation_is_greyscale() {
    assert_eq!(colors::hsv(0, 0, FULL), WHITE);
    assert_eq!(colors::hsv(30000, 0, FULL), WHITE);
    assert_eq!(colors::hsv(12345, 0, 32768), Rgb::new(32768, 32768, 32768));
}

#[test]
fn zero_value_is_black() {
    for h in [0u16, 10000, 30000, 50000, 65535] {
        assert_eq!(colors::hsv(h, FULL, 0), BLACK, "hue {h}");
    }
}

#[test]
fn value_scales_every_channel() {
    assert!(colors_equal(
        colors::hsv(0, FULL, 32768),
        Rgb::new(32768, 0, 0)
    ));
    assert!(colors_equal(
        colors::hsv(degrees(180), FULL, 32768),
        Rgb::new(0, 32768, 32768)
    ));
}

#[test]
fn degrees_wraps_at_full_circle() {
    assert_eq!(degrees(0), degrees(360));
    assert_eq!(degrees(90), degrees(450));
    assert_eq!(colors::hue(degrees(360)), RED);
}

#[test]
fn every_hue_keeps_one_channel_full_and_one_empty() {
    // A fully saturated, full-value color always has one channel at full scale and
    // one at zero. This catches sector-boundary arithmetic errors.
    let mut h = 0u32;
    while h <= 65535 {
        let c = colors::hue(h as u16);
        let max = c.r.max(c.g).max(c.b);
        let min = c.r.min(c.g).min(c.b);
        assert!(max >= FULL - 128, "hue {h} peaked at only {max}");
        assert!(min <= 128, "hue {h} bottomed out at {min}");
        h += 7;
    }
}
