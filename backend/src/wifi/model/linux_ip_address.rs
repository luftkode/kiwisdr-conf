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
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Ether,
    Loopback,
    Can,
    Dummy,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AddrInfo {
    pub family: AddressFamily,
    pub local: IpAddr,
    pub prefixlen: u8,

    #[serde(default)]
    pub broadcast: Option<IpAddr>,

    pub scope: String,

    #[serde(default)]
    pub label: Option<String>,

    pub valid_life_time: u32,
    pub preferred_life_time: u32,
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

    /// MAC address; left as string on purpose
    pub address: String,

    #[serde(default)]
    pub broadcast: Option<String>,

    #[serde(default)]
    pub addr_info: Vec<AddrInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpOutput {
    pub interfaces: BTreeMap<String, Interface>,
}

impl IpOutput {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let list: Vec<Interface> = serde_json::from_str(json)?;
        let interfaces = list
            .into_iter()
            .map(|iface| (iface.ifname.clone(), iface))
            .collect();

        Ok(Self { interfaces })
    }
}
