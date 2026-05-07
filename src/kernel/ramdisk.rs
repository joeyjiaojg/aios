// AIOS Ramdisk Filesystem
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement ramdisk filesystem for AIOS x86_64 kernel in Rust no_std.

const RAMDISK_SIZE: usize = 65536;
#[allow(dead_code)]
const BLOCK_SIZE: usize = 512;

use core::cmp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileType {
    #[default]
    Regular = 1,
    Directory = 2,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Inode {
    pub ino: u32,
    pub file_type: FileType,
    pub size: u32,
}

impl Inode {
    pub fn new(ino: u32) -> Self {
        Self {
            ino,
            file_type: FileType::Regular,
            size: 0,
        }
    }

    pub fn is_dir(&self) -> bool {
        self.file_type == FileType::Directory
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DirEntry {
    pub ino: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Ramdisk {
    pub data: [u8; RAMDISK_SIZE],
}

impl Default for Ramdisk {
    fn default() -> Self {
        Self {
            data: [0u8; RAMDISK_SIZE],
        }
    }
}

impl Ramdisk {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        // Initialize ramdisk with a simple filesystem structure
        // For now, we'll just zero out the data (already done by Default)
        // In a real implementation, we might set up a root directory, etc.
    }

    /// Read data from the ramdisk
    pub fn read(&self, ino: u32, _offset: u32, buf: &mut [u8]) -> Option<usize> {
        // Simple implementation: treat ino as block number for demonstration
        // In a real filesystem, we'd look up the inode to find data blocks
        let block_index = ino as usize;
        let block_start = block_index * BLOCK_SIZE;

        // Check bounds
        if block_start >= RAMDISK_SIZE {
            return None;
        }

        let available = RAMDISK_SIZE - block_start;
        let to_copy = cmp::min(buf.len(), available);

        if to_copy == 0 {
            return Some(0);
        }

        let end = cmp::min(block_start + to_copy, RAMDISK_SIZE);
        buf[..to_copy].copy_from_slice(&self.data[block_start..end]);
        Some(to_copy)
    }

    /// Write data to the ramdisk
    pub fn write(&mut self, ino: u32, _offset: u32, data: &[u8]) -> Option<usize> {
        // Simple implementation: treat ino as block number for demonstration
        let block_index = ino as usize;
        let block_start = block_index * BLOCK_SIZE;

        // Check bounds
        if block_start >= RAMDISK_SIZE {
            return None;
        }

        let available = RAMDISK_SIZE - block_start;
        let to_copy = cmp::min(data.len(), available);

        if to_copy == 0 {
            return Some(0);
        }

        let end = cmp::min(block_start + to_copy, RAMDISK_SIZE);
        self.data[block_start..end].copy_from_slice(&data[..to_copy]);
        Some(to_copy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ramdisk_new() {
        let rd = Ramdisk::new();
        assert_eq!(rd.data[0], 0);
    }

    #[test]
    fn test_init() {
        let mut rd = Ramdisk::new();
        rd.init();
    }

    #[test]
    fn test_inode_new() {
        let inode = Inode::new(5);
        assert_eq!(inode.ino, 5);
    }

    #[test]
    fn test_inode_is_dir() {
        let mut inode = Inode::new(1);
        inode.file_type = FileType::Directory;
        assert!(inode.is_dir());
    }

    #[test]
    fn test_file_type() {
        assert_eq!(FileType::Regular as u8, 1);
    }

    #[test]
    fn test_ramdisk_size() {
        assert_eq!(RAMDISK_SIZE, 65536);
    }

    #[test]
    fn test_block_size() {
        assert_eq!(BLOCK_SIZE, 512);
    }

    #[test]
    fn test_init_fn() {
        let mut ramdisk = Ramdisk::new();
        ramdisk.init();
    }

    #[test]
    fn test_read_write_basic() {
        let mut ramdisk = Ramdisk::new();
        let write_data = b"Hello, Ramdisk!";
        let mut read_buf = [0u8; 32];

        // Write data to block 0
        let write_result = ramdisk.write(0, 0, write_data);
        assert_eq!(write_result, Some(write_data.len()));

        // Read data back from block 0
        let read_result = ramdisk.read(0, 0, &mut read_buf);
        assert_eq!(read_result, Some(write_data.len()));
        assert_eq!(&read_buf[..write_data.len()], write_data);
    }

    #[test]
    fn test_read_write_offset() {
        let mut ramdisk = Ramdisk::new();
        let write_data = b"Offset test";
        let mut read_buf = [0u8; 16];

        // Write data with offset
        let write_result = ramdisk.write(0, 5, write_data);
        assert_eq!(write_result, Some(write_data.len()));

        // Read data with matching offset
        let read_result = ramdisk.read(0, 5, &mut read_buf);
        assert_eq!(read_result, Some(write_data.len()));
        assert_eq!(&read_buf[..write_data.len()], write_data);

        // Read data without offset should get zeros
        let mut zero_buf = [0u8; 10];
        let zero_result = ramdisk.read(0, 0, &mut zero_buf);
        assert_eq!(zero_result, Some(5)); // First 5 bytes should be zero
        assert_eq!(&zero_buf[..5], &[0u8; 5]);
    }

    #[test]
    fn test_read_write_bounds() {
        let mut ramdisk = Ramdisk::new();
        let data = [0u8; 100];

        // Write at valid position
        let result = ramdisk.write(0, 0, &data);
        assert_eq!(result, Some(data.len()));

        // Write at exactly the end should succeed with 0 bytes
        let result = ramdisk.write(0, RAMDISK_SIZE as u32, &data);
        assert_eq!(result, Some(0));

        // Write past the end should fail
        let result = ramdisk.write(0, (RAMDISK_SIZE + 1) as u32, &data);
        assert_eq!(result, None);

        // Write that goes past end should truncate
        let result = ramdisk.write(0, (RAMDISK_SIZE - 50) as u32, &[0u8; 100]);
        assert_eq!(result, Some(50)); // Only 50 bytes fit

        // Read bounds checking
        let mut buf = [0u8; 10];
        let result = ramdisk.read(0, RAMDISK_SIZE as u32, &mut buf);
        assert_eq!(result, Some(0));

        let result = ramdisk.read(0, (RAMDISK_SIZE + 5) as u32, &mut buf);
        assert_eq!(result, None);
    }

    #[test]
    fn test_read() {
        let ramdisk = Ramdisk::new();
        let mut buf = [0u8; 512];
        assert!(ramdisk.read(1, 0, &mut buf).is_none());
    }

    #[test]
    fn test_write() {
        let ramdisk = Ramdisk::new();
        let data = &[0u8; 512];
        assert!(ramdisk.write(1, 0, data).is_none());
    }

    #[test]
    fn test_direntry() {
        let entry = DirEntry::default();
        assert_eq!(entry.ino, 0);
    }
}
