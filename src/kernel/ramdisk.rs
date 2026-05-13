// AIOS Ramdisk Filesystem
//
// Model: claude-sonnet-4-6
// Tool: claude-code
// Prompt: Refactor ramdisk to path→module mapping for busybox integration

use spin::Mutex;

// File index entry: maps path to module index or embedded data
enum FileSource {
    Module(usize),           // Index into multiboot2 modules
    Embedded(&'static [u8]), // Embedded data (e.g., USER_INIT_ELF)
}

struct FileEntry {
    path: &'static str,
    source: FileSource,
}

static FILE_INDEX: Mutex<heapless::Vec<FileEntry, 16>> = Mutex::new(heapless::Vec::new());

/// Initialize ramdisk file index from multiboot2 modules
pub fn init_from_modules() {
    if crate::debug::is_debug_enabled() {
        crate::serial::write_str("[ramdisk] initializing file index from modules\r\n");
    }

    let mut index = FILE_INDEX.lock();

    // Register /init as embedded USER_INIT_ELF (fallback)
    if index
        .push(FileEntry {
            path: "/init",
            source: FileSource::Embedded(crate::user_init::USER_INIT_ELF),
        })
        .is_err()
    {
        crate::serial::write_str("[ramdisk] ERROR: failed to register /init\r\n");
        return;
    }
    if crate::debug::is_debug_enabled() {
        crate::serial::write_str("[ramdisk] registered /init → embedded USER_INIT_ELF\r\n");
    }

    // Register modules from multiboot2
    // Module 0 is expected to be busybox with cmdline "busybox"
    if let Some(module) = crate::multiboot2::get_module_by_index(0) {
        if crate::debug::is_debug_enabled() {
            crate::serial::write_str("[ramdisk] found module 0: ");
            crate::serial::write_str(module.cmdline);
            crate::serial::write_str("\r\n");
        }

        // Register as /bin/busybox
        if index
            .push(FileEntry {
                path: "/bin/busybox",
                source: FileSource::Module(0),
            })
            .is_err()
        {
            crate::serial::write_str("[ramdisk] ERROR: file index full\r\n");
            return;
        }
        if crate::debug::is_debug_enabled() {
            crate::serial::write_str("[ramdisk] registered /bin/busybox → module 0\r\n");
        }

        // Also register as /bin/sh (symlink equivalent)
        if index
            .push(FileEntry {
                path: "/bin/sh",
                source: FileSource::Module(0),
            })
            .is_ok()
            && crate::debug::is_debug_enabled()
        {
            crate::serial::write_str("[ramdisk] registered /bin/sh → module 0\r\n");
        }
    } else if crate::debug::is_debug_enabled() {
        crate::serial::write_str("[ramdisk] WARNING: no modules found\r\n");
    }

    let count = index.len();
    if crate::debug::is_debug_enabled() {
        crate::serial::write_str("[ramdisk] initialized with ");
        print_decimal(count);
        crate::serial::write_str(" files\r\n");
    }
}

/// Lookup file by path, returns slice to file data (zero-copy)
pub fn lookup_file(path: &str) -> Option<&'static [u8]> {
    if crate::debug::is_debug_enabled() {
        crate::serial::write_str("[ramdisk] lookup: ");
        crate::serial::write_str(path);
        crate::serial::write_str("\r\n");
    }

    let index = FILE_INDEX.lock();
    for entry in index.iter() {
        if entry.path == path {
            if crate::debug::is_debug_enabled() {
                crate::serial::write_str("[ramdisk] found: ");
                crate::serial::write_str(entry.path);
                crate::serial::write_str("\r\n");
            }

            return match entry.source {
                FileSource::Module(idx) => {
                    if crate::debug::is_debug_enabled() {
                        crate::serial::write_str("[ramdisk] loading from module ");
                        print_decimal(idx);
                        crate::serial::write_str("\r\n");
                    }
                    crate::multiboot2::get_module_by_index(idx).map(|m| m.as_slice())
                }
                FileSource::Embedded(data) => {
                    if crate::debug::is_debug_enabled() {
                        crate::serial::write_str("[ramdisk] loading from embedded data\r\n");
                    }
                    Some(data)
                }
            };
        }
    }

    if crate::debug::is_debug_enabled() {
        crate::serial::write_str("[ramdisk] not found: ");
        crate::serial::write_str(path);
        crate::serial::write_str("\r\n");
    }
    None
}

/// List directory contents (for `ls` command)
pub fn list_dir(dir_path: &str) {
    // Normalize: root stays "/", others get trailing "/"
    let mut norm_buf = [0u8; 256];
    let dir_prefix: &str = if dir_path == "/" {
        "/"
    } else {
        let bytes = dir_path.as_bytes();
        let trimmed = bytes
            .iter()
            .rposition(|&b| b != b'/')
            .map(|i| &bytes[..=i])
            .unwrap_or(bytes);
        let len = trimmed.len().min(254);
        norm_buf[..len].copy_from_slice(&trimmed[..len]);
        norm_buf[len] = b'/';
        core::str::from_utf8(&norm_buf[..=len]).unwrap_or("/")
    };

    // Collect names to print (static strs — slices of 'static paths)
    let mut names: heapless::Vec<&'static str, 32> = heapless::Vec::new();
    let mut seen_dirs: heapless::Vec<&'static str, 16> = heapless::Vec::new();
    {
        let index = FILE_INDEX.lock();
        for entry in index.iter() {
            let path: &'static str = entry.path;
            let suffix: &'static str = if dir_prefix == "/" {
                if path.len() > 1 {
                    &path[1..]
                } else {
                    continue;
                }
            } else if let Some(s) = path.strip_prefix(dir_prefix) {
                s
            } else {
                continue;
            };
            if suffix.is_empty() {
                continue;
            }
            if let Some(slash) = suffix.find('/') {
                // Entry is inside a subdirectory — show the subdir name once
                let subdir: &'static str = &suffix[..=slash];
                if !seen_dirs.contains(&subdir) {
                    let _ = seen_dirs.push(subdir);
                    let _ = names.push(subdir);
                }
            } else {
                let _ = names.push(suffix);
            }
        }
    }

    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            crate::serial::write_str("  ");
        }
        crate::serial::write_str(name);
    }
    crate::serial::write_str("\r\n");
}

/// List all files in ramdisk (for debugging)
pub fn list_files() {
    crate::serial::write_str("[ramdisk] file index:\r\n");

    // Copy entries to stack array to avoid holding lock during I/O
    let mut entries_copy: heapless::Vec<(&'static str, bool, usize), 16> = heapless::Vec::new();
    {
        let index = FILE_INDEX.lock();
        for entry in index.iter() {
            let (is_module, idx) = match entry.source {
                FileSource::Module(i) => (true, i),
                FileSource::Embedded(_) => (false, 0),
            };
            let _ = entries_copy.push((entry.path, is_module, idx));
        }
    } // Lock released here

    // Now print without holding the lock
    for (path, is_module, idx) in entries_copy.iter() {
        crate::serial::write_str("  ");
        crate::serial::write_str(path);
        if *is_module {
            crate::serial::write_str(" → module ");
            print_decimal(*idx);
        } else {
            crate::serial::write_str(" → embedded");
        }
        crate::serial::write_str("\r\n");
    }
}

fn print_decimal(val: usize) {
    if val == 0 {
        crate::serial::write_byte(b'0');
        return;
    }

    let mut buf = [0u8; 20];
    let mut n = val;
    let mut i = 0;

    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        crate::serial::write_byte(buf[i]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_entry_size() {
        // Verify FileEntry fits in expected memory
        assert!(core::mem::size_of::<FileEntry>() <= 32);
    }

    #[test]
    fn test_file_source_size() {
        assert!(core::mem::size_of::<FileSource>() <= 16);
    }
}
