// AIOS Device Driver Framework
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create device driver framework for AIOS x86_64 kernel in Rust no_std with Device struct, DriverManager with 16 devices, spin::Mutex

use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly};

/// Maximum number of devices
const MAX_DEVICES: usize = 16;

/// Device types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceType {
    Block,
    Char,
    Net,
}

/// Device structure
#[derive(Debug, Clone, Copy)]
pub struct Device {
    pub id: u64,
    pub name: [u8; 32],
    pub device_type: DeviceType,
    pub status: DeviceStatus,
}

/// Device status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceStatus {
    Active,
    Inactive,
    Error,
}

/// Driver manager
pub struct DriverManager {
    devices: [Option<Device>; MAX_DEVICES],
    next_id: u64,
    device_count: usize,
}

impl DriverManager {
    /// Create a new driver manager
    pub fn new() -> Self {
        Self {
            devices: [None; MAX_DEVICES],
            next_id: 1,
            device_count: 0,
        }
    }
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DriverManager {
    /// Register a new device
    pub fn register_device(
        &mut self,
        name: &str,
        device_type: DeviceType,
    ) -> Result<u64, DriverError> {
        if self.device_count >= MAX_DEVICES {
            return Err(DriverError::ManagerFull);
        }

        let mut name_arr = [0u8; 32];
        let bytes = name.as_bytes();
        let len = bytes.len().min(31);
        name_arr[..len].copy_from_slice(&bytes[..len]);

        let id = self.next_id;
        self.next_id += 1;

        for i in 0..MAX_DEVICES {
            if self.devices[i].is_none() {
                self.devices[i] = Some(Device {
                    id,
                    name: name_arr,
                    device_type,
                    status: DeviceStatus::Inactive,
                });
                self.device_count += 1;
                return Ok(id);
            }
        }

        Err(DriverError::ManagerFull)
    }

    /// Find device by ID
    pub fn find_device(&self, id: u64) -> Result<Device, DriverError> {
        for i in 0..MAX_DEVICES {
            if let Some(ref dev) = self.devices[i] {
                if dev.id == id {
                    return Ok(*dev);
                }
            }
        }
        Err(DriverError::DeviceNotFound)
    }

    /// Read from device
    pub fn read_device(&self, id: u64, buffer: &mut [u8]) -> Result<usize, DriverError> {
        let dev = self.find_device(id)?;
        if dev.status != DeviceStatus::Active {
            return Err(DriverError::DeviceNotActive);
        }

        let base_port = 0x300 + (id as u16 * 0x10);
        let mut data_port = Port::<u8>::new(base_port);
        let mut status_port = PortReadOnly::new(base_port + 1);

        let mut bytes_read = 0;
        let buffer_len = buffer.len();
        for byte in buffer.iter_mut() {
            // Safety: Reading from I/O ports is safe when the port address is valid
            // and we only read from ports that don't have side effects on read
            unsafe {
                for _ in 0..1000 {
                    let status: u8 = status_port.read();
                    if status & 0x01 != 0 {
                        break;
                    }
                }

                *byte = data_port.read();
            }
            bytes_read += 1;

            if bytes_read >= buffer_len {
                break;
            }
        }

        Ok(bytes_read)
    }

    /// Write to device
    pub fn write_device(&mut self, id: u64, data: &[u8]) -> Result<usize, DriverError> {
        let dev = self.find_device(id)?;
        if dev.status != DeviceStatus::Active {
            return Err(DriverError::DeviceNotActive);
        }

        let base_port = 0x300 + (id as u16 * 0x10);
        let mut data_port = Port::<u8>::new(base_port);
        let mut status_port = PortReadOnly::new(base_port + 1);

        let mut bytes_written = 0;
        let data_len = data.len();
        for byte in data.iter() {
            // Safety: Writing to I/O ports is safe when the port address is valid
            // and we only write to data ports that accept output
            unsafe {
                for _ in 0..1000 {
                    let status: u8 = status_port.read();
                    if status & 0x02 != 0 {
                        break;
                    }
                }

                data_port.write(*byte);
            }
            bytes_written += 1;

            if bytes_written >= data_len {
                break;
            }
        }

        Ok(bytes_written)
    }

    /// Activate device
    pub fn activate_device(&mut self, id: u64) -> Result<(), DriverError> {
        for i in 0..MAX_DEVICES {
            if let Some(ref mut dev) = self.devices[i] {
                if dev.id == id {
                    dev.status = DeviceStatus::Active;
                    return Ok(());
                }
            }
        }
        Err(DriverError::DeviceNotFound)
    }
}

/// Driver errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriverError {
    DeviceNotFound,
    ManagerFull,
    DeviceNotActive,
}

/// Global driver manager
static DRIVER_MANAGER: Mutex<Option<DriverManager>> = Mutex::new(None);

/// Initialize driver manager
pub fn init() {
    *DRIVER_MANAGER.lock() = Some(DriverManager::new());
}

/// Register device
pub fn register_device(name: &str, device_type: DeviceType) -> Result<u64, DriverError> {
    DRIVER_MANAGER
        .lock()
        .as_mut()
        .unwrap()
        .register_device(name, device_type)
}

/// Find device
pub fn find_device(id: u64) -> Result<Device, DriverError> {
    DRIVER_MANAGER.lock().as_ref().unwrap().find_device(id)
}

/// Read from device
pub fn read_device(id: u64, buffer: &mut [u8]) -> Result<usize, DriverError> {
    DRIVER_MANAGER
        .lock()
        .as_ref()
        .unwrap()
        .read_device(id, buffer)
}

/// Write to device
pub fn write_device(id: u64, data: &[u8]) -> Result<usize, DriverError> {
    DRIVER_MANAGER
        .lock()
        .as_mut()
        .unwrap()
        .write_device(id, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let dev = Device {
            id: 1,
            name: [0u8; 32],
            device_type: DeviceType::Block,
            status: DeviceStatus::Inactive,
        };
        assert_eq!(dev.id, 1);
        assert_eq!(dev.device_type, DeviceType::Block);
    }

    #[test]
    fn test_manager_register() {
        let mut mgr = DriverManager::new();
        let id = mgr.register_device("sda", DeviceType::Block);
        assert!(id.is_ok());
    }

    #[test]
    fn test_find_device() {
        let mut mgr = DriverManager::new();
        let id = mgr.register_device("eth0", DeviceType::Net).unwrap();
        let dev = mgr.find_device(id);
        assert!(dev.is_ok());
        assert_eq!(dev.unwrap().device_type, DeviceType::Net);
    }

    #[test]
    fn test_manager_full() {
        let mut mgr = DriverManager::new();
        for i in 0..17 {
            let name = [b'd', b'e', b'v', b'_', (b'0' + (i % 10))];
            let _ = mgr.register_device(core::str::from_utf8(&name).unwrap(), DeviceType::Char);
        }
        assert_eq!(mgr.device_count, 16);
    }

    #[test]
    fn test_activate_device() {
        let mut mgr = DriverManager::new();
        let id = mgr.register_device("sdb", DeviceType::Block).unwrap();
        assert!(mgr.activate_device(id).is_ok());
    }

    #[test]
    fn test_read_inactive_device() {
        let mut mgr = DriverManager::new();
        let id = mgr.register_device("sdc", DeviceType::Block).unwrap();
        let mut buf = [0u8; 10];
        assert!(mgr.read_device(id, &mut buf).is_err());
    }

    #[test]
    fn test_device_types() {
        assert_eq!(DeviceType::Block, DeviceType::Block);
        assert_eq!(DeviceType::Char, DeviceType::Char);
        assert_eq!(DeviceType::Net, DeviceType::Net);
    }
}
