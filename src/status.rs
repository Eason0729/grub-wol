use std::{collections::LinkedList, net};

pub const SERVER_PORT: u16 = 10870;

pub const SERVICE_TYPE: &str = "_grubwol._udp.local.";

pub type ID = u64;

pub struct MachineStatus {
    ip_address: net::Ipv4Addr,
    events: LinkedList<ProgressStatus>,
}

pub struct ProgressStatus {
    condition: Vec<(Condition, Event)>,
    timeout: usize,
}

pub struct OperatingSystem {
    id: ID,
}

enum Condition {
    None,
    ResponseFromOs(OperatingSystem),
    Ping, // I.E Response from any os
}

enum Event {
    Shutdown,
    Reboot,
    None,
}
