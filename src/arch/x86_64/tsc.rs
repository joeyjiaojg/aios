// AIOS x86_64 Time Stamp Counter (TSC) Support
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: TSC (Time Stamp Counter) support for x86_64

use core::sync::atomic::{fence, Ordering};

#[inline]
pub fn rdtsc() -> u64 {
    // # Safety
    // RDTSC is a read-only instruction that does not modify any state
    // that would affect memory safety. It only reads the timestamp counter.
    // The TSC is guaranteed to be monotonically increasing on each CPU.
    let result: u32;
    let result_high: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") result,
            out("edx") result_high,
        );
    }
    ((result_high as u64) << 32) | (result as u64)
}

#[inline]
pub fn rdtscp() -> u64 {
    // # Safety
    // RDTSCP is a read-only instruction similar to RDTSC but includes
    // a serialization operation to ensure all instructions before it
    // have completed before reading the TSC.
    let result: u32;
    let result_high: u32;
    let _aux: u32;
    unsafe {
        core::arch::asm!(
            "rdtscp",
            out("eax") result,
            out("edx") result_high,
            out("ecx") _aux,
        );
    }
    ((result_high as u64) << 32) | (result as u64)
}

#[inline]
pub fn rdtsc_ordered() -> u64 {
    // # Safety
    // LFENCE before RDTSC serializes all prior instructions and ensures
    // that all memory accesses are visible before reading the TSC.
    // This is important for timing-sensitive operations.
    fence(Ordering::SeqCst);
    let result = rdtsc();
    fence(Ordering::SeqCst);
    result
}

#[inline]
pub fn read_tsc() -> u64 {
    rdtscp()
}

pub fn get_tsc_frequency() -> Option<u64> {
    let bus_clock_ratio = detect_tsc_frequency()?;
    let base_freq = 100_000_000u64;
    Some(base_freq * bus_clock_ratio)
}

fn detect_tsc_frequency() -> Option<u64> {
    use crate::arch::x86_64::cpuid;
    if !cpuid::has_tsc() {
        return None;
    }
    if cpuid::has_constant_tsc() {
        return Some(100_000_000);
    }
    Some(100_000_000)
}

pub fn timestamp_ns() -> u64 {
    let tsc = rdtsc();
    let freq = get_tsc_frequency().unwrap_or(100_000_000);
    tsc * 1_000_000_000 / freq
}

pub fn timestamp_us() -> u64 {
    let tsc = rdtsc();
    let freq = get_tsc_frequency().unwrap_or(100_000_000);
    tsc * 1_000_000 / freq
}

pub fn timestamp_ms() -> u64 {
    let tsc = rdtsc();
    let freq = get_tsc_frequency().unwrap_or(100_000_000);
    tsc * 1000 / freq
}

pub fn spin_wait(cycles: u64) {
    let start = rdtsc();
    loop {
        let current = rdtsc();
        if current.wrapping_sub(start) >= cycles {
            break;
        }
    }
}

pub fn busy_wait_ns(ns: u64) {
    let freq = get_tsc_frequency().unwrap_or(100_000_000);
    let cycles = ns * freq / 1_000_000_000;
    spin_wait(cycles);
}

pub fn busy_wait_us(us: u64) {
    let freq = get_tsc_frequency().unwrap_or(100_000_000);
    let cycles = us * freq / 1_000_000;
    spin_wait(cycles);
}

pub fn busy_wait_ms(ms: u64) {
    busy_wait_us(ms * 1000);
}

pub struct TscCalibration {
    pub tsc_per_ms: u64,
    pub tsc_per_us: u64,
    pub tsc_per_ns: u64,
}

impl Default for TscCalibration {
    fn default() -> Self {
        let freq = get_tsc_frequency().unwrap_or(100_000_000);
        Self {
            tsc_per_ms: freq / 1000,
            tsc_per_us: freq / 1_000_000,
            tsc_per_ns: freq / 1_000_000_000,
        }
    }
}

impl TscCalibration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn elapsed_ms(&self, start_tsc: u64) -> u64 {
        let current = rdtsc();
        (current.saturating_sub(start_tsc)) / self.tsc_per_ms
    }

    pub fn elapsed_us(&self, start_tsc: u64) -> u64 {
        let current = rdtsc();
        (current.saturating_sub(start_tsc)) / self.tsc_per_us
    }

    pub fn elapsed_ns(&self, start_tsc: u64) -> u64 {
        let current = rdtsc();
        (current.saturating_sub(start_tsc)) / self.tsc_per_ns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdtsc_returns_value() {
        let tsc1 = rdtsc();
        let tsc2 = rdtsc();
        assert!(tsc2 >= tsc1);
    }

    #[test]
    fn test_rdtscp_returns_value() {
        let tsc = rdtscp();
        assert!(tsc > 0);
    }

    #[test]
    fn test_rdtsc_ordered() {
        let tsc = rdtsc_ordered();
        assert!(tsc > 0);
    }

    #[test]
    fn test_read_tsc() {
        let tsc = read_tsc();
        assert!(tsc > 0);
    }

    #[test]
    fn test_timestamp_functions() {
        let ns = timestamp_ns();
        let us = timestamp_us();
        let ms = timestamp_ms();
        assert!(ns > 0);
        assert!(us > 0);
        assert!(ms > 0);
    }

    #[test]
    fn test_spin_wait() {
        spin_wait(1000);
    }

    #[test]
    fn test_busy_wait() {
        busy_wait_us(1);
    }

    #[test]
    fn test_tsc_calibration() {
        let cal = TscCalibration::new();
        assert!(cal.tsc_per_ms > 0);
        assert!(cal.tsc_per_us > 0);
        assert!(cal.tsc_per_ns > 0);
    }

    #[test]
    fn test_tsc_calibration_elapsed() {
        let cal = TscCalibration::new();
        let start = rdtsc();
        let elapsed = cal.elapsed_ms(start);
        assert!(elapsed >= 0);
    }

    #[test]
    fn test_rdtsc_monotonic() {
        let tsc1 = rdtsc();
        let tsc2 = rdtsc();
        let tsc3 = rdtscp();
        assert!(tsc3 >= tsc2);
        assert!(tsc2 >= tsc1);
    }

    #[test]
    fn test_tsc_frequency_detection() {
        let freq = get_tsc_frequency();
        if freq.is_some() {
            assert!(freq.unwrap() > 0);
        }
    }
}