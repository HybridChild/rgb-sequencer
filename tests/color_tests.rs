//! Integration tests for the Rgb color type

mod common;

use rgb_sequencer::{Rgb, WHITE};

#[test]
fn from_u8_round_trips() {
    for v in 0..=255u8 {
        let c = Rgb::from_u8(v, v, v);
        assert_eq!(c.to_u8(), (v, v, v), "round trip failed for {v}");
    }
    assert_eq!(Rgb::from_u8(255, 255, 255), Rgb::new(65535, 65535, 65535));
    assert_eq!(Rgb::from_u8(0, 0, 0), Rgb::new(0, 0, 0));
}

#[test]
fn lerp_hits_both_endpoints() {
    let a = Rgb::new(0, 65535, 12345);
    let b = Rgb::new(65535, 0, 54321);
    assert_eq!(a.lerp(b, 0), a);
    assert_eq!(a.lerp(b, 32768), b);
}

#[test]
fn lerp_reaches_the_target_in_both_directions() {
    // Interpolation branches on direction internally; both branches must land
    // exactly on the target rather than one LSB short.
    for (from, to) in [(0u16, 65535u16), (65535, 0), (30000, 40000), (40000, 30000)] {
        let a = Rgb::new(from, from, from);
        let b = Rgb::new(to, to, to);
        assert_eq!(a.lerp(b, 32768), b, "{from} -> {to}");
        assert_eq!(a.lerp(b, 0), a, "{from} -> {to}");
    }
}

#[test]
fn scale_extremes() {
    let c = Rgb::new(65535, 32768, 100);
    assert_eq!(c.scale(65535), c);
    assert_eq!(c.scale(0), Rgb::default());
}

#[test]
fn scale_by_full_is_lossless_across_the_range() {
    // A factor of 65535 must be the exact identity - an approximate divide by 65535
    // loses the top LSB and silently dims full brightness.
    for v in [0u16, 1, 255, 12345, 32768, 65534, 65535] {
        assert_eq!(
            Rgb::new(v, v, v).scale(65535),
            Rgb::new(v, v, v),
            "value {v}"
        );
    }
}

#[test]
fn scale_resolves_the_bottom_of_its_range() {
    // The point of a 16-bit factor: small factors have to produce distinct, evenly
    // spaced channel values. An 8-bit factor jumped 0 -> 257 -> 514 here, so the last
    // few steps of a fade to off were visibly coarse.
    for f in 1u16..=8 {
        assert_eq!(WHITE.scale(f), Rgb::new(f, f, f), "factor {f}");
    }
}

#[test]
fn approx_eq_uses_per_channel_distance() {
    let a = Rgb::new(1000, 1000, 1000);
    assert!(a.approx_eq(Rgb::new(1050, 1000, 1000), 64));
    assert!(!a.approx_eq(Rgb::new(1064, 1000, 1000), 64));
    // A large difference in a single channel is enough to differ.
    assert!(!a.approx_eq(Rgb::new(1000, 1000, 60000), 64));
}

#[test]
fn default_is_black() {
    assert_eq!(Rgb::default(), Rgb::new(0, 0, 0));
}
