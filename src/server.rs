use crate::status::*;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

struct Server {
    id: ID,
}

impl Server {
    // fn publish_dnssd(&self) {
    //     let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    //     let service_type = SERVICE_TYPE;
    //     let instance_name = "my_instance";
    //     let host_ipv4: &str = &match local_ip().unwrap() {
    //         std::net::IpAddr::V4(x) => x.to_string(),
    //         std::net::IpAddr::V6(_) => todo!(),
    //     };
    //     let host_name: &str = &format!("{}.local.", host_ipv4);
    //     let port = SERVER_PORT;
    //     let mut properties = HashMap::new();
    //     properties.insert("id".to_string(), self.id.to_string());

    //     let my_service = ServiceInfo::new(
    //         service_type,
    //         instance_name,
    //         host_name,
    //         host_ipv4,
    //         port,
    //         Some(properties),
    //     )
    //     .unwrap();

    //     mdns.register(my_service)
    //         .expect("Failed to register our service");
    // }
}

struct ClientStatus {}
