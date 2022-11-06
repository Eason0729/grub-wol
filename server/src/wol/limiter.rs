use std::future::Future;
use std::net::{Ipv4Addr, UdpSocket};
use std::{mem, time};

const SIX_FF: [u8; 6] = [0xFF; 6];
const LIMIT_INTERVAL_MS: u128 = 500;

struct FlowLimiter {
    last_access: time::Instant,
    packet: MagicPacket,
}

impl FlowLimiter {
    fn new(mac_address: &[u8; 6]) -> Self {
        FlowLimiter {
            last_access: time::Instant::now(),
            packet: MagicPacket::new(mac_address),
        }
    }
    fn try_send(&mut self) -> bool {
        let now = &mut time::Instant::now();
        mem::swap(&mut self.last_access, now);
        if now.duration_since(self.last_access).as_millis() >= LIMIT_INTERVAL_MS {
            self.packet.send();
            true
        } else {
            false
        }
    }
}

pub struct MagicPacket {
    packet: Vec<u8>,
}

impl MagicPacket {
    pub fn new(mac_address: &[u8; 6]) -> MagicPacket {
        MagicPacket {
            packet: {
                let mac_address: &[u8; 6] = &mac_address;
                let dst: &mut Vec<u8> = &mut vec![0_u8; 0];

                dst.extend_from_slice(&SIX_FF);

                (0..16).for_each(|_iter| {
                    dst.extend_from_slice(mac_address);
                });

                dst.to_owned()
            },
        }
    }
    pub fn send(&self) {
        println!("user  : sending MagicPacket");
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        socket.set_broadcast(true).unwrap();
        socket
            .send_to(&self.packet, (Ipv4Addr::BROADCAST, 9))
            .unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn to_magic_packet_test() {
        assert_eq!(
            MagicPacket::new(&[1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8]).packet,
            [
                255_u8, 255_u8, 255_u8, 255_u8, 255_u8, 255_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8,
                1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8,
                3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8,
                5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8,
                1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8,
                3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8,
                5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8,
                1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8
            ]
            .to_vec()
        );
    }
}
