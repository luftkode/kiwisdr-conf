use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::net::IpAddr;

// Address family (inet/inet6)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressFamily {
    Inet,
    Inet6,
}

// Interface operational state
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OperState {
    Unknown,
    Up,
    Down,
    Dormant,
    LowerLayerDown,
    Testing,
    Other(String), // future-proofing
}

// Link type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Ether,
    Loopback,
    Can,
    Dummy,
    Other(String),
}

// Individual IP address info
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AddrInfo {
    pub family: AddressFamily,
    pub local: IpAddr,
    #[serde(default)]
    pub prefixlen: u8,
    #[serde(default)]
    pub broadcast: Option<IpAddr>,
    pub scope: String,
    pub label: String,
    pub valid_life_time: u32,
    pub preferred_life_time: u32,
}

// Interface flags
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InterfaceFlag {
    Up,
    Broadcast,
    Loopback,
    PointToPoint,
    NoArp,
    Dynamic,
    Multicast,
    LowerUp,
    Echo,
    Other(String),
}

// Individual network interface
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Interface {
    pub ifindex: u32,
    pub ifname: String,
    pub flags: HashSet<InterfaceFlag>,
    pub mtu: u32,
    pub qdisc: String,
    pub operstate: OperState,
    pub group: String,
    pub txqlen: u32,
    #[serde(rename = "link_type")]
    pub link_type: LinkType,
    pub address: String,
    pub broadcast: String,
    #[serde(default)]
    pub addr_info: Vec<AddrInfo>,
}

// Root type containing all interfaces, keyed by name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpOutput {
    pub interfaces: BTreeMap<String, Interface>,
}

impl IpOutput {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let vec: Vec<Interface> = serde_json::from_str(json)?;
        let map = vec
            .into_iter()
            .map(|iface| (iface.ifname.clone(), iface))
            .collect();
        Ok(IpOutput { interfaces: map })
    }
}
