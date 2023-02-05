// dummy client
pub mod state;

use async_std::task::sleep;
use proto::prelude::*;
use std::time::Duration;

#[async_std::test]
async fn test_main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    let mut state = state::MachineInfo::new();
    state.connect().await;
    loop {
        let res = match state.conn().read().await.unwrap() {
            server::Packet::Handshake(_) => {
                continue;
            }
            server::Packet::Reboot(x) => state.boot_by(x),
            server::Packet::InitId(x) => {
                log::info!("recieve InitId of {}", x);
                state.os_mut().change_uid(x)
            }
            server::Packet::ShutDown => {
                state.conn().send(host::Packet::ShutDown).await.unwrap();
                state.close();
                sleep(Duration::from_secs(3)).await;
                state.connect().await;
                continue;
            }
            server::Packet::GrubQuery => state.os().respond_query(),
            server::Packet::OsQuery => state.os().respond_query(),
            server::Packet::Ping => todo!(),
        };
        state.conn().send(res).await.unwrap();
    }
}
