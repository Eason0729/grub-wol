use async_std::net;
use proto::prelude::{host, server, GrubId, APIVERSION, ID, PROTO_IDENT, SERVER_PORT};
use rand::Rng;

use super::transfer::TcpConn;

const OS_VARIETY: usize = 3;

type Conn = TcpConn<host::Packet, server::Packet>;

pub struct MachineInfo {
    pub current_os: usize,
    pub packet: Option<Conn>,
    mac_address: [u8; 6],
    oss: Vec<OsInfo>,
}

impl MachineInfo {
    pub fn new() -> Self {
        let mut oss = Vec::new();
        oss.push(OsInfo {
            uid: 0,
            display_name: "Ubuntu".to_owned(),
            grub_path: (0..OS_VARIETY)
                .map(|_| Some(rand::thread_rng().gen()))
                .collect::<Vec<Option<GrubId>>>()
                .try_into()
                .unwrap(),
        });
        oss.push(OsInfo {
            uid: 0,
            display_name: "Debian".to_owned(),
            grub_path: (0..OS_VARIETY)
                .map(|_| Some(rand::thread_rng().gen()))
                .collect::<Vec<Option<GrubId>>>()
                .try_into()
                .unwrap(),
        });
        oss.push(OsInfo {
            uid: 0,
            display_name: "Windows".to_owned(),
            grub_path: (0..OS_VARIETY)
                .map(|_| None)
                .collect::<Vec<Option<GrubId>>>()
                .try_into()
                .unwrap(),
        });
        Self {
            current_os: 0,
            packet: None,
            oss,
            mac_address: rand::thread_rng().gen(),
        }
    }
    pub fn os(&self) -> &OsInfo {
        &self.oss[self.current_os]
    }
    pub fn os_mut(&mut self) -> &mut OsInfo {
        &mut self.oss[self.current_os]
    }
    pub fn boot_by(&mut self, grub_sec: GrubId) -> host::Packet {
        match self.os().grub_path.iter().enumerate().find_map(|(i, &x)| {
            if x == Some(grub_sec) {
                Some(i)
            } else {
                None
            }
        }) {
            Some(i) => {
                self.current_os = i;
            }
            None => log::error!("Cannot boot into specific os: invaild grub_sec"),
        }
        host::Packet::Reboot
    }
    pub async fn close(&mut self) {
        if self.packet.is_none() {
            log::error!("Packet already closed");
        }
        let mut packet = self.packet.take().unwrap();
        packet.flush().await.ok();
    }
    pub async fn connect(&mut self) {
        if self.packet.is_some() {
            log::error!("Packet already connected");
        }

        let handshake = host::Handshake {
            ident: PROTO_IDENT,
            mac_address: self.mac_address,
            uid: self.os().uid,
            version: APIVERSION,
        };

        let conn = Some(
            net::TcpStream::connect(format!("127.0.0.1:{}", SERVER_PORT))
                .await
                .unwrap(),
        );
        self.packet = Some(Conn::from_tcp(conn.unwrap()));
        self.packet
            .as_mut()
            .unwrap()
            .send(proto::prelude::host::Packet::Handshake(handshake))
            .await
            .unwrap();
    }
    pub fn conn(&mut self) -> &mut Conn {
        self.packet.as_mut().unwrap()
    }
}

pub struct OsInfo {
    uid: ID,
    display_name: String,
    grub_path: [Option<GrubId>; OS_VARIETY],
}

impl OsInfo {
    pub fn respond_grub(&self) -> host::Packet {
        host::Packet::GrubQuery(
            self.grub_path
                .iter()
                .filter_map(|x| x.as_ref())
                .map(|&grub_sec| host::GrubInfo { grub_sec })
                .collect(),
        )
    }
    pub fn respond_os(&self) -> host::Packet {
        host::Packet::OsQuery(host::OsQuery {
            display_name: self.display_name.clone(),
        })
    }
    pub fn change_uid(&mut self, uid: ID) -> host::Packet {
        self.uid = uid;
        host::Packet::InitId
    }
    pub fn name(&self) -> host::Packet {
        host::Packet::OsQuery(host::OsQuery {
            display_name: self.display_name.clone(),
        })
    }
}
