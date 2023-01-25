// dummy client
use async_std::{
    net::{self, UdpSocket},
    task::sleep,
};
use proto::prelude::*;
use rand::Rng;
use std::{collections::*, time::Duration};

type Conn = TcpConn<host::Packet, server::Packet>;

const SIX_FF: [u8; 6] = [0xFF; 6];

#[derive(Hash, PartialEq, Eq, Clone)]
enum OS {
    Down,
    Windows,
    Debian,
    Ubuntu,
}

struct OSInfo {
    uid: ID,
}

#[async_std::test]
async fn test_main() {
    loop {
        let mac_address: [u8; 6] = rand::thread_rng().gen();

        let mut magic_packet = Vec::new();
        magic_packet.extend_from_slice(&SIX_FF);
        (0..16).for_each(|_iter| {
            magic_packet.extend_from_slice(&mac_address);
        });

        let mut current_os = OS::Windows;
        let mut os_storage: HashMap<OS, OSInfo> = [
            (OS::Windows, OSInfo { uid: 0 }),
            (OS::Debian, OSInfo { uid: 0 }),
            (OS::Ubuntu, OSInfo { uid: 0 }),
        ]
        .into_iter()
        .collect();
        let boot_path: HashMap<OS, HashMap<GrubId, OS>> = [
            (OS::Windows, [].into_iter().collect()),
            (
                OS::Debian,
                [(1, OS::Windows), (2, OS::Debian), (3, OS::Ubuntu)]
                    .into_iter()
                    .collect(),
            ),
            (
                OS::Ubuntu,
                [(4, OS::Windows), (6, OS::Debian), (5, OS::Ubuntu)]
                    .into_iter()
                    .collect(),
            ),
        ]
        .into_iter()
        .collect();
        let conn = Some(
            net::TcpStream::connect(format!("127.0.0.1:{}", SERVER_PORT))
                .await
                .unwrap(),
        );

        let mut packet = Some(Conn::from_tcp(conn.unwrap()));

        let handshake = host::HandShake {
            ident: PROTO_IDENT,
            mac_address,
            uid: os_storage.get(&current_os).unwrap().uid,
            version: APIVERSION,
        };

        println!("handshake sent");

        packet
            .as_mut()
            .unwrap()
            .send(host::Packet::HandShake(handshake))
            .await
            .unwrap();

        loop {
            match packet.as_mut().unwrap().read().await.unwrap() {
                server::Packet::HandShake(_) => {}
                server::Packet::Reboot(x) => {
                    // TODO pretend packet dropped
                    current_os = boot_path.get(&current_os).unwrap().get(&x).unwrap().clone();
                    sleep(Duration::from_secs(2)).await;
                    break;
                }
                server::Packet::InitId(x) => {
                    *os_storage.get_mut(&current_os).unwrap() = OSInfo { uid: x };
                }
                server::Packet::ShutDown => {
                    // reminder: first boot os
                    sleep(Duration::from_secs(2)).await;
                    loop {
                        let broadcast = UdpSocket::bind("0.0.0.0:0").await.unwrap();
                        broadcast.connect("255.255.255.255").await.unwrap();

                        let mut buf = vec![0_u8; magic_packet.len()];
                        let byte_read = broadcast.recv(&mut buf).await.unwrap();

                        if byte_read == buf.len()
                            && buf
                                .iter()
                                .zip(&magic_packet)
                                .filter(|&(a, b)| a != b)
                                .count()
                                == 0
                        {
                            break;
                        }
                    }
                    break;
                }
                server::Packet::GrubQuery => {
                    let list = boot_path
                        .get(&current_os)
                        .unwrap()
                        .iter()
                        .map(|(sec, _)| host::GrubInfo { grub_sec: *sec })
                        .collect();
                    packet
                        .as_mut()
                        .unwrap()
                        .send(host::Packet::GrubQuery(list))
                        .await
                        .unwrap();
                }
                server::Packet::Ping => todo!(),
                server::Packet::OSQuery => todo!(),
            }
        }
    }
}
