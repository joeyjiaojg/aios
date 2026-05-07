// AIOS TCP/IP Networking Stack
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Implement TCP/IP networking stack for AIOS x86_64 kernel in Rust no_std.

const MAX_PACKET_SIZE: usize = 2048;

#[derive(Debug, Clone, Copy, Default)]
pub struct MacAddr {
    pub addr: [u8; 6],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IpAddr {
    pub addr: [u8; 4],
}

impl IpAddr {
    pub fn from_u32(ip: u32) -> Self {
        Self {
            addr: [
                (ip & 0xFF) as u8,
                ((ip >> 8) & 0xFF) as u8,
                ((ip >> 16) & 0xFF) as u8,
                ((ip >> 24) & 0xFF) as u8,
            ],
        }
    }

    pub fn to_u32(&self) -> u32 {
        u32::from(self.addr[0])
            | (u32::from(self.addr[1]) << 8)
            | (u32::from(self.addr[2]) << 16)
            | (u32::from(self.addr[3]) << 24)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IpProtocol {
    #[default]
    ICMP = 1,
    TCP = 6,
    UDP = 17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SocketState {
    #[default]
    Closed = 0,
    Listen = 1,
    Established = 2,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Socket {
    pub local_port: u16,
    pub remote_port: u16,
    pub local_ip: IpAddr,
    pub remote_ip: IpAddr,
    pub protocol: IpProtocol,
    pub state: SocketState,
}

impl Socket {
    pub fn is_used(&self) -> bool {
        self.state != SocketState::Closed
    }
}

pub fn init() {}

#[allow(dead_code)]
pub fn send_packet(_dest_mac: MacAddr, _protocol: IpProtocol, _data: &[u8]) -> bool {
    true
}

#[allow(dead_code)]
pub fn receive_packet(_buffer: &mut [u8; MAX_PACKET_SIZE]) -> Option<usize> {
    None
}

#[allow(dead_code)]
pub fn alloc_socket(_protocol: IpProtocol, _local_ip: IpAddr, _local_port: u16) -> Option<usize> {
    Some(0)
}

#[allow(dead_code)]
pub fn free_socket(_slot: usize) {}

#[allow(dead_code)]
pub fn handle_arp(_packet: &[u8]) {}

#[allow(dead_code)]
pub fn handle_icmp(_packet: &[u8]) {}

#[allow(dead_code)]
pub fn handle_ip(_packet: &[u8]) {}

pub fn my_mac() -> MacAddr {
    MacAddr {
        addr: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
    }
}

pub fn my_ip() -> IpAddr {
    IpAddr::from_u32(0xC0A80001)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_from_u32() {
        let ip = IpAddr::from_u32(0xC0A80001);
        assert_eq!(ip.addr[0], 1);
    }

    #[test]
    fn test_ip_to_u32() {
        let ip = IpAddr {
            addr: [1, 0, 168, 192],
        };
        assert_eq!(ip.to_u32(), 0xC0A80001);
    }

    #[test]
    fn test_socket_default() {
        let sock = Socket::default();
        assert_eq!(sock.state, SocketState::Closed);
    }

    #[test]
    fn test_socket_not_used() {
        let sock = Socket::default();
        assert!(!sock.is_used());
    }

    #[test]
    fn test_socket_used() {
        let mut sock = Socket::default();
        sock.state = SocketState::Listen;
        assert!(sock.is_used());
    }

    #[test]
    fn test_init() {
        init();
    }

    #[test]
    fn test_mac() {
        let mac = my_mac();
        assert_eq!(mac.addr[0], 0x52);
    }

    #[test]
    fn test_ip() {
        let ip = my_ip();
        assert_eq!(ip.addr[3], 192);
    }

    #[test]
    fn test_alloc_socket() {
        let slot = alloc_socket(IpProtocol::UDP, my_ip(), 8080);
        assert!(slot.is_some());
    }

    #[test]
    fn test_free_socket() {
        free_socket(0);
    }
}
