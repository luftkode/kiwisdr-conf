//! Parse the output of `ip -j a`

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressFamily {
    Inet,
    Inet6,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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


#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
    pub txqlen: u32,

    #[serde(rename = "link_type")]
    pub link_type: LinkType,

    /// MAC address (kept as string to avoid kernel-specific formats)
    pub address: String,

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
}
