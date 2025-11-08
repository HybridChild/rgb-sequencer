#!/usr/bin/env rust-script
//! Memory calculator for rgb-sequencer
//!
//! This utility calculates and displays the exact memory footprint of RGB sequences
//! and full RgbSequencer instances with different configurations. Run it to understand
//! the memory implications of your sequence capacity and LED implementation choices.
//!
//! Usage:
//!   cargo run --bin sequence_memory_calculator --features std
//!
//! Or with rust-script:
//!   rust-script sequence_memory_calculator.rs

use embassy_time::Duration as EmbassyDurationInner;
use embassy_time::Instant as EmbassyInstantInner;
use palette::Srgb;
use rgb_sequencer::{
    LoopCount, RgbLed, RgbSequence, RgbSequencer, SequenceStep, TimeDuration, TimeInstant,
    TimeSource, TransitionStyle,
};
use std::mem::size_of;

// ============================================================================
// Mock Duration Types
// ============================================================================

// u32 milliseconds duration (like in stm32f0 examples)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Duration32(u32);

impl TimeDuration for Duration32 {
    const ZERO: Self = Duration32(0);

    fn as_millis(&self) -> u64 {
        self.0 as u64
    }

    fn from_millis(millis: u64) -> Self {
        Duration32(millis as u32)
    }

    fn saturating_sub(self, other: Self) -> Self {
        Duration32(self.0.saturating_sub(other.0))
    }
}

// u64 milliseconds duration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Duration64(u64);

impl TimeDuration for Duration64 {
    const ZERO: Self = Duration64(0);

    fn as_millis(&self) -> u64 {
        self.0
    }

    fn from_millis(millis: u64) -> Self {
        Duration64(millis)
    }

    fn saturating_sub(self, other: Self) -> Self {
        Duration64(self.0.saturating_sub(other.0))
    }
}

// Embassy Duration wrapper (like in stm32f0-embassy examples)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EmbassyDuration(EmbassyDurationInner);

impl TimeDuration for EmbassyDuration {
    const ZERO: Self = EmbassyDuration(EmbassyDurationInner::from_ticks(0));

    fn as_millis(&self) -> u64 {
        self.0.as_millis()
    }

    fn from_millis(millis: u64) -> Self {
        EmbassyDuration(EmbassyDurationInner::from_millis(millis))
    }

    fn saturating_sub(self, other: Self) -> Self {
        EmbassyDuration(EmbassyDurationInner::from_ticks(
            self.0.as_ticks().saturating_sub(other.0.as_ticks()),
        ))
    }
}

// ============================================================================
// Mock Instant Types
// ============================================================================

// u32 milliseconds instant
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Instant32(u32);

impl TimeInstant for Instant32 {
    type Duration = Duration32;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        Duration32(self.0.saturating_sub(earlier.0))
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_add(duration.0).map(Instant32)
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_sub(duration.0).map(Instant32)
    }
}

// u64 milliseconds instant
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Instant64(u64);

impl TimeInstant for Instant64 {
    type Duration = Duration64;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        Duration64(self.0.saturating_sub(earlier.0))
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_add(duration.0).map(Instant64)
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_sub(duration.0).map(Instant64)
    }
}

// Embassy Instant wrapper
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EmbassyInstant(EmbassyInstantInner);

impl TimeInstant for EmbassyInstant {
    type Duration = EmbassyDuration;

    fn duration_since(&self, earlier: Self) -> Self::Duration {
        EmbassyDuration(self.0 - earlier.0)
    }

    fn checked_add(self, duration: Self::Duration) -> Option<Self> {
        Some(EmbassyInstant(self.0 + duration.0))
    }

    fn checked_sub(self, duration: Self::Duration) -> Option<Self> {
        self.0.checked_sub(duration.0).map(EmbassyInstant)
    }
}

// ============================================================================
// Mock LED Types (representing different implementation sizes)
// ============================================================================

// Small LED - minimal GPIO implementation
#[repr(C)]
struct SmallLed {
    _pin_r: u8,
    _pin_g: u8,
    _pin_b: u8,
}

impl RgbLed for SmallLed {
    fn set_color(&mut self, _color: Srgb) {
        // Mock implementation
    }
}

// Medium LED - typical PWM implementation
#[repr(C)]
struct MediumLed {
    _pwm_r: u32,
    _pwm_g: u32,
    _pwm_b: u32,
    _max_duty: u16,
}

impl RgbLed for MediumLed {
    fn set_color(&mut self, _color: Srgb) {
        // Mock implementation
    }
}

// Large LED - complex driver with state
#[repr(C)]
struct LargeLed {
    _pwm_r: u32,
    _pwm_g: u32,
    _pwm_b: u32,
    _max_duty: u16,
    _gamma_table: [u8; 16],
    _calibration: [f32; 3],
}

impl RgbLed for LargeLed {
    fn set_color(&mut self, _color: Srgb) {
        // Mock implementation
    }
}

// ============================================================================
// Mock TimeSource Types
// ============================================================================

#[allow(dead_code)]
struct TimeSource32;

impl TimeSource<Instant32> for TimeSource32 {
    fn now(&self) -> Instant32 {
        Instant32(0)
    }
}

#[allow(dead_code)]
struct TimeSource64;

impl TimeSource<Instant64> for TimeSource64 {
    fn now(&self) -> Instant64 {
        Instant64(0)
    }
}

#[allow(dead_code)]
struct EmbassyTimeSource;

impl TimeSource<EmbassyInstant> for EmbassyTimeSource {
    fn now(&self) -> EmbassyInstant {
        EmbassyInstant(EmbassyInstantInner::from_ticks(0))
    }
}

// ============================================================================
// Display Functions
// ============================================================================

fn print_header() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        RGB Sequencer Memory Footprint Calculator               ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
}

fn print_component_sizes() {
    println!("Component Sizes:");
    println!(
        "├─ Srgb:                          {} bytes",
        size_of::<Srgb>()
    );
    println!(
        "├─ Option<Srgb>:                  {} bytes",
        size_of::<Option<Srgb>>()
    );
    println!(
        "├─ TransitionStyle (enum):         {} bytes",
        size_of::<TransitionStyle>()
    );
    println!(
        "├─ LoopCount (enum):               {} bytes",
        size_of::<LoopCount>()
    );
    println!(
        "├─ Color function pointer:         {} bytes",
        size_of::<Option<fn(Srgb, Duration64) -> Srgb>>()
    );
    println!(
        "└─ Timing function pointer:        {} bytes",
        size_of::<Option<fn(Duration64) -> Option<Duration64>>>()
    );
    println!();
}

fn print_duration_sizes() {
    println!("Duration Type Sizes:");
    println!(
        "├─ u32 (milliseconds):             {} bytes",
        size_of::<Duration32>()
    );
    println!(
        "├─ u64 (milliseconds):             {} bytes",
        size_of::<Duration64>()
    );
    println!(
        "└─ Embassy Duration (ticks):       {} bytes",
        size_of::<EmbassyDuration>()
    );
    println!();
}

fn print_instant_sizes() {
    println!("Instant Type Sizes:");
    println!(
        "├─ u32 (milliseconds):             {} bytes",
        size_of::<Instant32>()
    );
    println!(
        "├─ u64 (milliseconds):             {} bytes",
        size_of::<Instant64>()
    );
    println!(
        "└─ Embassy Instant (ticks):        {} bytes",
        size_of::<EmbassyInstant>()
    );
    println!();
}

fn print_led_sizes() {
    println!("LED Implementation Sizes:");
    println!(
        "├─ SmallLed (GPIO):                {} bytes",
        size_of::<SmallLed>()
    );
    println!(
        "├─ MediumLed (PWM):               {} bytes",
        size_of::<MediumLed>()
    );
    println!(
        "└─ LargeLed (complex driver):     {} bytes",
        size_of::<LargeLed>()
    );
    println!();
}

fn print_step_sizes() {
    println!("Step Sizes (by duration type):");
    println!(
        "├─ SequenceStep<u32>:             {} bytes",
        size_of::<SequenceStep<Duration32>>()
    );
    println!(
        "├─ SequenceStep<u64>:             {} bytes",
        size_of::<SequenceStep<Duration64>>()
    );
    println!(
        "└─ SequenceStep<EmbassyDuration>: {} bytes",
        size_of::<SequenceStep<EmbassyDuration>>()
    );
    println!();
}

fn print_sequence_table<D: TimeDuration + Copy>(duration_name: &str, capacities: &[usize])
where
    [(); 4]: Sized,
    [(); 8]: Sized,
    [(); 16]: Sized,
    [(); 32]: Sized,
    [(); 64]: Sized,
{
    println!("RgbSequence<{}, N> Memory Usage:", duration_name);
    println!("┌──────────┬──────────────┬─────────────────┬────────────────┐");
    println!("│ Capacity │ Sequence     │ Storage Cost    │ Overhead       │");
    println!("│ (N)      │ Total Size   │ (Step size * N) │ (Fixed)        │");
    println!("├──────────┼──────────────┼─────────────────┼────────────────┤");

    let step_size = size_of::<SequenceStep<D>>();

    for &capacity in capacities {
        let total_size = match capacity {
            4 => size_of::<RgbSequence<D, 4>>(),
            8 => size_of::<RgbSequence<D, 8>>(),
            16 => size_of::<RgbSequence<D, 16>>(),
            32 => size_of::<RgbSequence<D, 32>>(),
            64 => size_of::<RgbSequence<D, 64>>(),
            _ => continue,
        };

        let storage_cost = step_size * capacity;
        let overhead = total_size - storage_cost;

        println!(
            "│ {:^8} │ {:>10} B │ {:>13} B │ {:>12} B │",
            capacity, total_size, storage_cost, overhead
        );
    }

    println!("└──────────┴──────────────┴─────────────────┴────────────────┘");
    println!();
}

fn print_sequencer_table<I, L, T>(instant_name: &str, led_name: &str, capacities: &[usize])
where
    I: TimeInstant,
    I::Duration: TimeDuration,
    L: RgbLed,
    T: TimeSource<I>,
    [(); 4]: Sized,
    [(); 8]: Sized,
    [(); 16]: Sized,
    [(); 32]: Sized,
    [(); 64]: Sized,
{
    println!(
        "RgbSequencer<{}, {}, N> Memory Usage:",
        instant_name, led_name
    );
    println!("┌──────────┬──────────────┬─────────────────┬────────────────┐");
    println!("│ Capacity │ Sequencer    │ Sequence Size   │ Sequencer OH   │");
    println!("│ (N)      │ Total Size   │                 │ (Fixed)        │");
    println!("├──────────┼──────────────┼─────────────────┼────────────────┤");

    for &capacity in capacities {
        let total_size = match capacity {
            4 => size_of::<RgbSequencer<I, L, T, 4>>(),
            8 => size_of::<RgbSequencer<I, L, T, 8>>(),
            16 => size_of::<RgbSequencer<I, L, T, 16>>(),
            32 => size_of::<RgbSequencer<I, L, T, 32>>(),
            64 => size_of::<RgbSequencer<I, L, T, 64>>(),
            _ => continue,
        };

        let sequence_size = match capacity {
            4 => size_of::<RgbSequence<I::Duration, 4>>(),
            8 => size_of::<RgbSequence<I::Duration, 8>>(),
            16 => size_of::<RgbSequence<I::Duration, 16>>(),
            32 => size_of::<RgbSequence<I::Duration, 32>>(),
            64 => size_of::<RgbSequence<I::Duration, 64>>(),
            _ => continue,
        };

        let sequencer_overhead = total_size - sequence_size;

        println!(
            "│ {:^8} │ {:>10} B │ {:>13} B │ {:>12} B │",
            capacity, total_size, sequence_size, sequencer_overhead
        );
    }

    println!("└──────────┴──────────────┴─────────────────┴────────────────┘");
    println!();
}

fn main() {
    print_header();

    // Component sizes
    print_component_sizes();
    print_duration_sizes();
    print_instant_sizes();
    print_led_sizes();
    print_step_sizes();

    let capacities = vec![4, 8, 16, 32, 64];

    println!("═══════════════════════════════════════════════════════════════");
    println!("                    SEQUENCE MEMORY USAGE                      ");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    print_sequence_table::<Duration32>("u32", &capacities);
    print_sequence_table::<Duration64>("u64", &capacities);
    print_sequence_table::<EmbassyDuration>("EmbassyDuration", &capacities);

    println!("═══════════════════════════════════════════════════════════════");
    println!("                   SEQUENCER MEMORY USAGE                      ");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("Note: Sequencer overhead includes LED, state, time tracking,");
    println!("      and the owned sequence (shown in 'Sequence Size').");
    println!();

    // Show different LED implementation sizes with u64/Embassy timing
    println!("── With u64 Instant/Duration ──");
    println!();
    print_sequencer_table::<Instant64, SmallLed, TimeSource64>("u64", "SmallLed", &capacities);
    print_sequencer_table::<Instant64, MediumLed, TimeSource64>("u64", "MediumLed", &capacities);
    print_sequencer_table::<Instant64, LargeLed, TimeSource64>("u64", "LargeLed", &capacities);

    println!("── With Embassy Instant/Duration ──");
    println!();
    print_sequencer_table::<EmbassyInstant, SmallLed, EmbassyTimeSource>(
        "Embassy",
        "SmallLed",
        &capacities,
    );
    print_sequencer_table::<EmbassyInstant, MediumLed, EmbassyTimeSource>(
        "Embassy",
        "MediumLed",
        &capacities,
    );
    print_sequencer_table::<EmbassyInstant, LargeLed, EmbassyTimeSource>(
        "Embassy",
        "LargeLed",
        &capacities,
    );

    println!("═══════════════════════════════════════════════════════════════");
    println!("                      KEY INSIGHTS                             ");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("• Sequence overhead is constant regardless of capacity");
    println!("• Sequencer adds fixed overhead for LED + state tracking");
    println!("• LED implementation directly affects total memory");
    println!();
    println!("Architecture Note:");
    println!(
        "  Running on host ({}-bit). Embedded 32-bit targets will have",
        std::mem::size_of::<usize>() * 8
    );
    println!("  slightly smaller sizes due to pointer differences.");
    println!("  Step storage costs remain the same.");
    println!();
}
