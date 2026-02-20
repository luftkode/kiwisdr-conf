//! Parse the output of `ip -j a`

use super::{
    Gateway, InterfaceMap, InterfaceName, Ipv4Connection, Ipv6Connection, NetworkInterface,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::net::IpAddr;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressFamily {
    Inet,
    Inet6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OperState {
    Up,
    Down,
    Unknown,
    Dormant,
    Testing,
    #[serde(rename = "LOWERLAYERDOWN")]
    LowerLayerDown,

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Ether,
    Loopback,
    Can,
    Dummy,
    Bridge,
    Vlan,
    Tun,

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InterfaceFlag {
    Up,
    Broadcast,
    Loopback,
    PointToPoint,
    Noarp,
    Dynamic,
    Multicast,
    LowerUp,
    Echo,

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressScope {
    Host,
    Link,
    Global,

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Lifetime(u32);

impl Lifetime {
    pub const FOREVER: u32 = u32::MAX;

    pub fn is_forever(self) -> bool {
        self.0 == Self::FOREVER
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AddrInfo {
    pub family: AddressFamily,
    pub local: IpAddr,
    pub prefixlen: u8,

    #[serde(default)]
    pub broadcast: Option<IpAddr>,

    pub scope: AddressScope,

    #[serde(default)]
    pub label: Option<String>,

    pub valid_life_time: Lifetime,
    pub preferred_life_time: Lifetime,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Interface {
    pub ifindex: u32,
    pub ifname: String,

    pub flags: BTreeSet<InterfaceFlag>,

    pub mtu: u32,
    pub qdisc: String,
    pub operstate: OperState,
    pub group: String,

    #[serde(default)]
    pub txqlen: Option<u32>,

    #[serde(rename = "link_type")]
    pub link_type: LinkType,

    /// MAC address (kept as `String` to avoid kernel-specific formats and as `Option` to not crash (it gets filtered later))
    pub address: Option<String>,

    #[serde(default)]
    pub broadcast: Option<String>,

    #[serde(default)]
    pub addr_info: Vec<AddrInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "Vec<Interface>")]
pub struct IpOutput {
    pub interfaces: BTreeMap<String, Interface>,
}

impl From<Vec<Interface>> for IpOutput {
    fn from(list: Vec<Interface>) -> Self {
        let interfaces = list
            .into_iter()
            .map(|iface| (iface.ifname.clone(), iface))
            .collect();

        Self { interfaces }
    }
}

impl IpOutput {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub async fn from_system() -> io::Result<Self> {
        let output = Command::new("ip").args(["-j", "address"]).output().await?;

        if !output.status.success() {
            return Err(io::Error::other(format!(
                "'ip -j address' exited with (exit_code, stdout, stderr) ({}, {}, {})",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let value = serde_json::from_slice(&output.stdout).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("JSON parse error: {}", e),
            )
        })?;

        Ok(value)
    }
}

impl From<IpOutput> for InterfaceMap {
    fn from(ip: IpOutput) -> Self {
        let mut out = BTreeMap::new();

        for (name, iface) in ip.interfaces {
            let ifname = match InterfaceName::try_from(name) {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Only keep Ethernet/WiFi-class devices
            if iface.link_type != LinkType::Ether {
                continue;
            }

            // Must have a MAC address
            if iface.address.is_none() {
                continue;
            }

            // Must not be kernel-virtual by naming convention
            let name = iface.ifname.as_str();

            if name.starts_with("docker")
                || name.starts_with("br-")
                || name.starts_with("veth")
                || name.starts_with("dummy")
                || name.starts_with("tun")
                || name.starts_with("tap")
                || name.starts_with("lo")
            {
                continue;
            }

            let mut ipv4 = Vec::new();
            let mut ipv6 = Vec::new();

            for addr in iface.addr_info {
                match addr.local {
                    std::net::IpAddr::V4(v4) => {
                        ipv4.push(Ipv4Connection::new(v4, addr.prefixlen, Gateway::Unknown));
                    }
                    std::net::IpAddr::V6(v6) => {
                        ipv6.push(Ipv6Connection::new(v6, addr.prefixlen, Gateway::Unknown));
                    }
                }
            }

            out.insert(ifname.clone(), NetworkInterface::new(ifname, ipv4, ipv6));
        }

        InterfaceMap(out)
    }
}
