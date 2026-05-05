// AIOS Virtual File System
//
// Model: opencode
// Tool: opencode
// Prompt: Create VFS framework for AIOS x86_64 kernel in Rust no_std
//         with VfsNode, VfsManager, mount table, and thread safety.

use spin::Mutex;

/// Maximum number of mount points
pub const MAX_MOUNT_POINTS: usize = 8;

/// Maximum path length
pub const MAX_PATH_LENGTH: usize = 256;

/// VfsNode type enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VfsNodeType {
    File,
    Dir,
    Symlink,
}

impl VfsNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VfsNodeType::File => "file",
            VfsNodeType::Dir => "dir",
            VfsNodeType::Symlink => "symlink",
        }
    }
}

/// VFS node structure
#[derive(Debug)]
pub struct VfsNode {
    pub id: u64,
    pub name: [u8; 64],
    pub node_type: VfsNodeType,
    pub size: u64,
    pub mount_point: u8,
}

impl VfsNode {
    /// Create a new VfsNode
    pub const fn new(id: u64, name: [u8; 64], node_type: VfsNodeType) -> Self {
        VfsNode {
            id,
            name,
            node_type,
            size: 0,
            mount_point: 0,
        }
    }

    /// Create a file node
    pub fn new_file(id: u64, name: &str) -> Self {
        let mut name_arr = [0u8; 64];
        let bytes = name.as_bytes();
        let len = bytes.len().min(64);
        // SAFETY: Copying bytes into a fixed-size array, bounds checked above
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                name_arr.as_mut_ptr(),
                len,
            );
        }
        VfsNode::new(id, name_arr, VfsNodeType::File)
    }

    /// Create a directory node
    pub fn new_dir(id: u64, name: &str) -> Self {
        let mut name_arr = [0u8; 64];
        let bytes = name.as_bytes();
        let len = bytes.len().min(64);
        // SAFETY: Copying bytes into a fixed-size array, bounds checked above
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                name_arr.as_mut_ptr(),
                len,
            );
        }
        VfsNode::new(id, name_arr, VfsNodeType::Dir)
    }

    /// Create a symlink node
    pub fn new_symlink(id: u64, name: &str, target: &str) -> Self {
        let mut name_arr = [0u8; 64];
        let bytes = name.as_bytes();
        let len = bytes.len().min(64);
        // SAFETY: Copying bytes into a fixed-size array, bounds checked above
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                name_arr.as_mut_ptr(),
                len,
            );
        }
        let mut node = VfsNode::new(id, name_arr, VfsNodeType::Symlink);
        node.size = target.len() as u64;
        node
    }

    /// Get the node name as a string slice
    pub fn name(&self) -> &str {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(64);
        core::str::from_utf8(&self.name[..len]).unwrap_or("")
    }

    /// Set the node name from a string
    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(64);
        // SAFETY: Writing into fixed-size array, bounds checked above
        unsafe {
            core::ptr::write_bytes(self.name.as_mut_ptr(), 0, 64);
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.name.as_mut_ptr(),
                len,
            );
        }
    }
}

/// Mount table entry
#[derive(Debug)]
pub struct MountEntry {
    pub path: [u8; 64],
    pub node_id: u64,
    pub mounted: bool,
}

impl MountEntry {
    /// Create a new empty mount entry
    pub const fn new() -> Self {
        MountEntry {
            path: [0u8; 64],
            node_id: 0,
            mounted: false,
        }
    }

    /// Check if this entry is active
    pub const fn is_active(&self) -> bool {
        self.mounted
    }
}

/// VFS Manager with mount table
pub struct VfsManager {
    nodes: [Option<VfsNode>; 256],
    node_count: usize,
    mount_table: [MountEntry; MAX_MOUNT_POINTS],
    mount_count: usize,
    next_node_id: u64,
}

impl VfsManager {
    /// Create a new VfsManager
    pub const fn new() -> Self {
        VfsManager {
            nodes: [None; 256],
            node_count: 0,
            mount_table: [MountEntry::new(); MAX_MOUNT_POINTS],
            mount_count: 0,
            next_node_id: 1,
        }
    }

    /// Allocate a new node ID
    fn allocate_node_id(&mut self) -> u64 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        id
    }

    /// Add a node to the VFS
    pub fn add_node(&mut self, node: VfsNode) -> Option<u64> {
        if self.node_count >= 256 {
            return None;
        }
        let id = node.id;
        self.nodes[self.node_count] = Some(node);
        self.node_count += 1;
        Some(id)
    }

    /// Mount a filesystem at a path
    ///
    /// # Safety
    /// Caller must ensure the mount point is valid and not already mounted
    pub unsafe fn mount(&mut self, path: &str, node_id: u64) -> Result<(), &'static str> {
        if self.mount_count >= MAX_MOUNT_POINTS {
            return Err("Mount table full");
        }

        // Check if path is already mounted
        for i in 0..self.mount_count {
            if self.mount_table[i].is_active() {
                let entry_path_len = self.mount_table[i].path.iter()
                    .position(|&b| b == 0)
                    .unwrap_or(64);
                let entry_path = &self.mount_table[i].path[..entry_path_len];
                if entry_path == bytes {
                    return Err("Path already mounted");
                }
            }
        }

        // Find empty slot
        let mut slot = None;
        for i in 0..MAX_MOUNT_POINTS {
            if !self.mount_table[i].is_active() {
                slot = Some(i);
                break;
            }
        }

        if let Some(idx) = slot {
            let bytes = path.as_bytes();
            let len = bytes.len().min(64);
            // SAFETY: Writing into fixed-size array, bounds checked
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.mount_table[idx].path.as_mut_ptr(),
                len,
            );
            self.mount_table[idx].node_id = node_id;
            self.mount_table[idx].mounted = true;
            self.mount_count += 1;
            Ok(())
        } else {
            Err("No available mount slots")
        }
    }

    /// Unmount a filesystem at a path
    pub fn unmount(&mut self, path: &str) -> Result<u64, &'static str> {
        let bytes = path.as_bytes();
        for i in 0..MAX_MOUNT_POINTS {
            if self.mount_table[i].is_active() {
                let entry_path_len = self.mount_table[i].path.iter()
                    .position(|&b| b == 0)
                    .unwrap_or(64);
                let entry_path = &self.mount_table[i].path[..entry_path_len];
                if entry_path == bytes {
                    let node_id = self.mount_table[i].node_id;
                    self.mount_table[i].mounted = false;
                    // SAFETY: Zeroing the path array
                    unsafe {
                        core::ptr::write_bytes(
                            self.mount_table[i].path.as_mut_ptr(),
                            0,
                            64,
                        );
                    }
                    self.mount_count -= 1;
                    return Ok(node_id);
                }
            }
        }
        Err("Mount point not found")
    }

    /// Lookup a node by path
    pub fn lookup(&self, path: &str) -> Option<&VfsNode> {
        let bytes = path.as_bytes();

        // Check mount table first
        for i in 0..MAX_MOUNT_POINTS {
            if self.mount_table[i].is_active() {
                let entry_path_len = self.mount_table[i].path.iter()
                    .position(|&b| b == 0)
                    .unwrap_or(64);
                let entry_path = &self.mount_table[i].path[..entry_path_len];
                if entry_path == bytes {
                    let node_id = self.mount_table[i].node_id;
                    return self.get_node_by_id(node_id);
                }
            }
        }

        // Search all nodes
        for i in 0..self.node_count {
            if let Some(ref node) = self.nodes[i] {
                let node_name = node.name();
                if node_name.as_bytes() == bytes {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Get a node by ID
    fn get_node_by_id(&self, id: u64) -> Option<&VfsNode> {
        for i in 0..self.node_count {
            if let Some(ref node) = self.nodes[i] {
                if node.id == id {
                    return Some(node);
                }
            }
        }
        None
    }

    /// Read from a node
    ///
    /// # Safety
    /// Caller must ensure the buffer is valid for writing
    pub unsafe fn read(&self, path: &str, _offset: u64, _buf: &mut [u8]) -> Result<u64, &'static str> {
        let node = self.lookup(path).ok_or("Node not found")?;

        if node.node_type != VfsNodeType::File {
            return Err("Not a file");
        }

        // Return current size (no actual file reading in this implementation)
        Ok(node.size)
    }

    /// Write to a node
    ///
    /// # Safety
    /// Caller must ensure the buffer is valid for reading
    pub unsafe fn write(&mut self, path: &str, _offset: u64, data: &[u8]) -> Result<u64, &'static str> {
        let node = self.lookup(path).ok_or("Node not found")?;

        if node.node_type != VfsNodeType::File {
            return Err("Not a file");
        }

        // Update size (no actual file writing in this implementation)
        let written = data.len() as u64;

        // Find and update the node
        for i in 0..self.node_count {
            if let Some(ref mut n) = self.nodes[i] {
                if n.id == node.id {
                    n.size = written;
                    break;
                }
            }
        }

        Ok(written)
    }

    /// Get the number of mounted filesystems
    pub fn mount_count(&self) -> usize {
        self.mount_count
    }

    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.node_count
    }
}

/// Global VFS Manager (protected by mutex)
pub static VFS_MANAGER: Mutex<VfsManager> = Mutex::new(VfsManager::new());

/// Initialize the VFS
pub fn init() {
    let mut guard = VFS_MANAGER.lock();
    let vfs = guard.as_mut();

    // Create root directory
    let root = VfsNode::new_dir(1, "/");
    let _ = vfs.add_node(root);

    // Create /dev directory
    let dev = VfsNode::new_dir(2, "dev");
    let _ = vfs.add_node(dev);

    // Create /tmp directory
    let tmp = VfsNode::new_dir(3, "tmp");
    let _ = vfs.add_node(tmp);
}

/// Mount a filesystem
pub fn mount(path: &str, node_id: u64) -> Result<(), &'static str> {
    let mut guard = VFS_MANAGER.lock();
    // SAFETY: Validating mount parameters
    unsafe { guard.as_mut().mount(path, node_id) }
}

/// Unmount a filesystem
pub fn unmount(path: &str) -> Result<u64, &'static str> {
    let mut guard = VFS_MANAGER.lock();
    guard.as_mut().unmount(path)
}

/// Lookup a node by path
pub fn lookup(path: &str) -> Option<VfsNode> {
    let guard = VFS_MANAGER.lock();
    guard.lookup(path).map(|n| *n)
}

/// Read from a node
pub fn read(path: &str, offset: u64, buf: &mut [u8]) -> Result<u64, &'static str> {
    let guard = VFS_MANAGER.lock();
    // SAFETY: Validating buffer and path
    unsafe { guard.as_mut().read(path, offset, buf) }
}

/// Write to a node
pub fn write(path: &str, offset: u64, data: &[u8]) -> Result<u64, &'static str> {
    let mut guard = VFS_MANAGER.lock();
    // SAFETY: Validating data buffer
    unsafe { guard.as_mut().write(path, offset, data) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs_node_creation() {
        let file = VfsNode::new_file(1, "test.txt");
        assert_eq!(file.name(), "test.txt");
        assert_eq!(file.node_type, VfsNodeType::File);
    }

    #[test]
    fn test_vfs_manager_new() {
        let vfs = VfsManager::new();
        assert_eq!(vfs.node_count(), 0);
        assert_eq!(vfs.mount_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut vfs = VfsManager::new();
        let file = VfsNode::new_file(1, "test.txt");
        let id = vfs.add_node(file);
        assert!(id.is_some());
        assert_eq!(vfs.node_count(), 1);
    }

    #[test]
    fn test_lookup() {
        let mut vfs = VfsManager::new();
        let file = VfsNode::new_file(1, "test.txt");
        let _ = vfs.add_node(file);

        let found = vfs.lookup("test.txt");
        assert!(found.is_some());
    }

    #[test]
    fn test_mount_unmount() {
        let mut vfs = VfsManager::new();
        let file = VfsNode::new_file(1, "test.txt");
        let _ = vfs.add_node(file);

        // Mount the file
        // SAFETY: Testing mount functionality with test data
        unsafe {
            let result = vfs.mount("/mnt", 1);
            assert!(result.is_ok());
        }

        assert_eq!(vfs.mount_count(), 1);

        // Unmount
        let result = vfs.unmount("/mnt");
        assert!(result.is_ok());
        assert_eq!(vfs.mount_count(), 0);
    }

    #[test]
    fn test_read_write() {
        let mut vfs = VfsManager::new();
        let file = VfsNode::new_file(1, "test.txt");
        let _ = vfs.add_node(file);

        // Write data
        let data = b"Hello, VFS!";
        // SAFETY: Testing write with valid data
        unsafe {
            let result = vfs.write("test.txt", 0, data);
            assert!(result.is_ok());
        }

        // Read data
        let mut buf = [0u8; 64];
        // SAFETY: Testing read with valid buffer
        unsafe {
            let result = vfs.read("test.txt", 0, &mut buf);
            assert!(result.is_ok());
        }
    }
}