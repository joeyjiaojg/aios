// AIOS Multiboot2 Module Parser
//
// Model: claude-sonnet-4-6
// Tool: claude-code
// Prompt: Parse multiboot2 info structure to extract modules loaded by GRUB

use core::slice;
use core::str;
use spin::Mutex;

const MODULE_TAG_TYPE: u32 = 3;
const END_TAG_TYPE: u32 = 0;

#[repr(C)]
struct MultibootTag {
    typ: u32,
    size: u32,
}

#[repr(C)]
struct MultibootModuleTag {
    typ: u32,
    size: u32,
    mod_start: u32,
    mod_end: u32,
    // cmdline follows as null-terminated string
}

pub struct ModuleInfo {
    pub start: *const u8,
    pub size: usize,
    pub cmdline: &'static str,
}

// # Safety
// ModuleInfo contains a raw pointer (*const u8) to module memory, but:
// 1. The pointer is never dereferenced in safe code
// 2. Access is only via as_slice() which returns &[u8] with proper bounds
// 3. Module memory is loaded by bootloader and remains valid for kernel lifetime
unsafe impl Send for ModuleInfo {}
unsafe impl Sync for ModuleInfo {}

static MODULES: Mutex<heapless::Vec<ModuleInfo, 8>> = Mutex::new(heapless::Vec::new());

impl ModuleInfo {
    pub fn as_slice(&self) -> &'static [u8] {
        // # Safety
        // start pointer and size come from multiboot2 module tag, which points to
        // bootloader-loaded module memory (identity-mapped in first 1GB). The memory
        // remains valid for the kernel lifetime and cannot be freed. Slice bounds are
        // checked by bootloader when it loaded the module.
        unsafe { slice::from_raw_parts(self.start, self.size) }
    }
}

/// Parse multiboot2 info structure and extract module information.
///
/// # Safety
/// `mbi_ptr` must point to a valid multiboot2 info structure placed by GRUB.
/// The structure must be complete and readable. GRUB guarantees this at boot time.
pub unsafe fn parse_modules(mbi_ptr: *const u8) {
    crate::serial::write_str("[multiboot2] parsing modules...\r\n");

    // # Safety
    // mbi_ptr is passed from boot.S and points to the multiboot2 info structure
    // placed by GRUB at a valid memory address. Reading the first 8 bytes (total_size
    // and reserved fields) is safe as GRUB guarantees the structure is complete.
    let total_size = unsafe { *(mbi_ptr as *const u32) };
    crate::serial::write_str("[multiboot2] total_size = ");
    print_hex_u32(total_size);
    crate::serial::write_str("\r\n");

    let mut offset: usize = 8; // Skip total_size and reserved fields

    while offset < total_size as usize {
        // # Safety
        // offset is bounds-checked against total_size. The tag pointer is computed
        // from mbi_ptr + offset, which is within the multiboot2 info structure.
        let tag_ptr = unsafe { mbi_ptr.add(offset) as *const MultibootTag };
        let tag = unsafe { &*tag_ptr };

        crate::serial::write_str("[multiboot2] tag type=");
        print_hex_u32(tag.typ);
        crate::serial::write_str(" size=");
        print_hex_u32(tag.size);
        crate::serial::write_str("\r\n");

        if tag.typ == END_TAG_TYPE {
            crate::serial::write_str("[multiboot2] end tag reached\r\n");
            break;
        }

        if tag.typ == MODULE_TAG_TYPE {
            // # Safety
            // tag_ptr is valid (checked above). Casting to MultibootModuleTag is safe
            // because MODULE_TAG_TYPE guarantees the tag has the module structure.
            let module_tag = unsafe { &*(tag_ptr as *const MultibootModuleTag) };
            let mod_start = module_tag.mod_start as usize;
            let mod_end = module_tag.mod_end as usize;
            let mod_size = mod_end - mod_start;

            // Extract cmdline (null-terminated string after mod_end field)
            let cmdline_ptr =
                unsafe { (tag_ptr as *const u8).add(core::mem::size_of::<MultibootModuleTag>()) };
            let cmdline = unsafe {
                let mut len = 0;
                while *cmdline_ptr.add(len) != 0 && len < 256 {
                    len += 1;
                }
                let bytes = slice::from_raw_parts(cmdline_ptr, len);
                str::from_utf8_unchecked(bytes)
            };

            crate::serial::write_str("[multiboot2] found module: ");
            crate::serial::write_str(cmdline);
            crate::serial::write_str(" (start=0x");
            print_hex_u32(mod_start as u32);
            crate::serial::write_str(", size=");
            print_hex_u32(mod_size as u32);
            crate::serial::write_str(")\r\n");

            let info = ModuleInfo {
                start: mod_start as *const u8,
                size: mod_size,
                cmdline,
            };

            let mut modules = MODULES.lock();
            if modules.push(info).is_err() {
                crate::serial::write_str("[multiboot2] WARNING: module limit reached\r\n");
            }
        }

        // Align to 8-byte boundary
        offset += ((tag.size + 7) & !7) as usize;
    }

    let count = MODULES.lock().len();
    crate::serial::write_str("[multiboot2] found ");
    print_hex_u32(count as u32);
    crate::serial::write_str(" modules\r\n");
}

pub fn get_module_by_name(name: &str) -> Option<&'static ModuleInfo> {
    let modules = MODULES.lock();
    for module in modules.iter() {
        if module.cmdline.contains(name) {
            // # Safety
            // Module references are stored in a static Mutex with kernel lifetime.
            // The reference is valid as long as the kernel runs (modules are never removed).
            // Transmuting the lifetime to 'static is safe because the data is static.
            return Some(unsafe {
                core::mem::transmute::<&ModuleInfo, &'static ModuleInfo>(module)
            });
        }
    }
    None
}

pub fn get_module_by_index(index: usize) -> Option<&'static ModuleInfo> {
    let modules = MODULES.lock();
    if let Some(module) = modules.get(index) {
        // # Safety
        // See get_module_by_name() for lifetime transmute rationale
        return Some(unsafe { core::mem::transmute::<&ModuleInfo, &'static ModuleInfo>(module) });
    }
    None
}

fn print_hex_u32(val: u32) {
    let hex_chars = b"0123456789abcdef";
    for i in (0..8).rev() {
        let nibble = ((val >> (i * 4)) & 0xF) as usize;
        crate::serial::write_byte(hex_chars[nibble]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_tag_size() {
        assert_eq!(core::mem::size_of::<MultibootModuleTag>(), 16);
    }

    #[test]
    fn test_module_info_size() {
        // Verify ModuleInfo fits in expected memory
        assert!(core::mem::size_of::<ModuleInfo>() <= 32);
    }
}
