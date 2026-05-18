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

/// djb2 hash for inode numbers
fn ramdisk_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 5381;
    for &b in data {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    if hash == 0 {
        1
    } else {
        hash
    }
}

/// Fill a getdents64 buffer with entries in dir_path starting at start_idx.
/// Returns (bytes_written, new_start_idx).
pub fn fill_getdents64(dir_path: &str, buf: &mut [u8], start_idx: usize) -> (usize, usize) {
    // Normalize dir_path with trailing slash
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

    // Collect (name_without_slash, is_dir) pairs
    // Use a heapless::Vec of fixed-size name buffers
    let mut entries: heapless::Vec<([u8; 64], usize, bool), 32> = heapless::Vec::new();
    let mut seen_dirs: heapless::Vec<[u8; 64], 16> = heapless::Vec::new();

    {
        let index = FILE_INDEX.lock();
        for entry in index.iter() {
            let path: &'static str = entry.path;
            let suffix: &str = if dir_prefix == "/" {
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
                // Entry inside a subdirectory — emit the subdir name once
                let subdir_name = &suffix[..slash]; // without slash
                let mut name_buf = [0u8; 64];
                let nlen = subdir_name.len().min(63);
                name_buf[..nlen].copy_from_slice(&subdir_name.as_bytes()[..nlen]);
                if !seen_dirs.iter().any(|d| d[..nlen] == name_buf[..nlen]) {
                    let _ = seen_dirs.push(name_buf);
                    let _ = entries.push((name_buf, nlen, true));
                }
            } else {
                let mut name_buf = [0u8; 64];
                let nlen = suffix.len().min(63);
                name_buf[..nlen].copy_from_slice(&suffix.as_bytes()[..nlen]);
                let _ = entries.push((name_buf, nlen, false));
            }
        }
    }

    let mut pos = 0usize;
    let mut entry_idx = start_idx;

    for (i, (name_buf, nlen, is_dir)) in entries.iter().enumerate() {
        if i < start_idx {
            continue;
        }
        let name_len = *nlen;
        // d_reclen = ((8+8+2+1 + name_len + 1) + 7) & !7
        let rec_len = ((19 + name_len + 1) + 7) & !7;
        if pos + rec_len > buf.len() {
            break;
        }
        // d_ino (u64, offset 0)
        let ino = ramdisk_hash(&name_buf[..name_len]);
        buf[pos..pos + 8].copy_from_slice(&ino.to_ne_bytes());
        // d_off (i64, offset 8)
        let d_off = (i as i64) + 1;
        buf[pos + 8..pos + 16].copy_from_slice(&d_off.to_ne_bytes());
        // d_reclen (u16, offset 16)
        let rec_len_u16 = rec_len as u16;
        buf[pos + 16..pos + 18].copy_from_slice(&rec_len_u16.to_ne_bytes());
        // d_type (u8, offset 18): 4 = dir, 8 = reg
        buf[pos + 18] = if *is_dir { 4 } else { 8 };
        // d_name (offset 19): name + null + padding
        buf[pos + 19..pos + 19 + name_len].copy_from_slice(&name_buf[..name_len]);
        buf[pos + 19 + name_len] = 0; // null terminator
                                      // zero padding
        for b in buf[pos + 19 + name_len + 1..pos + rec_len].iter_mut() {
            *b = 0;
        }
        pos += rec_len;
        entry_idx = i + 1;
    }

    (pos, entry_idx)
}

/// Read file data at byte offset into buf. Returns Some(bytes_read) or None if path not found.
pub fn read_file_at(path: &str, offset: usize, buf: &mut [u8]) -> Option<usize> {
    let data = lookup_file(path)?;
    if offset >= data.len() {
        return Some(0); // EOF
    }
    let available = data.len() - offset;
    let to_copy = available.min(buf.len());
    buf[..to_copy].copy_from_slice(&data[offset..offset + to_copy]);
    Some(to_copy)
}

/// Returns true if path exists as a file in the ramdisk.
pub fn path_exists(path: &str) -> bool {
    let index = FILE_INDEX.lock();
    index.iter().any(|e| e.path == path)
}

/// Returns true if path is a valid directory (any entry starts with path/).
pub fn is_valid_dir(path: &str) -> bool {
    if path == "/" {
        return true;
    }
    // Normalize: strip trailing slash then add one
    let mut norm_buf = [0u8; 256];
    let bytes = path.as_bytes();
    let trimmed = bytes
        .iter()
        .rposition(|&b| b != b'/')
        .map(|i| &bytes[..=i])
        .unwrap_or(bytes);
    let len = trimmed.len().min(254);
    norm_buf[..len].copy_from_slice(&trimmed[..len]);
    norm_buf[len] = b'/';
    let prefix = match core::str::from_utf8(&norm_buf[..=len]) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let index = FILE_INDEX.lock();
    index.iter().any(|e| e.path.starts_with(prefix))
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

    #[test]
    fn test_ramdisk_hash_nonzero() {
        let h = ramdisk_hash(b"busybox");
        assert!(h > 0);
    }

    #[test]
    fn test_ramdisk_hash_deterministic() {
        assert_eq!(ramdisk_hash(b"foo"), ramdisk_hash(b"foo"));
    }

    #[test]
    fn test_ramdisk_hash_different_inputs() {
        assert_ne!(ramdisk_hash(b"foo"), ramdisk_hash(b"bar"));
    }

    #[test]
    fn test_ramdisk_hash_empty_returns_one() {
        // djb2 of empty slice: starts at 5381, no iterations => 5381, not 0 => returns 5381
        let h = ramdisk_hash(b"");
        assert_eq!(h, 5381);
    }

    #[test]
    fn test_read_file_at_not_found() {
        // path that doesn't exist
        let mut buf = [0u8; 8];
        assert!(read_file_at("/nonexistent/file", 0, &mut buf).is_none());
    }

    #[test]
    fn test_path_exists_nonexistent() {
        assert!(!path_exists("/no/such/file"));
    }

    #[test]
    fn test_is_valid_dir_root() {
        assert!(is_valid_dir("/"));
    }

    #[test]
    fn test_is_valid_dir_nonexistent() {
        assert!(!is_valid_dir("/no/such/dir"));
    }

    #[test]
    fn test_fill_getdents64_empty_buf() {
        let mut buf = [];
        let (written, _) = fill_getdents64("/", &mut buf, 0);
        assert_eq!(written, 0);
    }

    #[test]
    fn test_fill_getdents64_small_buf() {
        // buf too small to hold any entry → written = 0
        let mut buf = [0u8; 4];
        let (written, _) = fill_getdents64("/", &mut buf, 0);
        assert_eq!(written, 0);
    }

    #[test]
    fn test_fill_getdents64_start_idx_beyond_entries() {
        let mut buf = [0u8; 512];
        // start_idx way beyond any entries → nothing written
        let (written, _) = fill_getdents64("/nonexistent_dir", &mut buf, 9999);
        assert_eq!(written, 0);
    }
}
