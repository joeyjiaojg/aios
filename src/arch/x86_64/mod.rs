// AIOS x86_64 Architecture Support
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Fill empty src/arch/x86_64/mod.rs stub with x86_64 architecture utilities

pub mod cpuid;
pub mod msr;
pub mod tsc;

pub const VIRTUAL_MEMORY_LAYOUT: VirtualMemoryLayout = VirtualMemoryLayout {
    kernel_start: 0xFFFF_8000_0000_0000,
    kernel_end: 0xFFFF_8000_FFFF_FFFF,
    user_start: 0x0000_0000_0000_0000,
    user_end: 0x0000_7FFF_FFFF_FFFF,
};

pub struct VirtualMemoryLayout {
    pub kernel_start: u64,
    pub kernel_end: u64,
    pub user_start: u64,
    pub user_end: u64,
}

pub fn is_in_kernel_memory(addr: u64) -> bool {
    addr >= VIRTUAL_MEMORY_LAYOUT.kernel_start && addr <= VIRTUAL_MEMORY_LAYOUT.kernel_end
}

pub fn is_in_user_memory(addr: u64) -> bool {
    addr >= VIRTUAL_MEMORY_LAYOUT.user_start && addr <= VIRTUAL_MEMORY_LAYOUT.user_end
}

pub fn is_canonical_address(addr: u64) -> bool {
    let sign_extended = (addr as i64) as u64;
    (addr >> 47) == (sign_extended >> 47)
}

pub fn canonicalize(addr: u64) -> u64 {
    if is_canonical_address(addr) {
        addr
    } else {
        let high_bit = (addr >> 47) & 1;
        if high_bit == 0 {
            addr | 0xFFFF_8000_0000_0000
        } else {
            addr & 0x0000_7FFF_FFFF_FFFF
        }
    }
}

pub const fn page_size() -> u64 {
    4096
}

pub const fn huge_page_size() -> u64 {
    2 * 1024 * 1024
}

pub const fn is_aligned(addr: u64, align: u64) -> bool {
    (addr & (align - 1)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_memory_layout_constants() {
        assert_eq!(VIRTUAL_MEMORY_LAYOUT.kernel_start, 0xFFFF_8000_0000_0000);
        assert_eq!(VIRTUAL_MEMORY_LAYOUT.kernel_end, 0xFFFF_8000_FFFF_FFFF);
        assert_eq!(VIRTUAL_MEMORY_LAYOUT.user_start, 0x0000_0000_0000_0000);
        assert_eq!(VIRTUAL_MEMORY_LAYOUT.user_end, 0x0000_7FFF_FFFF_FFFF);
    }

    #[test]
    fn test_is_in_kernel_memory() {
        let kernel_addr = 0xFFFF_8000_0000_1000;
        let user_addr = 0x0000_0000_0000_1000;
        assert!(is_in_kernel_memory(kernel_addr));
        assert!(!is_in_kernel_memory(user_addr));
    }

    #[test]
    fn test_is_in_user_memory() {
        let kernel_addr = 0xFFFF_8000_0000_1000;
        let user_addr = 0x0000_0000_0000_1000;
        assert!(is_in_user_memory(user_addr));
        assert!(!is_in_user_memory(kernel_addr));
    }

    #[test]
    fn test_is_canonical_address() {
        assert!(is_canonical_address(0x0000_0000_0000_1000));
        assert!(is_canonical_address(0xFFFF_8000_0000_1000));
        assert!(!is_canonical_address(0x0000_8000_0000_1000));
        assert!(!is_canonical_address(0xFFFF_7FFF_FFFF_FFFF));
    }

    #[test]
    fn test_canonicalize_valid() {
        let addr = 0xFFFF_8000_0000_1000;
        assert_eq!(canonicalize(addr), addr);
    }

    #[test]
    fn test_page_size() {
        assert_eq!(page_size(), 4096);
    }

    #[test]
    fn test_huge_page_size() {
        assert_eq!(huge_page_size(), 2 * 1024 * 1024);
    }

    #[test]
    fn test_is_aligned() {
        let addr = 0x1000;
        assert!(is_aligned(addr, 4096));
        assert!(!is_aligned(addr, 8192));
        assert!(is_aligned(0, 4096));
        assert!(is_aligned(0x1000, 1));
    }

    #[test]
    fn test_page_alignment() {
        assert!(is_aligned(0x1000, page_size()));
        assert!(!is_aligned(0x1001, page_size()));
    }
}