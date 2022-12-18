pub const SERVER_PORT: u16 = 10870;
pub const SERVICE_TYPE: &str = "_grubwol._udp.local.";
pub(super) type APIVersionType = u64;
pub const APIVERSION: APIVersionType = 4;
pub type GrubId = u64;
pub type ID = u64;
pub type Integer = i64;
pub type PacketPrefix = u64;
pub type ProtoIdentType = [u8; 32];
pub const PROTO_IDENT: ProtoIdentType = [
    148, 5, 15, 226, 189, 18, 191, 45, 95, 39, 31, 36, 225, 208, 182, 27, 230, 132, 13, 153, 104,
    19, 247, 46, 67, 194, 71, 79, 147, 85, 109, 79,
];
