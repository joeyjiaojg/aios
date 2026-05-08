// AIOS ext2 Filesystem
//
// Model: opencode/minimax-m2.5-free
// Prompt: Implement ext2 filesystem support for AIOS x86_64 kernel in Rust no_std.

use spin::Mutex;

const EXT2_BLOCK_SIZE: usize = 1024;
const EXT2_MAX_DIR_ENTRIES: usize = 32;
const EXT2_MAX_NAME_LEN: usize = 255;
#[allow(dead_code)]
const EXT2_SUPERBLOCK_OFFSET: usize = 1024;
#[allow(dead_code)]
const EXT2_INODE_SIZE: usize = 128;
#[allow(dead_code)]
const EXT2_DIR_ENTRY_MIN_SIZE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ext2FileType {
    Unknown = 0,
    Regular = 1,
    Directory = 2,
    CharDevice = 3,
    BlockDevice = 4,
    Fifo = 5,
    Socket = 6,
    Symlink = 7,
}

#[derive(Debug, Clone, Copy)]
pub struct Ext2SuperBlock {
    pub inodes_count: u32,
    pub blocks_count: u32,
    pub r_blocks_count: u32,
    pub free_blocks_count: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub blocks_per_group: u32,
    pub inodes_per_group: u32,
    pub magic: u16,
    pub state: u16,
    pub errors: u16,
    pub minor_rev: u16,
    pub lastcheck: u32,
    pub checkinterval: u32,
    pub creator_os: u32,
    pub rev_level: u32,
    pub def_resuid: u16,
    pub def_resgid: u16,
    pub first_ino: u32,
    pub inode_size: u16,
    pub block_group_nr: u16,
    pub feature_compat: u32,
    pub feature_incompat: u32,
    pub feature_ro_compat: u32,
    pub uuid: [u8; 16],
    pub volume_name: [u8; 64],
    pub last_mounted: [u8; 64],
    pub algo_bitmap: u32,
    pub prealloc_blocks: u8,
    pub prealloc_dir_blocks: u8,
    pub padding: u16,
    pub journal_inum: u32,
    pub journal_dev: u32,
    pub last_orphan: u32,
    pub hash_seed: [u32; 4],
    pub htree_version: u8,
    pub padding2: u8,
    pub padding3: [u16; 2],
}

impl Default for Ext2SuperBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl Ext2SuperBlock {
    pub fn new() -> Self {
        Self {
            inodes_count: 128,
            blocks_count: 256,
            r_blocks_count: 0,
            free_blocks_count: 240,
            free_inodes_count: 120,
            first_data_block: 0,
            log_block_size: 0,
            blocks_per_group: 8192,
            inodes_per_group: 128,
            magic: 0xEF53,
            state: 1,
            errors: 1,
            minor_rev: 0,
            lastcheck: 0,
            checkinterval: 0,
            creator_os: 0,
            rev_level: 0,
            def_resuid: 0,
            def_resgid: 0,
            first_ino: 11,
            inode_size: 128,
            block_group_nr: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            uuid: [0u8; 16],
            volume_name: [0u8; 64],
            last_mounted: [0u8; 64],
            algo_bitmap: 0,
            prealloc_blocks: 0,
            prealloc_dir_blocks: 0,
            padding: 0,
            journal_inum: 0,
            journal_dev: 0,
            last_orphan: 0,
            hash_seed: [0u32; 4],
            htree_version: 0,
            padding2: 0,
            padding3: [0u16; 2],
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == 0xEF53
    }

    pub fn block_size(&self) -> usize {
        1024 << self.log_block_size
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ext2BlockGroupDescriptor {
    pub block_bitmap: u32,
    pub inode_bitmap: u32,
    pub inode_table: u32,
    pub free_blocks_count: u16,
    pub free_inodes_count: u16,
    pub used_dirs_count: u16,
    pub pad: u16,
    pub reserved: [u8; 12],
}

impl Default for Ext2BlockGroupDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

impl Ext2BlockGroupDescriptor {
    pub fn new() -> Self {
        Self {
            block_bitmap: 1,
            inode_bitmap: 2,
            inode_table: 3,
            free_blocks_count: 240,
            free_inodes_count: 120,
            used_dirs_count: 0,
            pad: 0,
            reserved: [0u8; 12],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ext2Inode {
    pub mode: u16,
    pub uid: u16,
    pub size: u32,
    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub dtime: u32,
    pub gid: u16,
    pub links_count: u16,
    pub blocks: u32,
    pub flags: u32,
    pub os_flags: u32,
    pub block: [u32; 15],
    pub generation: u32,
    pub file_acl: u32,
    pub dir_acl: u32,
    pub faddr: u32,
    pub os_bytes: [u8; 12],
}

impl Default for Ext2Inode {
    fn default() -> Self {
        Self::new()
    }
}

impl Ext2Inode {
    pub fn new() -> Self {
        Self {
            mode: 0,
            uid: 0,
            size: 0,
            atime: 0,
            ctime: 0,
            mtime: 0,
            dtime: 0,
            gid: 0,
            links_count: 0,
            blocks: 0,
            flags: 0,
            os_flags: 0,
            block: [0u32; 15],
            generation: 0,
            file_acl: 0,
            dir_acl: 0,
            faddr: 0,
            os_bytes: [0u8; 12],
        }
    }

    pub fn is_directory(&self) -> bool {
        (self.mode & 0x4000) != 0
    }

    pub fn is_regular_file(&self) -> bool {
        (self.mode & 0x8000) != 0
    }

    pub fn set_mode(&mut self, file_type: Ext2FileType) {
        match file_type {
            Ext2FileType::Regular => self.mode = 0x8000,
            Ext2FileType::Directory => self.mode = 0x4000,
            Ext2FileType::Symlink => self.mode = 0xA000,
            Ext2FileType::CharDevice => self.mode = 0x2000,
            Ext2FileType::BlockDevice => self.mode = 0x6000,
            _ => self.mode = 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ext2DirEntry {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    pub name: [u8; EXT2_MAX_NAME_LEN],
}

impl Default for Ext2DirEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl Ext2DirEntry {
    pub fn new() -> Self {
        Self {
            inode: 0,
            rec_len: 0,
            name_len: 0,
            file_type: 0,
            name: [0u8; EXT2_MAX_NAME_LEN],
        }
    }

    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(EXT2_MAX_NAME_LEN);
        self.name[..len].copy_from_slice(&bytes[..len]);
        self.name_len = len as u8;
    }
}

pub struct Ext2Filesystem {
    superblock: Ext2SuperBlock,
    block_groups: [Option<Ext2BlockGroupDescriptor>; 8],
    inode_bitmap: [u8; 128],
    block_bitmap: [u8; 128],
    inodes: [Option<Ext2Inode>; 128],
    data_blocks: [[u8; EXT2_BLOCK_SIZE]; 64],
    mounted: bool,
}

impl Default for Ext2Filesystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Ext2Filesystem {
    pub fn new() -> Self {
        let mut fs = Self {
            superblock: Ext2SuperBlock::new(),
            block_groups: [None; 8],
            inode_bitmap: [0u8; 128],
            block_bitmap: [0u8; 128],
            inodes: [None; 128],
            data_blocks: [[0u8; EXT2_BLOCK_SIZE]; 64],
            mounted: false,
        };
        fs.init();
        fs
    }

    fn init(&mut self) {
        self.block_groups[0] = Some(Ext2BlockGroupDescriptor::new());
        self.inode_bitmap[0] = 0x03;
        self.block_bitmap[0] = 0x07;
        self.mounted = true;

        let root_inode = self.allocate_inode();
        if let Some(ino) = root_inode {
            if let Some(inode) = self.inodes[ino as usize].as_mut() {
                inode.set_mode(Ext2FileType::Directory);
                inode.size = EXT2_BLOCK_SIZE as u32;
                inode.blocks = 1;
                inode.block[0] = 0;

                self.write_dir_entry(0, 2, ".", Ext2FileType::Directory);
                self.write_dir_entry(0, 2, "..", Ext2FileType::Directory);
            }
        }
    }

    fn allocate_inode(&mut self) -> Option<u32> {
        for i in 0..128 {
            let byte = self.inode_bitmap[i / 8];
            let bit = (byte >> (i % 8)) & 1;
            if bit == 0 {
                self.inode_bitmap[i / 8] |= 1 << (i % 8);
                self.inodes[i] = Some(Ext2Inode::new());
                return Some(i as u32 + 1);
            }
        }
        None
    }

    fn allocate_block(&mut self) -> Option<u32> {
        for i in 0..64 {
            let byte = self.block_bitmap[i / 8];
            let bit = (byte >> (i % 8)) & 1;
            if bit == 0 {
                self.block_bitmap[i / 8] |= 1 << (i % 8);
                return Some(i as u32);
            }
        }
        None
    }

    fn write_dir_entry(&mut self, block: u32, inode: u32, name: &str, file_type: Ext2FileType) {
        let block_idx = block as usize;
        if block_idx >= 64 {
            return;
        }

        let offset = if inode == 2 {
            0
        } else {
            let mut pos = 12;
            let mut found = false;
            while pos < EXT2_BLOCK_SIZE {
                let existing_inode = u32::from_le_bytes([
                    self.data_blocks[block_idx][pos],
                    self.data_blocks[block_idx][pos + 1],
                    self.data_blocks[block_idx][pos + 2],
                    self.data_blocks[block_idx][pos + 3],
                ]);
                if existing_inode == 0 {
                    found = true;
                    break;
                }
                let rec_len = u16::from_le_bytes([
                    self.data_blocks[block_idx][pos + 4],
                    self.data_blocks[block_idx][pos + 5],
                ]);
                pos += rec_len as usize;
            }
            if found {
                pos
            } else {
                0
            }
        };

        let name_bytes = name.as_bytes();
        let entry_size = ((8 + name_bytes.len() + 3) & !3) as u16;

        let base = block_idx * EXT2_BLOCK_SIZE + offset;
        self.data_blocks[block_idx][base..base + 4].copy_from_slice(&inode.to_le_bytes());
        self.data_blocks[block_idx][base + 4..base + 6].copy_from_slice(&entry_size.to_le_bytes());
        self.data_blocks[block_idx][base + 6] = name_bytes.len() as u8;
        self.data_blocks[block_idx][base + 7] = match file_type {
            Ext2FileType::Directory => 2,
            Ext2FileType::Regular => 1,
            _ => 0,
        };
        self.data_blocks[block_idx][base + 8..base + 8 + name_bytes.len()]
            .copy_from_slice(name_bytes);
    }

    pub fn mount(&mut self) -> bool {
        if !self.superblock.is_valid() {
            return false;
        }
        self.mounted = true;
        true
    }

    pub fn unmount(&mut self) -> bool {
        self.mounted = false;
        true
    }

    pub fn is_mounted(&self) -> bool {
        self.mounted
    }

    pub fn get_superblock(&self) -> &Ext2SuperBlock {
        &self.superblock
    }

    pub fn get_inode(&self, ino: u32) -> Option<Ext2Inode> {
        let idx = (ino - 1) as usize;
        if idx < 128 {
            self.inodes[idx].as_ref().copied()
        } else {
            None
        }
    }

    pub fn create_file(&mut self, parent_ino: u32, name: &str) -> Option<u32> {
        if !self.mounted {
            return None;
        }

        let ino = self.allocate_inode()?;
        if let Some(inode) = self.inodes[(ino - 1) as usize].as_mut() {
            inode.set_mode(Ext2FileType::Regular);
            inode.size = 0;
            inode.blocks = 0;
        }

        if let Some(parent) = self.get_inode(parent_ino) {
            if parent.is_directory() {
                self.write_dir_entry(parent.block[0], ino, name, Ext2FileType::Regular);
            }
        }

        Some(ino)
    }

    pub fn create_directory(&mut self, parent_ino: u32, name: &str) -> Option<u32> {
        if !self.mounted {
            return None;
        }

        let ino = self.allocate_inode()?;
        let block = self.allocate_block()?;

        if let Some(inode) = self.inodes[(ino - 1) as usize].as_mut() {
            inode.set_mode(Ext2FileType::Directory);
            inode.size = EXT2_BLOCK_SIZE as u32;
            inode.blocks = 1;
            inode.block[0] = block;

            self.write_dir_entry(block, ino, ".", Ext2FileType::Directory);
            self.write_dir_entry(block, 2, "..", Ext2FileType::Directory);
        }

        if let Some(parent) = self.get_inode(parent_ino) {
            if parent.is_directory() {
                self.write_dir_entry(parent.block[0], ino, name, Ext2FileType::Directory);
            }
        }

        Some(ino)
    }

    pub fn read_data(&self, ino: u32, offset: u32, buf: &mut [u8]) -> Option<usize> {
        if !self.mounted {
            return None;
        }

        let inode = self.get_inode(ino)?;
        if !inode.is_regular_file() && !inode.is_directory() {
            return None;
        }

        let block_num = offset as usize / EXT2_BLOCK_SIZE;
        if block_num >= 15 {
            return None;
        }

        let block_num = inode.block[block_num] as usize;
        if block_num >= 64 {
            return None;
        }

        let block_offset = (offset as usize) % EXT2_BLOCK_SIZE;
        let available = EXT2_BLOCK_SIZE - block_offset;
        let to_read = buf.len().min(available);

        buf[..to_read]
            .copy_from_slice(&self.data_blocks[block_num][block_offset..block_offset + to_read]);
        Some(to_read)
    }

    pub fn write_data(&mut self, ino: u32, offset: u32, data: &[u8]) -> Option<usize> {
        if !self.mounted {
            return None;
        }

        let idx = (ino - 1) as usize;
        if idx >= 128 {
            return None;
        }

        let block_num = offset as usize / EXT2_BLOCK_SIZE;
        if block_num >= 15 {
            return None;
        }

        let block_idx_option = {
            let inode = self.inodes[idx].as_ref()?;
            if inode.block[block_num] == 0 {
                None
            } else {
                Some(inode.block[block_num] as usize)
            }
        };

        let block_idx = match block_idx_option {
            Some(b) => b,
            None => {
                let new_block = self.allocate_block()?;
                if let Some(inode) = self.inodes[idx].as_mut() {
                    inode.block[block_num] = new_block;
                    inode.blocks += 1;
                }
                new_block as usize
            }
        };

        if block_idx >= 64 {
            return None;
        }

        let block_offset = (offset as usize) % EXT2_BLOCK_SIZE;
        let available = EXT2_BLOCK_SIZE - block_offset;
        let to_write = data.len().min(available);

        self.data_blocks[block_idx][block_offset..block_offset + to_write]
            .copy_from_slice(&data[..to_write]);

        if let Some(inode) = self.inodes[idx].as_mut() {
            let new_size = offset + to_write as u32;
            if new_size > inode.size {
                inode.size = new_size;
            }
        }

        Some(to_write)
    }

    pub fn read_directory(&self, ino: u32) -> Option<[(u32, &'static str); EXT2_MAX_DIR_ENTRIES]> {
        let inode = self.get_inode(ino)?;
        if !inode.is_directory() {
            return None;
        }

        let block = inode.block[0] as usize;
        if block >= 64 {
            return None;
        }

        let mut entries: [(u32, &'static str); EXT2_MAX_DIR_ENTRIES] =
            [(0, ""); EXT2_MAX_DIR_ENTRIES];
        let mut entry_count = 0;
        let mut pos = 0;

        while pos < EXT2_BLOCK_SIZE && entry_count < EXT2_MAX_DIR_ENTRIES {
            let entry_inode = u32::from_le_bytes([
                self.data_blocks[block][pos],
                self.data_blocks[block][pos + 1],
                self.data_blocks[block][pos + 2],
                self.data_blocks[block][pos + 3],
            ]);

            if entry_inode == 0 {
                break;
            }

            let rec_len = u16::from_le_bytes([
                self.data_blocks[block][pos + 4],
                self.data_blocks[block][pos + 5],
            ]);

            let name_len = self.data_blocks[block][pos + 6] as usize;
            let file_type = self.data_blocks[block][pos + 7];

            if name_len > 0 && name_len < EXT2_MAX_NAME_LEN {
                let type_str = match file_type {
                    1 => "file",
                    2 => "dir",
                    7 => "symlink",
                    _ => "unknown",
                };
                entries[entry_count] = (entry_inode, type_str);
                entry_count += 1;
            }

            pos += rec_len as usize;
            if rec_len == 0 {
                break;
            }
        }

        Some(entries)
    }

    pub fn find_entry(&self, parent_ino: u32, name: &str) -> Option<u32> {
        let inode = self.get_inode(parent_ino)?;
        if !inode.is_directory() {
            return None;
        }

        let block = inode.block[0] as usize;
        if block >= 64 {
            return None;
        }

        let mut pos = 0;

        while pos < EXT2_BLOCK_SIZE {
            let entry_inode = u32::from_le_bytes([
                self.data_blocks[block][pos],
                self.data_blocks[block][pos + 1],
                self.data_blocks[block][pos + 2],
                self.data_blocks[block][pos + 3],
            ]);

            if entry_inode == 0 {
                break;
            }

            let rec_len = u16::from_le_bytes([
                self.data_blocks[block][pos + 4],
                self.data_blocks[block][pos + 5],
            ]);

            let entry_name_len = self.data_blocks[block][pos + 6] as usize;

            if entry_name_len > 0 && entry_name_len < EXT2_MAX_NAME_LEN {
                let entry_name_start = pos + 8;
                let entry_name_end = entry_name_start + entry_name_len;
                if entry_name_end <= EXT2_BLOCK_SIZE {
                    let entry_name = core::str::from_utf8(
                        &self.data_blocks[block][entry_name_start..entry_name_end],
                    )
                    .unwrap_or("");

                    if entry_name == name {
                        return Some(entry_inode);
                    }
                }
            }

            pos += rec_len as usize;
            if rec_len == 0 {
                break;
            }
        }

        None
    }
}

static EXT2_FS: Mutex<Option<Ext2Filesystem>> = Mutex::new(None);

fn get_ext2_fs() -> &'static Mutex<Option<Ext2Filesystem>> {
    let mut fs = EXT2_FS.lock();
    if fs.is_none() {
        *fs = Some(Ext2Filesystem::new());
    }
    &EXT2_FS
}

pub fn ext2_mount() -> bool {
    get_ext2_fs();
    if let Some(ref mut fs) = *EXT2_FS.lock() {
        fs.mount()
    } else {
        false
    }
}

pub fn ext2_unmount() -> bool {
    if let Some(ref mut fs) = *EXT2_FS.lock() {
        fs.unmount()
    } else {
        false
    }
}

pub fn ext2_is_mounted() -> bool {
    if let Some(ref fs) = *EXT2_FS.lock() {
        fs.is_mounted()
    } else {
        false
    }
}

pub fn ext2_create_file(parent_ino: u32, name: &str) -> Option<u32> {
    get_ext2_fs();
    if let Some(ref mut fs) = *EXT2_FS.lock() {
        fs.create_file(parent_ino, name)
    } else {
        None
    }
}

pub fn ext2_create_directory(parent_ino: u32, name: &str) -> Option<u32> {
    get_ext2_fs();
    if let Some(ref mut fs) = *EXT2_FS.lock() {
        fs.create_directory(parent_ino, name)
    } else {
        None
    }
}

pub fn ext2_read_data(ino: u32, offset: u32, buf: &mut [u8]) -> Option<usize> {
    if let Some(ref fs) = *EXT2_FS.lock() {
        fs.read_data(ino, offset, buf)
    } else {
        None
    }
}

pub fn ext2_write_data(ino: u32, offset: u32, data: &[u8]) -> Option<usize> {
    if let Some(ref mut fs) = *EXT2_FS.lock() {
        fs.write_data(ino, offset, data)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_superblock_new() {
        let sb = Ext2SuperBlock::new();
        assert_eq!(sb.magic, 0xEF53);
    }

    #[test]
    fn test_superblock_is_valid() {
        let sb = Ext2SuperBlock::new();
        assert!(sb.is_valid());
    }

    #[test]
    fn test_superblock_block_size() {
        let sb = Ext2SuperBlock::new();
        assert_eq!(sb.block_size(), 1024);
    }

    #[test]
    fn test_block_group_descriptor_new() {
        let bgd = Ext2BlockGroupDescriptor::new();
        assert_eq!(bgd.block_bitmap, 1);
        assert_eq!(bgd.inode_bitmap, 2);
        assert_eq!(bgd.inode_table, 3);
    }

    #[test]
    fn test_inode_new() {
        let inode = Ext2Inode::new();
        assert_eq!(inode.mode, 0);
        assert_eq!(inode.size, 0);
    }

    #[test]
    fn test_inode_is_directory() {
        let mut inode = Ext2Inode::new();
        inode.set_mode(Ext2FileType::Directory);
        assert!(inode.is_directory());
    }

    #[test]
    fn test_inode_is_regular_file() {
        let mut inode = Ext2Inode::new();
        inode.set_mode(Ext2FileType::Regular);
        assert!(inode.is_regular_file());
    }

    #[test]
    fn test_dir_entry_new() {
        let entry = Ext2DirEntry::new();
        assert_eq!(entry.inode, 0);
        assert_eq!(entry.name_len, 0);
    }

    #[test]
    fn test_filesystem_new() {
        let fs = Ext2Filesystem::new();
        assert!(fs.is_mounted());
    }

    #[test]
    fn test_filesystem_mount_unmount() {
        let mut fs = Ext2Filesystem::new();
        assert!(fs.mount());
        assert!(fs.is_mounted());
        assert!(fs.unmount());
        assert!(!fs.is_mounted());
    }

    #[test]
    fn test_filesystem_create_file() {
        let mut fs = Ext2Filesystem::new();
        let ino = fs.create_file(2, "test.txt");
        assert!(ino.is_some());
    }

    #[test]
    fn test_filesystem_create_directory() {
        let mut fs = Ext2Filesystem::new();
        let ino = fs.create_directory(2, "testdir");
        assert!(ino.is_some());
    }

    #[test]
    fn test_filesystem_read_write_data() {
        let mut fs = Ext2Filesystem::new();
        let ino = fs.create_file(2, "data.bin").unwrap();

        let write_data = b"Hello, ext2!";
        let written = fs.write_data(ino, 0, write_data);
        assert_eq!(written, Some(write_data.len()));

        let mut read_buf = [0u8; 32];
        let read = fs.read_data(ino, 0, &mut read_buf);
        assert_eq!(read, Some(write_data.len()));
        assert_eq!(&read_buf[..write_data.len()], write_data);
    }

    #[test]
    fn test_filesystem_read_directory() {
        let fs = Ext2Filesystem::new();
        let entries = fs.read_directory(2);
        assert!(entries.is_some());
    }

    #[test]
    fn test_global_ext2_mount() {
        get_ext2_fs();
        assert!(ext2_is_mounted());
    }

    #[test]
    fn test_global_ext2_create() {
        get_ext2_fs();
        let ino = ext2_create_file(2, "global.txt");
        assert!(ino.is_some());
    }
}
