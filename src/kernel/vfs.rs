// AIOS Virtual Filesystem
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create VFS framework for AIOS x86_64 kernel in Rust no_std with VfsNode,
//         VfsManager with 256 nodes and 8 mount points, thread safety with spin::Mutex

#![no_std]

use spin::Mutex;

/// Maximum filename length (null-terminated string)
const MAX_NAME_LEN: usize = 64;
/// Maximum number of VFS nodes
const MAX_NODES: usize = 256;
/// Maximum number of mount points
const MAX_MOUNTS: usize = 8;
/// Maximum data size per node (4KB)
const MAX_DATA_SIZE: usize = 4096;

/// VFS node types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VfsNodeType {
    File,
    Directory,
    Symlink,
}

/// VFS node structure
#[derive(Debug, Clone)]
pub struct VfsNode {
    id: u64,
    name: [u8; MAX_NAME_LEN],
    node_type: VfsNodeType,
    size: u64,
    data: [u8; MAX_DATA_SIZE],
    data_size: usize,
    parent_id: Option<u64>,
    is_mounted: bool,
}

impl VfsNode {
    /// Create a new VFS node
    pub fn new(id: u64, name: &str, node_type: VfsNodeType) -> Self {
        let mut name_arr = [0u8; MAX_NAME_LEN];
        let bytes = name.as_bytes();
        let len = bytes.len().min(MAX_NAME_LEN - 1);
        name_arr[..len].copy_from_slice(&bytes[..len]);
        
        Self {
            id,
            name: name_arr,
            node_type,
            size: 0,
            data: [0u8; MAX_DATA_SIZE],
            data_size: 0,
            parent_id: None,
            is_mounted: false,
        }
    }

    /// Get node ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get node name as a string
    pub fn name(&self) -> &str {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(MAX_NAME_LEN);
        core::str::from_utf8(&self.name[..len]).unwrap_or("")
    }

    /// Get node type
    pub fn node_type(&self) -> VfsNodeType {
        self.node_type
    }

    /// Get node size
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get data size
    pub fn data_size(&self) -> usize {
        self.data_size
    }

    /// Set parent node ID
    pub fn set_parent(&mut self, parent: Option<u64>) {
        self.parent_id = parent;
    }

    /// Get parent node ID
    pub fn parent_id(&self) -> Option<u64> {
        self.parent_id
    }

    /// Mark node as mounted
    pub fn set_mounted(&mut self, mounted: bool) {
        self.is_mounted = mounted;
    }

    /// Check if node is a mount point
    pub fn is_mounted(&self) -> bool {
        self.is_mounted
    }
}

/// Mount point structure
#[derive(Debug, Clone)]
struct MountPoint {
    source: u64,
    target: u64,
    flags: u32,
}

impl MountPoint {
    /// Create a new mount point
    pub fn new(source: u64, target: u64, flags: u32) -> Self {
        Self { source, target, flags }
    }
}

/// VFS manager with node and mount management
pub struct VfsManager {
    nodes: [Option<VfsNode>; MAX_NODES],
    mounts: [Option<MountPoint>; MAX_MOUNTS],
    next_node_id: u64,
    node_count: usize,
    mount_count: usize,
}

impl VfsManager {
    /// Create a new VFS manager
    pub fn new() -> Self {
        let mut manager = Self {
            nodes: [None; MAX_NODES],
            mounts: [None; MAX_MOUNTS],
            next_node_id: 2,  // Start from 2 because 1 is root
            node_count: 1,
            mount_count: 0,
        };
        
        // Create root directory "/"
        manager.nodes[0] = Some(VfsNode::new(1, "/", VfsNodeType::Directory));
        
        manager
    }

    /// Allocate a new node slot
    fn alloc_node(&mut self) -> Option<usize> {
        for i in 0..MAX_NODES {
            if self.nodes[i].is_none() {
                return Some(i);
            }
        }
        None
    }

    /// Allocate a new mount point
    fn alloc_mount(&mut self) -> Option<usize> {
        for i in 0..MAX_MOUNTS {
            if self.mounts[i].is_none() {
                return Some(i);
            }
        }
        None
    }

    /// Mount a source node to a target mount point
    pub fn mount(&mut self, source: u64, target: u64, flags: u32) -> Result<(), VfsError> {
        if self.mount_count >= MAX_MOUNTS {
            return Err(VfsError::MountTableFull);
        }

        let source_idx = self.find_node_index(source);
        let target_idx = self.find_node_index(target);

        if source_idx.is_none() {
            return Err(VfsError::NodeNotFound);
        }
        if target_idx.is_none() {
            return Err(VfsError::NodeNotFound);
        }

        let t_idx = target_idx.unwrap();
        if let Some(ref node) = self.nodes[t_idx] {
            if node.node_type() != VfsNodeType::Directory {
                return Err(VfsError::NotADirectory);
            }
            if node.is_mounted() {
                return Err(VfsError::AlreadyMounted);
            }
        }

        let mount_idx = self.alloc_mount().ok_or(VfsError::MountTableFull)?;
        self.mounts[mount_idx] = Some(MountPoint::new(source, target, flags));
        self.mount_count += 1;

        if let Some(ref mut node) = self.nodes[t_idx] {
            node.set_mounted(true);
        }

        Ok(())
    }

    /// Unmount a target mount point
    pub fn unmount(&mut self, target: u64) -> Result<(), VfsError> {
        for i in 0..MAX_MOUNTS {
            if let Some(ref mp) = self.mounts[i] {
                if mp.target == target {
                    let target_idx = self.find_node_index(target);
                    if let Some(idx) = target_idx {
                        if let Some(ref mut node) = self.nodes[idx] {
                            node.set_mounted(false);
                        }
                    }
                    self.mounts[i] = None;
                    self.mount_count -= 1;
                    return Ok(());
                }
            }
        }
        Err(VfsError::NotMounted)
    }

    /// Find node index by ID
    fn find_node_index(&self, id: u64) -> Option<usize> {
        for i in 0..MAX_NODES {
            if let Some(ref node) = self.nodes[i] {
                if node.id() == id {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Lookup a node by path
    pub fn lookup(&self, path: &[u8]) -> Result<VfsNode, VfsError> {
        if path.is_empty() {
            return Err(VfsError::InvalidPath);
        }

        let mut current_id: Option<u64> = None;
        let mut components: [&[u8]; 32] = [&[]; 32];
        let mut comp_count: usize = 0;

        let path_str = core::str::from_utf8(path).map_err(|_| VfsError::InvalidPath)?;
        for component in path_str.split('/') {
            if !component.is_empty() {
                if comp_count < 32 {
                    components[comp_count] = component.as_bytes();
                    comp_count += 1;
                }
            }
        }

        if comp_count == 0 {
            // Lookup root directory "/"
            for i in 0..MAX_NODES {
                if let Some(ref node) = self.nodes[i] {
                    if node.name() == "/" {
                        return Ok(node.clone());
                    }
                }
            }
            return Err(VfsError::NodeNotFound);
        }

        // Find root first
        for i in 0..MAX_NODES {
            if let Some(ref node) = self.nodes[i] {
                if node.name() == "/" {
                    current_id = Some(node.id());
                    break;
                }
            }
        }

        for i in 0..comp_count {
            let comp = components[i];
            let found = if let Some(parent_id) = current_id {
                self.find_child_node(parent_id, comp)
            } else {
                None
            };

            match found {
                Some(id) => current_id = Some(id),
                None => return Err(VfsError::NodeNotFound),
            }
        }

        if let Some(id) = current_id {
            let idx = self.find_node_index(id).ok_or(VfsError::NodeNotFound)?;
            if let Some(ref node) = self.nodes[idx] {
                return Ok(node.clone());
            }
        }

        Err(VfsError::NodeNotFound)
    }

    /// Find a child node by name
    fn find_child_node(&self, parent_id: u64, name: &[u8]) -> Option<u64> {
        for i in 0..MAX_NODES {
            if let Some(ref node) = self.nodes[i] {
                if node.parent_id() == Some(parent_id) {
                    let node_name = node.name();
                    if node_name.as_bytes() == name {
                        return Some(node.id());
                    }
                }
            }
        }
        None
    }

    /// Read data from a node
    pub fn read(&self, id: u64, offset: u64, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let idx = self.find_node_index(id).ok_or(VfsError::NodeNotFound)?;
        
        let node = self.nodes[idx].as_ref().ok_or(VfsError::NodeNotFound)?;
        
        if node.node_type() != VfsNodeType::File {
            return Err(VfsError::NotAFile);
        }

        let data_size = node.data_size();
        
        if offset >= data_size as u64 {
            return Ok(0);
        }

        let remaining = data_size - offset as usize;
        let to_read = buffer.len().min(remaining);
        
        buffer[..to_read].copy_from_slice(&node.data[offset as usize..(offset as usize + to_read)]);
        
        Ok(to_read)
    }

    /// Write data to a node
    pub fn write(&mut self, id: u64, offset: u64, data: &[u8]) -> Result<usize, VfsError> {
        let idx = self.find_node_index(id).ok_or(VfsError::NodeNotFound)?;
        
        let node = self.nodes[idx].as_ref().ok_or(VfsError::NodeNotFound)?;
        
        if node.node_type() != VfsNodeType::File {
            return Err(VfsError::NotAFile);
        }

        let write_len = data.len() as u64;
        let new_size = (offset + write_len) as usize;
        
        if new_size > MAX_DATA_SIZE {
            return Err(VfsError::NoData);
        }
        
        if let Some(ref mut n) = self.nodes[idx] {
            n.data[offset as usize..(offset as usize + data.len())].copy_from_slice(data);
            n.data_size = n.data_size.max(new_size);
            n.size = n.size.max(new_size as u64);
        }
        
        Ok(data.len())
    }

    /// Get a node by ID
    pub fn get_node(&self, id: u64) -> Result<VfsNode, VfsError> {
        let idx = self.find_node_index(id).ok_or(VfsError::NodeNotFound)?;
        self.nodes[idx].as_ref().cloned().ok_or(VfsError::NodeNotFound)
    }

    /// Delete a node
    pub fn delete_node(&mut self, id: u64) -> Result<(), VfsError> {
        let idx = self.find_node_index(id).ok_or(VfsError::NodeNotFound)?;
        
        if let Some(ref node) = self.nodes[idx] {
            if node.is_mounted() {
                return Err(VfsError::AlreadyMounted);
            }
        }

        self.nodes[idx] = None;
        self.node_count -= 1;
        
        Ok(())
    }

    /// Create a new node
    pub fn create_node(&mut self, name: &str, node_type: VfsNodeType, parent: Option<u64>) -> Result<VfsNode, VfsError> {
        if self.node_count >= MAX_NODES {
            return Err(VfsError::NodeNotFound);
        }

        let idx = self.alloc_node().ok_or(VfsError::NodeNotFound)?;
        let id = self.next_node_id;
        self.next_node_id += 1;

        let mut node = VfsNode::new(id, name, node_type);
        node.set_parent(parent);

        self.nodes[idx] = Some(node.clone());
        self.node_count += 1;

        Ok(node)
    }
}

/// VFS errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VfsError {
    NodeNotFound,
    NotAFile,
    NotADirectory,
    MountTableFull,
    AlreadyMounted,
    NotMounted,
    NoData,
    InvalidPath,
}

impl VfsError {
    pub fn as_str(&self) -> &'static str {
        match self {
            VfsError::NodeNotFound => "Node not found",
            VfsError::NotAFile => "Not a file",
            VfsError::NotADirectory => "Not a directory",
            VfsError::MountTableFull => "Mount table full",
            VfsError::AlreadyMounted => "Already mounted",
            VfsError::NotMounted => "Not mounted",
            VfsError::NoData => "No data",
            VfsError::InvalidPath => "Invalid path",
        }
    }
}

/// Global VFS manager
static VFS_MANAGER: Mutex<VfsManager> = Mutex::new(VfsManager::new());

/// Get a reference to the global VFS manager
pub fn vfs_manager() -> &'static Mutex<VfsManager> {
    &VFS_MANAGER
}

/// Mount a filesystem
pub fn mount(source: u64, target: u64, flags: u32) -> Result<(), VfsError> {
    VFS_MANAGER.lock().mount(source, target, flags)
}

/// Unmount a filesystem
pub fn unmount(target: u64) -> Result<(), VfsError> {
    VFS_MANAGER.lock().unmount(target)
}

/// Lookup a node by path
pub fn lookup(path: &[u8]) -> Result<VfsNode, VfsError> {
    VFS_MANAGER.lock().lookup(path)
}

/// Read from a node
pub fn read(id: u64, offset: u64, buffer: &mut [u8]) -> Result<usize, VfsError> {
    VFS_MANAGER.lock().read(id, offset, buffer)
}

/// Write to a node
pub fn write(id: u64, offset: u64, data: &[u8]) -> Result<usize, VfsError> {
    VFS_MANAGER.lock().write(id, offset, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs_node_creation() {
        let node = VfsNode::new(1, "test", VfsNodeType::File);
        assert_eq!(node.id(), 1);
        assert_eq!(node.name(), "test");
        assert_eq!(node.node_type(), VfsNodeType::File);
    }

    #[test]
    fn test_vfs_manager_new() {
        let mgr = VfsManager::new();
        assert_eq!(mgr.node_count, 1);  // Root directory "/" is created
        assert_eq!(mgr.mount_count, 0);
    }

    #[test]
    fn test_node_name_with_slash() {
        let node = VfsNode::new(2, "test", VfsNodeType::File);
        assert_eq!(node.name(), "test");
    }

    #[test]
    fn test_create_multiple_nodes() {
        let mut mgr = VfsManager::new();
        let n1 = mgr.create_node("a", VfsNodeType::File, None);
        let n2 = mgr.create_node("b", VfsNodeType::File, None);
        assert!(n1.is_ok());
        assert!(n2.is_ok());
        assert_ne!(n1.unwrap().id, n2.unwrap().id);
    }

    #[test]
    fn test_lookup_root() {
        let mgr = VfsManager::new();
        let result = mgr.lookup(b"/");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "/");
    }

    #[test]
    fn test_lookup_nonexistent() {
        let mgr = VfsManager::new();
        let result = mgr.lookup(b"/nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_parent_child_relationship() {
        let mut mgr = VfsManager::new();
        let parent = mgr.create_node("parent", VfsNodeType::Directory, None).unwrap();
        let child = mgr.create_node("child", VfsNodeType::File, Some(parent.id()));
        assert!(child.is_ok());
        assert_eq!(child.unwrap().parent_id(), Some(parent.id()));
    }

    #[test]
    fn test_mount_unmount() {
        let mut mgr = VfsManager::new();
        let node = mgr.create_node("mnt", VfsNodeType::Directory, None).unwrap();
        assert!(mgr.mount(node.id(), 1).is_ok());
        assert!(mgr.unmount(node.id()).is_ok());
    }

    #[test]
    fn test_delete_node() {
        let mut mgr = VfsManager::new();
        let node = mgr.create_node("del", VfsNodeType::File, None).unwrap();
        assert!(mgr.delete_node(node.id()).is_ok());
        assert!(mgr.lookup(b"/del").is_err());
    }

    #[test]
    fn test_node_type_check() {
        let file = VfsNode::new(1, "f", VfsNodeType::File);
        let dir = VfsNode::new(2, "d", VfsNodeType::Directory);
        let symlink = VfsNode::new(3, "s", VfsNodeType::Symlink);
        assert_eq!(file.node_type(), VfsNodeType::File);
        assert_eq!(dir.node_type(), VfsNodeType::Directory);
        assert_eq!(symlink.node_type(), VfsNodeType::Symlink);
    }

    #[test]
    fn test_read_write_not_a_file() {
        let mut mgr = VfsManager::new();
        let dir = mgr.create_node("dir", VfsNodeType::Directory, None).unwrap();
        let mut buf = [0u8; 10];
        assert!(mgr.read(dir.id(), 0, &mut buf).is_err());
        assert!(mgr.write(dir.id(), 0, &[1,2,3]).is_err());
    }

    #[test]
    fn test_duplicate_mount() {
        let mut mgr = VfsManager::new();
        let node = mgr.create_node("mnt", VfsNodeType::Directory, None).unwrap();
        assert!(mgr.mount(node.id(), 1).is_ok());
        assert!(mgr.mount(node.id(), 2).is_err());
    }

    #[test]
    fn test_get_node() {
        let mut mgr = VfsManager::new();
        let node = mgr.create_node("get", VfsNodeType::File, None).unwrap();
        let fetched = mgr.get_node(node.id());
        assert!(fetched.is_ok());
        assert_eq!(fetched.unwrap().id(), node.id());
    }

    #[test]
    fn test_is_mounted() {
        let mut node = VfsNode::new(1, "mounted", VfsNodeType::Directory);
        assert!(!node.is_mounted());
        node.set_mounted(true);
        assert!(node.is_mounted());
    }
}
