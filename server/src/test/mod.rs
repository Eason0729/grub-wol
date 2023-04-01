// dummy client
pub mod state;
pub mod transfer;

use async_std::task::sleep;
use proto::prelude::*;
use std::time::Duration;

#[ignore]
#[async_std::test]
async fn test_main() {
    env_logger::builder().filter_level(log::LevelFilter::Debug).try_init().unwrap();
    
    let mut state = state::MachineInfo::new();
    state.connect().await;
    loop {
        let req = state.conn().read().await.unwrap();
        log::debug!("recieived {:?}", req);
        let res = match req {
            server::Packet::Handshake(_) => {
                log::trace!("recieived callback Handshake");
                continue;
            }
            server::Packet::Reboot(x) => {
                let res = state.boot_by(x);
                state.conn().send(res).await.unwrap();
                state.conn().flush().await.unwrap();
                state.close().await;
                sleep(Duration::from_secs(1)).await;
                state.connect().await;
                continue;
            }
            server::Packet::InitId(x) => {
                log::info!("recieve InitId of {}", x);
                state.os_mut().change_uid(x)
            }
            server::Packet::Shutdown => {
                state.conn().send(host::Packet::Shutdown).await.unwrap();
                state.conn().flush().await.unwrap();
                state.close().await;
                state.current_os=1;
                sleep(Duration::from_secs(1)).await;
                state.connect().await;
                continue;
            }
            server::Packet::GrubQuery => state.os().respond_grub(),
            server::Packet::OsQuery => state.os().respond_os(),
            server::Packet::Ping => todo!(),
        };
        state.conn().send(res).await.unwrap();
    }
}
