//! Memory calculator for rgb-sequencer
//!
//! This utility calculates and displays the exact memory footprint of RGB sequences
//! and full RgbSequencer instances with different configurations. Run it to understand
//! the memory implications of your sequence capacity and LED implementation choices.
//!
//! Usage:
//!   cd tools/memory-calculator
//!   cargo run --release
//!   cat report.md

use embassy_time::Duration as EmbassyDurationInner;
use embassy_time::Instant as EmbassyInstantInner;
use palette::Srgb;
use rgb_sequencer::{
    LoopCount, RgbLed, RgbSequence, RgbSequencer, SequenceStep, TimeDuration, TimeInstant,
    TimeSource, TransitionStyle,
};
use std::fs::File;
use std::io::Write;
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
// Report Generation Functions
// ============================================================================

fn write_header(f: &mut File) -> std::io::Result<()> {
    writeln!(f, "# RGB Sequencer Memory Footprint Analysis")?;
    writeln!(f)?;
    writeln!(
        f,
        "**Generated:** {}  ",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;
    writeln!(
        f,
        "**Architecture:** {}-bit host",
        std::mem::size_of::<usize>() * 8
    )?;
    writeln!(f)?;
    Ok(())
}

fn write_component_sizes(f: &mut File) -> std::io::Result<()> {
    writeln!(f, "## Component Sizes")?;
    writeln!(f)?;
    writeln!(f, "| Component | Size |")?;
    writeln!(f, "|-----------|------|")?;
    writeln!(f, "| `Srgb` | {} B |", size_of::<Srgb>())?;
    writeln!(f, "| `Option<Srgb>` | {} B |", size_of::<Option<Srgb>>())?;
    writeln!(
        f,
        "| `TransitionStyle` | {} B |",
        size_of::<TransitionStyle>()
    )?;
    writeln!(f, "| `LoopCount` | {} B |", size_of::<LoopCount>())?;
    writeln!(
        f,
        "| Color function pointer | {} B |",
        size_of::<Option<fn(Srgb, Duration64) -> Srgb>>()
    )?;
    writeln!(
        f,
        "| Timing function pointer | {} B |",
        size_of::<Option<fn(Duration64) -> Option<Duration64>>>()
    )?;
    writeln!(f)?;
    Ok(())
}

fn write_duration_sizes(f: &mut File) -> std::io::Result<()> {
    writeln!(f, "## Duration Type Sizes")?;
    writeln!(f)?;
    writeln!(f, "| Type | Size |")?;
    writeln!(f, "|------|------|")?;
    writeln!(
        f,
        "| `u32` (milliseconds) | {} B |",
        size_of::<Duration32>()
    )?;
    writeln!(
        f,
        "| `u64` (milliseconds) | {} B |",
        size_of::<Duration64>()
    )?;
    writeln!(
        f,
        "| Embassy `Duration` | {} B |",
        size_of::<EmbassyDuration>()
    )?;
    writeln!(f)?;
    Ok(())
}

fn write_instant_sizes(f: &mut File) -> std::io::Result<()> {
    writeln!(f, "## Instant Type Sizes")?;
    writeln!(f)?;
    writeln!(f, "| Type | Size |")?;
    writeln!(f, "|------|------|")?;
    writeln!(f, "| `u32` (milliseconds) | {} B |", size_of::<Instant32>())?;
    writeln!(f, "| `u64` (milliseconds) | {} B |", size_of::<Instant64>())?;
    writeln!(
        f,
        "| Embassy `Instant` | {} B |",
        size_of::<EmbassyInstant>()
    )?;
    writeln!(f)?;
    Ok(())
}

fn write_led_sizes(f: &mut File) -> std::io::Result<()> {
    writeln!(f, "## LED Implementation Sizes")?;
    writeln!(f)?;
    writeln!(f, "| Implementation | Size | Description |")?;
    writeln!(f, "|----------------|------|-------------|")?;
    writeln!(
        f,
        "| Small | {} B | Minimal GPIO (3× u8 pins) |",
        size_of::<SmallLed>()
    )?;
    writeln!(
        f,
        "| Medium | {} B | PWM (3× u32 channels + u16 duty) |",
        size_of::<MediumLed>()
    )?;
    writeln!(
        f,
        "| Large | {} B | Complex (PWM + gamma table + calibration) |",
        size_of::<LargeLed>()
    )?;
    writeln!(f)?;
    Ok(())
}

fn write_step_sizes(f: &mut File) -> std::io::Result<()> {
    writeln!(f, "## `SequenceStep` Sizes")?;
    writeln!(f)?;
    writeln!(f, "| Duration Type | Step Size |")?;
    writeln!(f, "|---------------|-----------|")?;
    writeln!(f, "| `u32` | {} B |", size_of::<SequenceStep<Duration32>>())?;
    writeln!(f, "| `u64` | {} B |", size_of::<SequenceStep<Duration64>>())?;
    writeln!(
        f,
        "| Embassy | {} B |",
        size_of::<SequenceStep<EmbassyDuration>>()
    )?;
    writeln!(f)?;
    Ok(())
}

fn write_sequence_table<D: TimeDuration + Copy>(
    f: &mut File,
    duration_name: &str,
    capacities: &[usize],
) -> std::io::Result<()>
where
    [(); 4]: Sized,
    [(); 8]: Sized,
    [(); 16]: Sized,
    [(); 32]: Sized,
    [(); 64]: Sized,
{
    writeln!(f, "### `RgbSequence<{}, N>`", duration_name)?;
    writeln!(f)?;
    writeln!(f, "| Capacity | Total Size | Storage Cost | Overhead |")?;
    writeln!(f, "|----------|------------|--------------|----------|")?;

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

        writeln!(
            f,
            "| {} | {} B | {} B | {} B |",
            capacity, total_size, storage_cost, overhead
        )?;
    }

    writeln!(f)?;
    Ok(())
}

fn write_sequencer_table<'t, I, L, T>(
    f: &mut File,
    instant_name: &str,
    led_name: &str,
    capacities: &[usize],
) -> std::io::Result<()>
where
    I: TimeInstant,
    I::Duration: TimeDuration,
    L: RgbLed,
    T: TimeSource<I> + 't,
    [(); 4]: Sized,
    [(); 8]: Sized,
    [(); 16]: Sized,
    [(); 32]: Sized,
    [(); 64]: Sized,
{
    writeln!(f, "### `RgbSequencer<{}, {}, N>`", instant_name, led_name)?;
    writeln!(f)?;
    writeln!(
        f,
        "| Capacity | Total Size | Sequence Size | Sequencer OH |"
    )?;
    writeln!(
        f,
        "|----------|------------|---------------|--------------|"
    )?;

    for &capacity in capacities {
        let total_size = match capacity {
            4 => size_of::<RgbSequencer<'t, I, L, T, 4>>(),
            8 => size_of::<RgbSequencer<'t, I, L, T, 8>>(),
            16 => size_of::<RgbSequencer<'t, I, L, T, 16>>(),
            32 => size_of::<RgbSequencer<'t, I, L, T, 32>>(),
            64 => size_of::<RgbSequencer<'t, I, L, T, 64>>(),
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

        writeln!(
            f,
            "| {} | {} B | {} B | {} B |",
            capacity, total_size, sequence_size, sequencer_overhead
        )?;
    }

    writeln!(f)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let report_path = "report.md";

    // Remove old report if it exists
    let _ = std::fs::remove_file(report_path);

    let mut file = File::create(report_path)?;

    println!("Generating memory footprint analysis...");

    // Header and metadata
    write_header(&mut file)?;

    // Component sizes
    write_component_sizes(&mut file)?;
    write_duration_sizes(&mut file)?;
    write_instant_sizes(&mut file)?;
    write_led_sizes(&mut file)?;
    write_step_sizes(&mut file)?;

    let capacities = vec![4, 8, 16, 32, 64];

    // Sequence tables
    writeln!(&mut file, "## Sequence Memory Usage")?;
    writeln!(&mut file)?;
    write_sequence_table::<Duration32>(&mut file, "u32", &capacities)?;
    write_sequence_table::<Duration64>(&mut file, "u64", &capacities)?;
    write_sequence_table::<EmbassyDuration>(&mut file, "Embassy", &capacities)?;

    // Sequencer tables
    writeln!(&mut file, "## Sequencer Memory Usage")?;
    writeln!(&mut file)?;
    writeln!(
        &mut file,
        "Sequencer overhead includes LED, state, time tracking, and the owned sequence."
    )?;
    writeln!(&mut file)?;

    writeln!(&mut file, "#### With u64 Instant/Duration")?;
    writeln!(&mut file)?;
    write_sequencer_table::<Instant64, SmallLed, TimeSource64>(
        &mut file,
        "u64",
        "Small",
        &capacities,
    )?;
    write_sequencer_table::<Instant64, MediumLed, TimeSource64>(
        &mut file,
        "u64",
        "Medium",
        &capacities,
    )?;
    write_sequencer_table::<Instant64, LargeLed, TimeSource64>(
        &mut file,
        "u64",
        "Large",
        &capacities,
    )?;

    writeln!(&mut file, "#### With Embassy Instant/Duration")?;
    writeln!(&mut file)?;
    write_sequencer_table::<EmbassyInstant, SmallLed, EmbassyTimeSource>(
        &mut file,
        "Embassy",
        "Small",
        &capacities,
    )?;
    write_sequencer_table::<EmbassyInstant, MediumLed, EmbassyTimeSource>(
        &mut file,
        "Embassy",
        "Medium",
        &capacities,
    )?;
    write_sequencer_table::<EmbassyInstant, LargeLed, EmbassyTimeSource>(
        &mut file,
        "Embassy",
        "Large",
        &capacities,
    )?;

    // Key insights
    writeln!(&mut file, "## Key Insights")?;
    writeln!(&mut file)?;
    writeln!(
        &mut file,
        "- Sequence overhead is constant regardless of capacity"
    )?;
    writeln!(
        &mut file,
        "- Sequencer adds fixed overhead for LED + state tracking"
    )?;
    writeln!(
        &mut file,
        "- LED implementation size directly affects total sequencer size"
    )?;
    writeln!(&mut file)?;
    writeln!(&mut file, "## Architecture Note")?;
    writeln!(&mut file)?;
    writeln!(
        &mut file,
        "Analysis performed on {}-bit host architecture. Embedded 32-bit targets will have slightly smaller sizes due to pointer differences (4B vs 8B). Step storage costs remain identical across architectures.",
        std::mem::size_of::<usize>() * 8
    )?;

    println!("✓ Report generated: {}", report_path);
    println!("  View with: cat {}", report_path);

    Ok(())
}
