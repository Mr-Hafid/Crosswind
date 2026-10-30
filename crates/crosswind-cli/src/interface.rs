use std::io;
use std::net::Ipv6Addr;

use crosswind_core::Platform;

pub struct LinkInterface {
    pub name: String,
    pub index: u32,
    pub link_local_addr: Ipv6Addr,
}

impl LinkInterface {
    pub fn lookup(name: &str) -> io::Result<Self> {
        let network_interface = netdev::get_interfaces()
            .into_iter()
            .find(|candidate| candidate.name == name)
            .ok_or_else(|| io::Error::other(missing_interface_hint(name)))?;
        let link_local_addr = network_interface
            .ipv6
            .iter()
            .map(|network| network.addr())
            .find(|ip| is_link_local(*ip))
            .ok_or_else(|| io::Error::other(format!("{name} has no IPv6 link-local address")))?;
        Ok(LinkInterface {
            name: name.to_owned(),
            index: network_interface.index,
            link_local_addr,
        })
    }
}

pub fn default_interface_name() -> &'static str {
    match Platform::current() {
        Platform::MacOs | Platform::Linux => "awdl0",
        Platform::Windows | Platform::Android | Platform::Other => "",
    }
}

fn is_link_local(ip: Ipv6Addr) -> bool {
    ip.segments()[0] & 0xffc0 == 0xfe80
}

fn missing_interface_hint(name: &str) -> String {
    match (name, Platform::current()) {
        ("", Platform::Windows) => {
            "Windows has no AWDL interface; pass --iface with a network adapter name to test against another Crosswind on the same link".to_owned()
        }
        ("", _) => "no interface given; pass --iface".to_owned(),
        ("awdl0", Platform::Linux) => {
            "awdl0 not found; bring the radio up first with `crosswind-linux radio up`".to_owned()
        }
        _ => format!("interface {name} not found"),
    }
}
