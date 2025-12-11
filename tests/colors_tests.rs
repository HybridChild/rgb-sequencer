//! Integration tests for colors module

use palette::Srgb;
use rgb_sequencer::colors;

fn colors_equal(a: Srgb, b: Srgb) -> bool {
    const EPSILON: f32 = 0.01;
    (a.red - b.red).abs() < EPSILON
        && (a.green - b.green).abs() < EPSILON
        && (a.blue - b.blue).abs() < EPSILON
}

#[test]
fn hsv_creates_primary_colors() {
    // Red (hue = 0)
    let red = colors::hsv(0.0, 1.0, 1.0);
    assert!(colors_equal(red, Srgb::new(1.0, 0.0, 0.0)));

    // Green (hue = 120)
    let green = colors::hsv(120.0, 1.0, 1.0);
    assert!(colors_equal(green, Srgb::new(0.0, 1.0, 0.0)));

    // Blue (hue = 240)
    let blue = colors::hsv(240.0, 1.0, 1.0);
    assert!(colors_equal(blue, Srgb::new(0.0, 0.0, 1.0)));
}

#[test]
fn hsv_handles_saturation() {
    // Full saturation
    let full = colors::hsv(0.0, 1.0, 1.0);
    assert!(full.red > 0.99);

    // Zero saturation (gray)
    let gray = colors::hsv(0.0, 0.0, 0.5);
    assert!(colors_equal(gray, Srgb::new(0.5, 0.5, 0.5)));
}

#[test]
fn hsv_handles_value() {
    // Full value
    let bright = colors::hsv(0.0, 1.0, 1.0);
    assert!(bright.red > 0.99);

    // Half value
    let dim = colors::hsv(0.0, 1.0, 0.5);
    assert!(dim.red > 0.49 && dim.red < 0.51);

    // Zero value (black)
    let black = colors::hsv(0.0, 1.0, 0.0);
    assert!(colors_equal(black, Srgb::new(0.0, 0.0, 0.0)));
}

#[test]
fn hue_creates_fully_saturated_colors() {
    let red = colors::hue(0.0);
    assert!(colors_equal(red, Srgb::new(1.0, 0.0, 0.0)));

    let cyan = colors::hue(180.0);
    assert!(colors_equal(cyan, Srgb::new(0.0, 1.0, 1.0)));

    let yellow = colors::hue(60.0);
    assert!(colors_equal(yellow, Srgb::new(1.0, 1.0, 0.0)));
}

#[test]
fn hue_wraps_around_360() {
    // Hue should wrap, so 360 == 0
    let red1 = colors::hue(0.0);
    let red2 = colors::hue(360.0);
    assert!(colors_equal(red1, red2));
}
