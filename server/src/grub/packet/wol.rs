use std::net::Ipv4Addr;

use async_std::net::UdpSocket;

const SIX_FF: [u8; 6] = [0xFF; 6];

pub struct MagicPacket {
    packet: Vec<u8>,
}

impl MagicPacket {
    pub fn new(mac_address: &[u8; 6]) -> MagicPacket {
        log::debug!("construct MagicPacket of {:x?}", mac_address);
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
    pub async fn send(&self) {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await.unwrap();
        socket.set_broadcast(true).unwrap();
        socket
            .send_to(&self.packet, (Ipv4Addr::BROADCAST, 9))
            .await
            .unwrap();
    }
}
