// AIOS PCI Enumeration
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create PCI enumeration for AIOS x86_64 kernel in Rust no_std. Implement PCI config space access, device scanning, vendor/device ID detection, BAR reading, and PciDevice struct with 256 slots.

use spin::Mutex;
use x86_64::instructions::port::Port;

pub const PCI_MAX_DEVICES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision_id: u8,
    pub header_type: u8,
    pub bist: u8,
    pub bars: [u32; 6],
    pub interrupt_pin: u8,
    pub interrupt_line: u8,
    pub present: bool,
}

impl PciDevice {
    pub const fn new() -> Self {
        Self {
            bus: 0,
            device: 0,
            function: 0,
            vendor_id: 0xFFFF,
            device_id: 0xFFFF,
            class_code: 0,
            subclass: 0,
            prog_if: 0,
            revision_id: 0,
            header_type: 0,
            bist: 0,
            bars: [0; 6],
            interrupt_pin: 0,
            interrupt_line: 0,
            present: false,
        }
    }

    pub fn get_bar(&self, index: usize) -> Option<u32> {
        if index < 6 {
            Some(self.bars[index])
        } else {
            None
        }
    }
}

impl Default for PciDevice {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PciBus {
    devices: [PciDevice; PCI_MAX_DEVICES],
    device_count: usize,
}

impl PciBus {
    pub const fn new() -> Self {
        Self {
            devices: [PciDevice::new(); PCI_MAX_DEVICES],
            device_count: 0,
        }
    }

    pub fn scan(&mut self) -> usize {
        self.device_count = 0;

        for bus in 0..=255u8 {
            for dev in 0..32u8 {
                for func in 0..8u8 {
                    if self.device_count >= PCI_MAX_DEVICES {
                        return self.device_count;
                    }

                    if let Some(device) = self.read_device(bus, dev, func) {
                        if device.present {
                            self.devices[self.device_count] = device;
                            self.device_count += 1;
                        }
                    }
                }
            }
        }

        self.device_count
    }

    fn read_device(&self, bus: u8, dev: u8, func: u8) -> Option<PciDevice> {
        let vendor = self.read_config16(bus, dev, func, 0x00);
        if vendor == 0xFFFF || vendor == 0x0000 {
            return None;
        }

        let mut device = PciDevice::new();
        device.bus = bus;
        device.device = dev;
        device.function = func;
        device.vendor_id = vendor;
        device.device_id = self.read_config16(bus, dev, func, 0x02);
        device.class_code = self.read_config8(bus, dev, func, 0x0B);
        device.subclass = self.read_config8(bus, dev, func, 0x0A);
        device.prog_if = self.read_config8(bus, dev, func, 0x09);
        device.revision_id = self.read_config8(bus, dev, func, 0x08);
        device.header_type = self.read_config8(bus, dev, func, 0x0E);
        device.bist = self.read_config8(bus, dev, func, 0x0F);
        device.interrupt_pin = self.read_config8(bus, dev, func, 0x3D);
        device.interrupt_line = self.read_config8(bus, dev, func, 0x3C);

        for i in 0..6 {
            let bar_offset = 0x10 + (i * 4) as u8;
            device.bars[i] = self.read_config32(bus, dev, func, bar_offset);
        }

        device.present = true;
        Some(device)
    }

    fn read_config8(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u8 {
        let address = self.make_address(bus, dev, func, offset as u32);
        self.config_read8(address)
    }

    fn read_config16(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u16 {
        let address = self.make_address(bus, dev, func, offset as u32);
        self.config_read16(address)
    }

    fn read_config32(&self, bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
        let address = self.make_address(bus, dev, func, offset as u32);
        self.config_read32(address)
    }

    pub const fn make_address(&self, bus: u8, dev: u8, func: u8, offset: u32) -> u32 {
        let mut address: u32 = 0x80000000;
        address |= (bus as u32) << 16;
        address |= (dev as u32) << 11;
        address |= (func as u32) << 8;
        address |= offset & 0xFC;
        address
    }

    fn config_read8(&self, address: u32) -> u8 {
        // Safety: Reading from PCI config space ports (0xCF8, 0xCFC).
        // These are standard I/O ports for PCI configuration on x86.
        // Address is properly constructed with make_address().
        unsafe {
            let mut address_port = Port::<u32>::new(0xCF8);
            address_port.write(address);
            let mut data_port = Port::<u8>::new(0xCFC + ((address & 0x03) as u16));
            data_port.read()
        }
    }

    fn config_read16(&self, address: u32) -> u16 {
        // Safety: Reading from PCI config space ports (0xCF8, 0xCFC).
        // These are standard I/O ports for PCI configuration on x86.
        // Address is properly constructed with make_address().
        unsafe {
            let mut address_port = Port::<u32>::new(0xCF8);
            address_port.write(address);
            let mut data_port = Port::<u16>::new(0xCFC + ((address & 0x02) as u16));
            data_port.read()
        }
    }

    fn config_read32(&self, address: u32) -> u32 {
        // Safety: Reading from PCI config space ports (0xCF8, 0xCFC).
        // These are standard I/O ports for PCI configuration on x86.
        // Address is properly constructed with make_address().
        unsafe {
            let mut address_port = Port::<u32>::new(0xCF8);
            address_port.write(address);
            let mut data_port = Port::<u32>::new(0xCFC);
            data_port.read()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_config32(&self, bus: u8, dev: u8, func: u8, offset: u8, value: u32) {
        let address = self.make_address(bus, dev, func, offset as u32);
        // Safety: Writing to PCI config space ports. This is required for
        // device initialization (enabling memory/IO space, etc.).
        unsafe {
            let mut address_port = Port::<u32>::new(0xCF8);
            address_port.write(address);
            let mut data_port = Port::<u32>::new(0xCFC);
            data_port.write(value);
        }
    }

    pub fn enable_device(&self, bus: u8, dev: u8, func: u8) {
        let command = self.read_config16(bus, dev, func, 0x04);
        self.write_config32(bus, dev, func, 0x04, (command | 0x0007) as u32);
    }

    pub fn get_device(&self, index: usize) -> Option<&PciDevice> {
        if index < self.device_count {
            Some(&self.devices[index])
        } else {
            None
        }
    }

    pub fn device_count(&self) -> usize {
        self.device_count
    }

    pub fn find_device(&self, vendor_id: u16, device_id: u16) -> Option<&PciDevice> {
        for i in 0..self.device_count {
            if self.devices[i].vendor_id == vendor_id && self.devices[i].device_id == device_id {
                return Some(&self.devices[i]);
            }
        }
        None
    }

    pub fn find_by_class(&self, class_code: u8, subclass: u8) -> Option<&PciDevice> {
        for i in 0..self.device_count {
            if self.devices[i].class_code == class_code && self.devices[i].subclass == subclass {
                return Some(&self.devices[i]);
            }
        }
        None
    }
}

impl Default for PciBus {
    fn default() -> Self {
        Self::new()
    }
}

static PCI_BUS: Mutex<PciBus> = Mutex::new(PciBus::new());

pub fn init() {
    let mut bus = PCI_BUS.lock();
    let count = bus.scan();
    let _ = count;
}

pub fn scan() -> usize {
    PCI_BUS.lock().scan()
}

pub fn get_device(index: usize) -> Option<PciDevice> {
    PCI_BUS.lock().get_device(index).copied()
}

pub fn device_count() -> usize {
    PCI_BUS.lock().device_count()
}

pub fn find_device(vendor_id: u16, device_id: u16) -> Option<PciDevice> {
    PCI_BUS.lock().find_device(vendor_id, device_id).copied()
}

pub fn find_by_class(class_code: u8, subclass: u8) -> Option<PciDevice> {
    PCI_BUS.lock().find_by_class(class_code, subclass).copied()
}

pub fn enable_device(bus: u8, dev: u8, func: u8) {
    PCI_BUS.lock().enable_device(bus, dev, func)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pci_device_new() {
        let dev = PciDevice::new();
        assert!(!dev.present);
        assert_eq!(0xFFFF, dev.vendor_id);
    }

    #[test]
    fn test_pci_bus_new() {
        let bus = PciBus::new();
        assert_eq!(0, bus.device_count());
    }

    #[test]
    fn test_pci_device_default() {
        let dev = PciDevice::default();
        assert!(!dev.present);
    }

    #[test]
    fn test_pci_bus_default() {
        let bus = PciBus::default();
        assert_eq!(0, bus.device_count());
    }

    #[test]
    fn test_pci_device_get_bar() {
        let mut dev = PciDevice::new();
        dev.bars[0] = 0x12345678;
        dev.bars[5] = 0xABCDEF00;
        assert_eq!(Some(0x12345678), dev.get_bar(0));
        assert_eq!(Some(0xABCDEF00), dev.get_bar(5));
        assert_eq!(None, dev.get_bar(6));
    }

    #[test]
    fn test_make_address() {
        let bus = PciBus::new();
        let addr = bus.make_address(0, 0, 0, 0);
        assert_eq!(0x80000000, addr);

        let addr2 = bus.make_address(1, 2, 3, 4);
        assert_eq!(0x8001_2104, addr2);
    }

    #[test]
    fn test_pci_max_devices() {
        assert_eq!(256, PCI_MAX_DEVICES);
    }

    #[test]
    fn test_scan_returns_count() {
        let mut bus = PciBus::new();
        let count = bus.scan();
        assert!(count <= PCI_MAX_DEVICES);
    }

    #[test]
    fn test_find_device_not_found() {
        let bus = PciBus::new();
        let result = bus.find_device(0xFFFF, 0xFFFF);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_by_class_not_found() {
        let bus = PciBus::new();
        let result = bus.find_by_class(0xFF, 0xFF);
        assert!(result.is_none());
    }
}
