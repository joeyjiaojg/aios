// AIOS x86_64 Model Specific Register (MSR) Support
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: MSR read/write support for x86_64

pub const MSR_EFER: u32 = 0xC000_0080;
pub const MSR_STAR: u32 = 0xC000_0081;
pub const MSR_LSTAR: u32 = 0xC000_0082;
pub const MSR_CSTAR: u32 = 0xC000_0083;
pub const MSR_SFMASK: u32 = 0xC000_0084;
pub const MSR_FS_BASE: u32 = 0xC000_0100;
pub const MSR_GS_BASE: u32 = 0xC000_0101;
pub const MSR_KERNEL_GS_BASE: u32 = 0xC000_0102;
pub const MSR_TSC_AUX: u32 = 0xC000_0103;

pub const EFER_SCE: u64 = 1 << 0;
pub const EFER_LME: u64 = 1 << 8;
pub const EFER_LMA: u64 = 1 << 10;
pub const EFER_NXE: u64 = 1 << 11;
pub const EFER_SVME: u64 = 1 << 12;
pub const EFER_LMSLE: u64 = 1 << 13;
pub const EFER_FFXSR: u64 = 1 << 14;

#[inline]
pub fn read_msr(msr: u32) -> u64 {
    // # Safety
    // Reading an MSR is safe when the MSR number is valid for the CPU.
    // The MSR addresses used here are standard x86_64 MSRs that are documented
    // in the AMD64 Architecture Programmer's Manual and Intel SDM.
    // Invalid MSR reads will cause a #GP fault, not memory corruption.
    let result: u64;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") result as u32,
            out("edx") (result >> 32) as u32,
        );
    }
    result
}

#[inline]
pub fn write_msr(msr: u32, value: u64) {
    // # Safety
    // Writing to an MSR is safe when:
    // 1. The MSR number is valid for the CPU
    // 2. The value written is within the valid range for that MSR
    // 3. The CPU is in a state where writing to that MSR is allowed
    //
    // The MSR addresses used here are standard x86_64 MSRs. Writing invalid
    // values may cause a #GP fault but not memory corruption.
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") value as u32,
            in("edx") (value >> 32) as u32,
        );
    }
}

pub fn read_efer() -> u64 {
    read_msr(MSR_EFER)
}

pub fn write_efer(value: u64) {
    write_msr(MSR_EFER, value);
}

pub fn read_star() -> u64 {
    read_msr(MSR_STAR)
}

pub fn write_star(value: u64) {
    write_msr(MSR_STAR, value);
}

pub fn read_lstar() -> u64 {
    read_msr(MSR_LSTAR)
}

pub fn write_lstar(value: u64) {
    write_msr(MSR_LSTAR, value);
}

pub fn read_cstar() -> u64 {
    read_msr(MSR_CSTAR)
}

pub fn write_cstar(value: u64) {
    write_msr(MSR_CSTAR, value);
}

pub fn read_fsmask() -> u64 {
    read_msr(MSR_SFMASK)
}

pub fn write_fsmask(value: u64) {
    write_msr(MSR_SFMASK, value);
}

pub fn read_fs_base() -> u64 {
    read_msr(MSR_FS_BASE)
}

pub fn write_fs_base(value: u64) {
    write_msr(MSR_FS_BASE, value);
}

pub fn read_gs_base() -> u64 {
    read_msr(MSR_GS_BASE)
}

pub fn write_gs_base(value: u64) {
    write_msr(MSR_GS_BASE, value);
}

pub fn read_kernel_gs_base() -> u64 {
    read_msr(MSR_KERNEL_GS_BASE)
}

pub fn write_kernel_gs_base(value: u64) {
    write_msr(MSR_KERNEL_GS_BASE, value);
}

pub fn read_tsc_aux() -> u64 {
    read_msr(MSR_TSC_AUX)
}

pub fn write_tsc_aux(value: u64) {
    write_msr(MSR_TSC_AUX, value);
}

pub fn enable_syscall() {
    let mut efer = read_efer();
    efer |= EFER_SCE;
    write_efer(efer);
}

pub fn enable_long_mode() {
    let mut efer = read_efer();
    efer |= EFER_LME;
    write_efer(efer);
}

pub fn enable_nxe() {
    let mut efer = read_efer();
    efer |= EFER_NXE;
    write_efer(efer);
}

pub fn enable_ia32e_mode() {
    let mut efer = read_efer();
    efer |= EFER_LME | EFER_LMA;
    write_efer(efer);
}

pub fn is_long_mode_active() -> bool {
    (read_efer() & EFER_LMA) != 0
}

pub fn is_nxe_enabled() -> bool {
    (read_efer() & EFER_NXE) != 0
}

pub fn is_syscall_enabled() -> bool {
    (read_efer() & EFER_SCE) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msr_constants() {
        assert_eq!(MSR_EFER, 0xC000_0080);
        assert_eq!(MSR_STAR, 0xC000_0081);
        assert_eq!(MSR_LSTAR, 0xC000_0082);
    }

    #[test]
    fn test_efer_constants() {
        assert_eq!(EFER_SCE, 1 << 0);
        assert_eq!(EFER_LME, 1 << 8);
        assert_eq!(EFER_LMA, 1 << 10);
        assert_eq!(EFER_NXE, 1 << 11);
    }

    #[test]
    fn test_read_efer() {
        let value = read_efer();
        assert!(value != 0);
    }

    #[test]
    fn test_read_write_msr_roundtrip() {
        let original = read_msr(MSR_FS_BASE);
        let test_value = original ^ 0x1234_5678_9ABC_DEF0;
        write_msr(MSR_FS_BASE, test_value);
        let read_back = read_msr(MSR_FS_BASE);
        assert_eq!(read_back, test_value);
        write_msr(MSR_FS_BASE, original);
    }

    #[test]
    fn test_fs_base_roundtrip() {
        let original = read_fs_base();
        let test_value = 0x1_0000;
        write_fs_base(test_value);
        let read_back = read_fs_base();
        assert_eq!(read_back, test_value);
        write_fs_base(original);
    }

    #[test]
    fn test_gs_base_roundtrip() {
        let original = read_gs_base();
        let test_value = 0x2_0000;
        write_gs_base(test_value);
        let read_back = read_gs_base();
        assert_eq!(read_back, test_value);
        write_gs_base(original);
    }

    #[test]
    fn test_kernel_gs_base_roundtrip() {
        let original = read_kernel_gs_base();
        let test_value = 0x3_0000;
        write_kernel_gs_base(test_value);
        let read_back = read_kernel_gs_base();
        assert_eq!(read_back, test_value);
        write_kernel_gs_base(original);
    }

    #[test]
    fn test_star_roundtrip() {
        let original = read_star();
        let test_value = 0x0011_2233_4455_6677;
        write_star(test_value);
        let read_back = read_star();
        assert_eq!(read_back, test_value);
        write_star(original);
    }

    #[test]
    fn test_read_lstar() {
        let value = read_lstar();
        let _ = value;
    }

    #[test]
    fn test_read_fsmask() {
        let value = read_fsmask();
        let _ = value;
    }

    #[test]
    fn test_is_long_mode_active() {
        let _ = is_long_mode_active();
    }

    #[test]
    fn test_is_nxe_enabled() {
        let _ = is_nxe_enabled();
    }

    #[test]
    fn test_is_syscall_enabled() {
        let _ = is_syscall_enabled();
    }
}