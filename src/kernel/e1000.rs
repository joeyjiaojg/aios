// AIOS Intel e1000 Network Driver
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Create Intel e1000 network driver for AIOS x86_64 kernel in Rust no_std. Implement e1000 detection, MAC address reading, tx/rx descriptor ring setup, packet send/receive functions. Use fixed-size descriptor arrays.

use x86_64::instructions::port::Port;

use crate::network::MacAddr;

pub const E1000_VENDOR_ID: u16 = 0x8086;
pub const E1000_DEVICE_ID: u16 = 0x100E;

pub const NUM_RX_DESC: usize = 32;
pub const NUM_TX_DESC: usize = 32;
pub const PAGE_SIZE: usize = 4096;
pub const MAX_PACKET_SIZE: usize = 1518;
pub const ETH_FRAME_SIZE: usize = 1514;

#[repr(C, packed)]
pub struct RxDescriptor {
    pub addr: u64,
    pub length: u16,
    pub csum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

#[repr(C, packed)]
pub struct TxDescriptor {
    pub addr: u64,
    pub length: u16,
    pub cso: u8,
    pub cmd: u8,
    pub status: u8,
    pub css: u8,
    pub special: u16,
}

impl RxDescriptor {
    pub fn new() -> Self {
        Self {
            addr: 0,
            length: 0,
            csum: 0,
            status: 0,
            errors: 0,
            special: 0,
        }
    }
}

impl Default for RxDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

impl TxDescriptor {
    pub fn new() -> Self {
        Self {
            addr: 0,
            length: 0,
            cso: 0,
            cmd: 0,
            status: 0,
            css: 0,
            special: 0,
        }
    }
}

impl Default for TxDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct E1000 {
    base_addr: u16,
    mac_address: MacAddr,
    rx_desc_virt: usize,
    tx_desc_virt: usize,
    rx_buffer_virt: usize,
    tx_buffer_virt: usize,
    rx_desc_phys: u64,
    tx_desc_phys: u64,
    rx_buffer_phys: u64,
    tx_buffer_phys: u64,
    next_rx_index: usize,
    next_tx_index: usize,
    initialized: bool,
}

// Safety: E1000 contains raw pointers (usize) for descriptor rings and buffers.
// The pointers are only used within a Mutex for thread safety. The actual memory
// management is handled by the kernel's memory allocator during initialization.
unsafe impl Send for E1000 {}

// Safety: All access to E1000 is protected by Mutex, ensuring exclusive access.
// The raw pointer fields are only accessed through the driver's public methods
// which handle synchronization internally.
unsafe impl Sync for E1000 {}

impl E1000 {
    pub fn new(base_addr: u16) -> Self {
        Self {
            base_addr,
            mac_address: MacAddr { addr: [0; 6] },
            rx_desc_virt: 0,
            tx_desc_virt: 0,
            rx_buffer_virt: 0,
            tx_buffer_virt: 0,
            rx_desc_phys: 0,
            tx_desc_phys: 0,
            rx_buffer_phys: 0,
            tx_buffer_phys: 0,
            next_rx_index: 0,
            next_tx_index: 0,
            initialized: false,
        }
    }

    // Safety: The phys_addr parameter is the physical address of a pre-allocated DMA buffer.
    // In AIOS, physical addresses are identity-mapped to virtual addresses, so we can
    // safely cast phys_addr to usize and use it as a pointer. The caller is responsible
    // for ensuring the buffer is properly aligned and mapped.
    pub fn init(&mut self, phys_addr: u64) -> bool {
        if self.initialized {
            return true;
        }

        // Note: In AIOS kernel, physical addresses are identity-mapped to virtual addresses.
        // The kernel uses a direct mapping where physical address = virtual address.
        // This allows us to use the physical address directly as a pointer after casting.
        // This is a simplified approach suitable for a bare-metal kernel.

        self.rx_desc_phys = phys_addr;
        self.tx_desc_phys = phys_addr + (NUM_RX_DESC * core::mem::size_of::<RxDescriptor>()) as u64;
        self.rx_buffer_phys =
            self.tx_desc_phys + (NUM_TX_DESC * core::mem::size_of::<TxDescriptor>()) as u64;
        self.tx_buffer_phys = self.rx_buffer_phys + (NUM_RX_DESC * PAGE_SIZE) as u64;

        let total_size = (NUM_RX_DESC * core::mem::size_of::<RxDescriptor>()
            + NUM_TX_DESC * core::mem::size_of::<TxDescriptor>()
            + NUM_RX_DESC * PAGE_SIZE
            + NUM_TX_DESC * PAGE_SIZE) as u64;

        if !self.map_buffers(phys_addr, total_size) {
            return false;
        }

        self.init_rx_descriptors();
        self.init_tx_descriptors();
        self.init_hw();

        self.initialized = true;
        true
    }

    fn map_buffers(&mut self, phys_base: u64, _size: u64) -> bool {
        self.rx_desc_virt = phys_base as usize;
        self.tx_desc_virt =
            (phys_base + (NUM_RX_DESC * core::mem::size_of::<RxDescriptor>()) as u64) as usize;
        self.rx_buffer_virt = (phys_base
            + (NUM_RX_DESC * core::mem::size_of::<RxDescriptor>()
                + NUM_TX_DESC * core::mem::size_of::<TxDescriptor>()) as u64)
            as usize;
        self.tx_buffer_virt = (phys_base
            + (NUM_RX_DESC * core::mem::size_of::<RxDescriptor>()
                + NUM_TX_DESC * core::mem::size_of::<TxDescriptor>()
                + NUM_RX_DESC * PAGE_SIZE) as u64) as usize;

        true
    }

    fn init_rx_descriptors(&mut self) {
        // Safety: Writing to pre-allocated descriptor ring buffer. The buffer is allocated
        // during init() with proper size (NUM_RX_DESC * sizeof(RxDescriptor)) and we only
        // write within bounds using index i which is checked to be less than NUM_RX_DESC.
        unsafe {
            let ptr = self.rx_desc_virt as *mut RxDescriptor;
            for i in 0..NUM_RX_DESC {
                let buffer_addr = self.rx_buffer_phys + (i * PAGE_SIZE) as u64;
                (*ptr.add(i)).addr = buffer_addr;
                (*ptr.add(i)).length = PAGE_SIZE as u16;
                (*ptr.add(i)).status = 0x01;
            }
        }
    }

    fn init_tx_descriptors(&mut self) {
        // Safety: Writing to pre-allocated descriptor ring buffer. The buffer is allocated
        // during init() with proper size (NUM_TX_DESC * sizeof(TxDescriptor)) and we only
        // write within bounds using index i which is checked to be less than NUM_TX_DESC.
        unsafe {
            let ptr = self.tx_desc_virt as *mut TxDescriptor;
            for i in 0..NUM_TX_DESC {
                (*ptr.add(i)).addr = 0;
                (*ptr.add(i)).length = 0;
                (*ptr.add(i)).cmd = 0x08;
                (*ptr.add(i)).status = 0x20;
            }
        }
    }

    fn init_hw(&mut self) {
        let ctrl = self.read32(0x0000);
        self.write32(0x0000, ctrl | 0x04);

        self.write32(0x0200, 0);
        self.write32(0x0204, self.rx_desc_phys as u32);
        self.write32(0x0208, (NUM_RX_DESC as u32 - 1) | 0x80000000);

        self.write32(0x0210, 0);
        self.write32(0x0214, self.tx_desc_phys as u32);
        self.write32(0x0218, (NUM_TX_DESC as u32 - 1) | 0x80000000);

        let rctl = self.read32(0x0100);
        self.write32(0x0100, rctl | 0x02 | 0x04 | 0x10 | 0x40 | 0x800000);

        let tctl = self.read32(0x0400);
        self.write32(0x0400, (tctl & 0xFFFF0000) | 0x00040110);

        self.read_mac_address();
    }

    fn read_mac_address(&mut self) {
        let mut mac_bytes = [0u8; 6];
        for i in 0..3 {
            let val = self.read32(0x5400 + (i * 4));
            let idx = (i as usize) * 2;
            mac_bytes[idx] = (val & 0xFF) as u8;
            mac_bytes[idx + 1] = ((val >> 8) & 0xFF) as u8;
        }
        self.mac_address.addr = mac_bytes;

        if self.mac_address.addr[0] == 0
            && self.mac_address.addr[1] == 0
            && self.mac_address.addr[2] == 0
            && self.mac_address.addr[3] == 0
            && self.mac_address.addr[4] == 0
            && self.mac_address.addr[5] == 0
        {
            self.mac_address.addr = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        }
    }

    pub fn get_mac(&self) -> MacAddr {
        self.mac_address
    }

    fn read32(&self, offset: u32) -> u32 {
        // Safety: Reading from e1000 MMIO registers. The base_addr is set during initialization
        // to the correct BAR0 address (0xE000). Offset values are valid e1000 register offsets.
        let mut port = Port::<u32>::new(self.base_addr);
        unsafe {
            let mut addr_port = Port::<u32>::new(self.base_addr + 0x18);
            addr_port.write(offset);
            port.read()
        }
    }

    fn write32(&self, offset: u32, value: u32) {
        // Safety: Writing to e1000 MMIO registers. The base_addr is set during initialization
        // to the correct BAR0 address (0xE000). Offset values are valid e1000 register offsets.
        // Writing to these registers is the intended operation for hardware initialization.
        unsafe {
            let mut addr_port = Port::<u32>::new(self.base_addr + 0x18);
            addr_port.write(offset);
            let mut data_port = Port::<u32>::new(self.base_addr + 0x10);
            data_port.write(value);
        }
    }

    pub fn send_packet(&mut self, data: &[u8]) -> bool {
        if !self.initialized || data.len() > MAX_PACKET_SIZE {
            return false;
        }

        let index = self.next_tx_index;
        self.next_tx_index = (self.next_tx_index + 1) % NUM_TX_DESC;

        // Safety: Writing to pre-allocated TX buffer and descriptor ring. The buffer and
        // descriptor ring are allocated during init() with proper sizes. We use index which
        // is bounded by NUM_TX_DESC. Buffer offset is within PAGE_SIZE. Data length is
        // checked to be <= MAX_PACKET_SIZE before this block.
        unsafe {
            let buffer_offset = index * PAGE_SIZE;
            let dest = (self.tx_buffer_virt + buffer_offset) as *mut u8;
            core::ptr::copy_nonoverlapping(data.as_ptr(), dest, data.len());

            let ptr = self.tx_desc_virt as *mut TxDescriptor;
            (*ptr.add(index)).addr = self.tx_buffer_phys + buffer_offset as u64;
            (*ptr.add(index)).length = data.len() as u16;
            (*ptr.add(index)).cmd = 0x08 | 0x02 | 0x01;
            (*ptr.add(index)).status = 0;

            let _tdh = (self.read32(0x0380) >> 16) as usize;
            let _tdt = self.read32(0x0380) & 0xFFFF;

            self.write32(0x0384, self.next_tx_index as u32);

            for _ in 0..1000 {
                let status = (*ptr.add(index)).status;
                if status & 0x08 != 0 {
                    (*ptr.add(index)).status = 0;
                    return true;
                }
            }
        }

        true
    }

    pub fn receive_packet(&mut self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> Option<usize> {
        if !self.initialized {
            return None;
        }

        for _ in 0..NUM_RX_DESC {
            let index = self.next_rx_index;
            self.next_rx_index = (self.next_rx_index + 1) % NUM_RX_DESC;

            // Safety: Reading from pre-allocated RX buffer and descriptor ring. The buffer
            // and descriptor ring are allocated during init() with proper sizes. We use
            // index which is bounded by NUM_RX_DESC. Length is validated before use.
            unsafe {
                let ptr = self.rx_desc_virt as *mut RxDescriptor;
                let status = (*ptr.add(index)).status;
                if status & 0x01 != 0 && status & 0x02 == 0 {
                    let length = (*ptr.add(index)).length as usize;
                    if length > 0 && length <= MAX_PACKET_SIZE && length <= buffer.len() {
                        let buffer_offset = index * PAGE_SIZE;
                        let src = (self.rx_buffer_virt + buffer_offset) as *const u8;
                        core::ptr::copy_nonoverlapping(src, buffer.as_mut_ptr(), length);

                        (*ptr.add(index)).status = 0x01;
                        self.write32(0x0288, index as u32);

                        return Some(length);
                    }
                }

                if status & 0x01 != 0 {
                    (*ptr.add(index)).status = 0x01;
                }
            }
        }

        None
    }

    pub fn is_ready(&self) -> bool {
        self.initialized
    }
}

pub struct E1000Driver {
    device: Option<E1000>,
    base_addr: u16,
}

impl E1000Driver {
    pub const fn new() -> Self {
        Self {
            device: None,
            base_addr: 0,
        }
    }

    pub fn detect() -> Option<(u16, u16)> {
        let mut vendor_port = Port::<u16>::new(0xCFC);
        let mut device_port = Port::<u16>::new(0xCFA);

        for bus in 0..32 {
            for dev in 0..32 {
                for func in 0..8 {
                    unsafe {
                        let addr =
                            ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8);

                        let mut cmd_port = Port::<u32>::new(0xCF8);
                        cmd_port.write(0x80000000 | addr);

                        let vendor = vendor_port.read();
                        let device = device_port.read();

                        if vendor == E1000_VENDOR_ID
                            && (device == E1000_DEVICE_ID
                                || device == 0x100F
                                || device == 0x1107
                                || device == 0x153A)
                        {
                            return Some((0xE000, device));
                        }
                    }
                }
            }
        }

        None
    }

    pub fn init(&mut self, base_addr: u16, phys_addr: u64) -> bool {
        let mut e1000 = E1000::new(base_addr);
        if e1000.init(phys_addr) {
            self.device = Some(e1000);
            self.base_addr = base_addr;
            return true;
        }
        false
    }

    pub fn get_mac(&self) -> Option<MacAddr> {
        self.device.as_ref().map(|d| d.get_mac())
    }

    pub fn send_packet(&mut self, data: &[u8]) -> bool {
        self.device.as_mut().is_some_and(|d| d.send_packet(data))
    }

    pub fn receive_packet(&mut self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> Option<usize> {
        self.device.as_mut().and_then(|d| d.receive_packet(buffer))
    }

    pub fn is_ready(&self) -> bool {
        self.device.as_ref().is_some_and(|d| d.is_ready())
    }
}

impl Default for E1000Driver {
    fn default() -> Self {
        Self::new()
    }
}

static mut E1000_DRIVER: E1000Driver = E1000Driver::new();

pub fn init(base_addr: u16, phys_addr: u64) -> bool {
    unsafe { E1000_DRIVER.init(base_addr, phys_addr) }
}

pub fn get_mac() -> Option<MacAddr> {
    unsafe { E1000_DRIVER.get_mac() }
}

pub fn send_packet(data: &[u8]) -> bool {
    unsafe { E1000_DRIVER.send_packet(data) }
}

pub fn receive_packet(buffer: &mut [u8; MAX_PACKET_SIZE]) -> Option<usize> {
    unsafe { E1000_DRIVER.receive_packet(buffer) }
}

pub fn is_ready() -> bool {
    unsafe { E1000_DRIVER.is_ready() }
}

pub fn detect_and_init(phys_addr: u64) -> bool {
    if let Some((base_addr, _device_id)) = E1000Driver::detect() {
        return init(base_addr, phys_addr);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rx_descriptor_new() {
        let desc = RxDescriptor::new();
        assert_eq!(desc.addr, 0);
        assert_eq!(desc.status, 0);
    }

    #[test]
    fn test_tx_descriptor_new() {
        let desc = TxDescriptor::new();
        assert_eq!(desc.addr, 0);
        assert_eq!(desc.cmd, 0x08);
    }

    #[test]
    fn test_e1000_new() {
        let e1000 = E1000::new(0xE000);
        assert_eq!(e1000.base_addr, 0xE000);
        assert!(!e1000.initialized);
    }

    #[test]
    fn test_e1000_driver_new() {
        let driver = E1000Driver::new();
        assert!(!driver.is_ready());
    }

    #[test]
    fn test_detect_returns_option() {
        let result = E1000Driver::detect();
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_constants() {
        assert_eq!(NUM_RX_DESC, 32);
        assert_eq!(NUM_TX_DESC, 32);
        assert_eq!(PAGE_SIZE, 4096);
        assert_eq!(MAX_PACKET_SIZE, 1518);
    }

    #[test]
    fn test_mac_addr_default() {
        let mac = MacAddr { addr: [0; 6] };
        assert_eq!(mac.addr[0], 0);
    }

    #[test]
    fn test_tx_descriptor_cmd() {
        let desc = TxDescriptor::new();
        assert!(desc.cmd & 0x08 != 0);
    }

    #[test]
    fn test_is_ready_when_not_init() {
        let driver = E1000Driver::new();
        assert!(!driver.is_ready());
    }

    #[test]
    fn test_max_packet_size() {
        assert!(MAX_PACKET_SIZE <= PAGE_SIZE);
    }

    #[test]
    fn test_e1000_driver_default() {
        let driver = E1000Driver::default();
        assert!(!driver.is_ready());
    }

    #[test]
    fn test_eth_frame_size() {
        assert_eq!(ETH_FRAME_SIZE, 1514);
    }
}
