#!/usr/bin/env rust-script
//! Memory calculator for rgb-sequencer
//! 
//! This utility calculates and displays the exact memory footprint of RGB sequences
//! with different configurations. Run it to understand the memory implications of
//! your sequence capacity choices.
//!
//! Usage:
//!   cargo run --bin sequence_memory_calculator --features std
//!
//! Or with rust-script:
//!   rust-script sequence_memory_calculator.rs

use std::mem::size_of;
use rgb_sequencer::{RgbSequence, SequenceStep, TransitionStyle, LoopCount, TimeDuration};
use palette::Srgb;
use embassy_time::Duration as EmbassyDurationInner;

// Common duration types used in examples

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
            self.0.as_ticks().saturating_sub(other.0.as_ticks())
        ))
    }
}

fn print_header() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        RGB Sequencer Memory Footprint Calculator               ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
}

fn print_component_sizes() {
    println!("Component Sizes:");
    println!("├─ Srgb (f32 RGB):              {} bytes", size_of::<Srgb>());
    println!("├─ TransitionStyle (enum):      {} bytes", size_of::<TransitionStyle>());
    println!("├─ LoopCount (enum):             {} bytes", size_of::<LoopCount>());
    println!("├─ Option<Srgb>:                 {} bytes", size_of::<Option<Srgb>>());
    println!("├─ Color function pointer:       {} bytes", size_of::<Option<fn(Srgb, Duration64) -> Srgb>>());
    println!("└─ Timing function pointer:      {} bytes", size_of::<Option<fn(Duration64) -> Option<Duration64>>>());
    println!();
}

fn print_duration_sizes() {
    println!("Duration Type Sizes:");
    println!("├─ u32 (milliseconds):           {} bytes", size_of::<Duration32>());
    println!("├─ u64 (milliseconds):           {} bytes", size_of::<Duration64>());
    println!("└─ Embassy Duration (ticks):     {} bytes", size_of::<EmbassyDuration>());
    println!();
}

fn print_step_sizes() {
    println!("Step Sizes (by duration type):");
    println!("├─ SequenceStep<u32>:            {} bytes", size_of::<SequenceStep<Duration32>>());
    println!("├─ SequenceStep<u64>:            {} bytes", size_of::<SequenceStep<Duration64>>());
    println!("└─ SequenceStep<EmbassyDuration>: {} bytes", size_of::<SequenceStep<EmbassyDuration>>());
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
        
        println!("│ {:^8} │ {:>10} B │ {:>13} B │ {:>12} B │", 
                 capacity, 
                 total_size, 
                 storage_cost,
                 overhead);
    }
    
    println!("└──────────┴──────────────┴─────────────────┴────────────────┘");
    println!();
}

fn main() {
    print_header();
    print_component_sizes();
    print_duration_sizes();
    print_step_sizes();
    
    let capacities = vec![4, 8, 16, 32, 64];
    
    print_sequence_table::<Duration32>("u32", &capacities);
    print_sequence_table::<Duration64>("u64", &capacities);
    print_sequence_table::<EmbassyDuration>("EmbassyDuration", &capacities);
}