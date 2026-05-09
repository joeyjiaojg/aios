// AIOS x86_64 CPUID Support
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: CPUID instruction support for x86_64

#[derive(Debug, Clone, Copy)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

impl CpuidResult {
    pub const fn new(eax: u32, ebx: u32, ecx: u32, edx: u32) -> Self {
        Self { eax, ebx, ecx, edx }
    }
}

#[inline]
pub fn cpuid(function: u32) -> CpuidResult {
    // # Safety
    // CPUID is a read-only instruction that does not modify any processor state
    // that would affect memory safety. It only reads CPU identification and
    // feature information.
    let result: (u32, u32, u32, u32);
    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") function => result.0,
            out("ebx") result.1,
            out("ecx") result.2,
            out("edx") result.3,
        );
    }
    CpuidResult::new(result.0, result.1, result.2, result.3)
}

#[inline]
pub fn cpuid_extended(function: u32, subfunction: u32) -> CpuidResult {
    // # Safety
    // CPUID with extended input is also read-only, same safety rationale as cpuid().
    let result: (u32, u32, u32, u32);
    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") function => result.0,
            inout("ecx") subfunction => result.2,
            out("ebx") result.1,
            out("edx") result.3,
        );
    }
    CpuidResult::new(result.0, result.1, result.2, result.3)
}

pub fn get_vendor_id() -> [u8; 12] {
    let result = cpuid(0);
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&result.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&result.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&result.ecx.to_le_bytes());
    vendor
}

pub fn get_max_basic_function() -> u32 {
    cpuid(0).eax
}

pub fn has_feature(leaf: u32, register: u32, bit: u8) -> bool {
    let result = cpuid(leaf);
    let value = match register {
        0 => result.eax,
        1 => result.ebx,
        2 => result.ecx,
        3 => result.edx,
        _ => return false,
    };
    (value & (1 << bit)) != 0
}

pub fn has_long_mode() -> bool {
    let result = cpuid(0x8000_0001);
    (result.edx & (1 << 29)) != 0
}

pub fn has_apic() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 9)) != 0
}

pub fn has_x2apic() -> bool {
    let result = cpuid(1);
    (result.ecx & (1 << 21)) != 0
}

pub fn has_sse() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 25)) != 0
}

pub fn has_sse2() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 26)) != 0
}

pub fn has_sse3() -> bool {
    let result = cpuid(1);
    (result.ecx & (1 << 0)) != 0
}

pub fn has_ssse3() -> bool {
    let result = cpuid(1);
    (result.ecx & (1 << 9)) != 0
}

pub fn has_sse41() -> bool {
    let result = cpuid(1);
    (result.ecx & (1 << 19)) != 0
}

pub fn has_sse42() -> bool {
    let result = cpuid(1);
    (result.ecx & (1 << 20)) != 0
}

pub fn has_mmx() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 23)) != 0
}

pub fn has_fpu() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 0)) != 0
}

pub fn has_pge() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 13)) != 0
}

pub fn has_pae() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 6)) != 0
}

pub fn has_cmpxchg8b() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 8)) != 0
}

pub fn has_syscall() -> bool {
    let result = cpuid(0x8000_0001);
    (result.edx & (1 << 11)) != 0
}

pub fn has_msr() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 5)) != 0
}

pub fn has_rdrand() -> bool {
    let result = cpuid(1);
    (result.ecx & (1 << 30)) != 0
}

pub fn has_rdseed() -> bool {
    let result = cpuid(0x8000_0008);
    (result.ebx & (1 << 18)) != 0
}

pub fn has_tsc() -> bool {
    let result = cpuid(1);
    (result.edx & (1 << 4)) != 0
}

pub fn has_constant_tsc() -> bool {
    let result = cpuid(0x8000_0007);
    (result.edx & (1 << 8)) != 0
}

pub fn has_invariance_tsc() -> bool {
    let result = cpuid(0x8000_0007);
    (result.edx & (1 << 8)) != 0
}

pub fn get_processor_brand_string() -> Option<[u8; 48]> {
    if cpuid(0x8000_0000).eax < 0x8000_0004 {
        return None;
    }
    let mut brand = [0u8; 48];
    let r1 = cpuid(0x8000_0002);
    let r2 = cpuid(0x8000_0003);
    let r3 = cpuid(0x8000_0004);
    brand[0..16].copy_from_slice(&r1.to_le_bytes());
    brand[16..32].copy_from_slice(&r2.to_le_bytes());
    brand[32..48].copy_from_slice(&r3.to_le_bytes());
    Some(brand)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpuid_basic() {
        let result = cpuid(0);
        assert!(result.eax >= 0);
        assert!(result.ebx != 0 || result.ecx != 0 || result.edx != 0);
    }

    #[test]
    fn test_vendor_id_length() {
        let vendor = get_vendor_id();
        assert_eq!(vendor.len(), 12);
    }

    #[test]
    fn test_max_basic_function() {
        let max_fn = get_max_basic_function();
        assert!(max_fn >= 1);
    }

    #[test]
    fn test_has_feature_register_bounds() {
        assert!(!has_feature(1, 4, 0));
    }

    #[test]
    fn test_has_msr() {
        let result = has_msr();
        let _ = result;
    }

    #[test]
    fn test_has_tsc() {
        let result = has_tsc();
        let _ = result;
    }

    #[test]
    fn test_has_long_mode() {
        let result = has_long_mode();
        let _ = result;
    }

    #[test]
    fn test_has_fpu() {
        let result = has_fpu();
        let _ = result;
    }

    #[test]
    fn test_cpuid_result_new() {
        let r = CpuidResult::new(1, 2, 3, 4);
        assert_eq!(r.eax, 1);
        assert_eq!(r.ebx, 2);
        assert_eq!(r.ecx, 3);
        assert_eq!(r.edx, 4);
    }

    #[test]
    fn test_cpuid_extended() {
        let result = cpuid_extended(0x8000_0000, 0);
        assert!(result.eax >= 0x8000_0000);
    }
}