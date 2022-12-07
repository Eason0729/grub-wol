use proto::prelude as protocal;

struct Packets {}

struct Packet<'a> {
    manager: &'a mut Packets,
    conn: Option<protocal::Conn>,
}

impl<'a> Packet<'a> {
    async fn reconnect() {}
}

enum Error {
    ClientOffline,
}
