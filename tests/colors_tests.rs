//! Integration tests for colors module

mod common;

use common::{channel, colors_equal, rgb_f};

use rgb_sequencer::colors::{self, degrees};

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
