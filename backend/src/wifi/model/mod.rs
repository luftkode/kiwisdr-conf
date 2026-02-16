pub mod linux_ip_address;

use crate::wifi::error::WifiError;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    io,
    net::{Ipv4Addr, Ipv6Addr},
    ops::Deref,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiStatusResponse {
    interfaces: InterfaceMap,
    wifi_networks: Vec<WifiNetwork>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WifiConnectionPayload {
    uid: String,
    password: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct InterfaceMap(pub BTreeMap<InterfaceName, NetworkInterface>);

#[derive(Debug, Clone, Serialize)]
pub struct WifiNetwork {
    ssid: Option<String>,
    uid: String,
    state: WifiStatus,
    strength: Option<u8>,
    interface: Option<InterfaceName>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WifiStatus {
    Idle,
    Association,
    Configuration,
    Ready,
    Online,
    Disconnect,
    Failure,
}

/// Linux Network-Interface Name fx "eth0", "wlan0"
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterfaceName(String);

#[derive(Debug, Clone, Serialize)]
pub struct NetworkInterface {
    name: InterfaceName,
    ipv4: Vec<Ipv4Connection>,
    ipv6: Vec<Ipv6Connection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gateway<T> {
    None,
    Known(T),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Ipv4Connection {
    address: Ipv4Addr,
    prefix: u8,
    gateway: Gateway<Ipv4Addr>,
}

#[derive(Debug, Clone)]
pub struct Ipv6Connection {
    address: Ipv6Addr,
    prefix: u8,
    gateway: Gateway<Ipv6Addr>,
}

impl WifiConnectionPayload {
    pub fn uid(&self) -> &str {
        &self.uid
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

impl WifiStatusResponse {
    pub fn new(interfaces: InterfaceMap, wifi_networks: Vec<WifiNetwork>) -> Self {
        Self {
            interfaces,
            wifi_networks,
        }
    }
}

impl<T> Serialize for Gateway<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Gateway::None => serializer.serialize_str("none"),
            Gateway::Unknown => serializer.serialize_str("unknown"),
            Gateway::Known(value) => value.serialize(serializer),
        }
    }
}

impl InterfaceName {
    pub fn new(name: impl Into<String>) -> io::Result<Self> {
        let name = name.into();
        if name.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Interface name must not be empty",
            ))
        } else {
            Ok(InterfaceName(name))
        }
    }
}

impl Deref for InterfaceName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for InterfaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for InterfaceName {
    type Error = io::Error;

    fn try_from(s: String) -> io::Result<Self> {
        Self::new(s)
    }
}

impl TryFrom<&str> for InterfaceName {
    type Error = io::Error;

    fn try_from(s: &str) -> io::Result<Self> {
        Self::new(s)
    }
}

impl NetworkInterface {
    pub fn new(name: InterfaceName, ipv4: Vec<Ipv4Connection>, ipv6: Vec<Ipv6Connection>) -> Self {
        Self { name, ipv4, ipv6 }
    }
}

impl WifiNetwork {
    pub fn new(
        ssid: Option<String>,
        uid: String,
        state: WifiStatus,
        strength: Option<u8>,
        interface: Option<InterfaceName>,
    ) -> Self {
        Self {
            ssid,
            uid,
            state,
            strength,
            interface,
        }
    }

    pub fn ssid(&self) -> Option<&str> {
        self.ssid.as_deref()
    }

    pub fn uid(&self) -> &str {
        &self.uid
    }

    pub fn state(&self) -> WifiStatus {
        self.state
    }

    pub fn strength(&self) -> Option<u8> {
        self.strength
    }

    pub fn interface(&self) -> Option<&InterfaceName> {
        self.interface.as_ref()
    }
}

impl Ipv4Connection {
    pub fn new(address: Ipv4Addr, prefix: u8, gateway: Gateway<Ipv4Addr>) -> Self {
        Self {
            address,
            prefix,
            gateway,
        }
    }

    pub fn new_from_netmask(
        address: Ipv4Addr,
        netmask: Ipv4Addr,
        gateway: Gateway<Ipv4Addr>,
    ) -> io::Result<Self> {
        let prefix = Self::netmask_to_prefix(netmask)?;

        Ok(Self {
            address,
            prefix,
            gateway,
        })
    }

    fn netmask_to_prefix(netmask: Ipv4Addr) -> io::Result<u8> {
        let octets = netmask.octets();
        let mut prefix = 0;
        for &octet in &octets {
            let mut bits = octet;
            while bits & 0x80 != 0 {
                prefix += 1;
                bits <<= 1;
            }
            if bits != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Could not convert netmask {} to prefix", netmask),
                ));
            }
        }
        Ok(prefix)
    }

    pub fn address(&self) -> Ipv4Addr {
        self.address
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    pub fn gateway(&self) -> Gateway<Ipv4Addr> {
        self.gateway
    }

    pub fn cidr(&self) -> String {
        format!("{}/{}", self.address, self.prefix)
    }

    pub fn netmask(&self) -> Ipv4Addr {
        if self.prefix == 0 {
            return Ipv4Addr::new(0, 0, 0, 0);
        }

        let mask = u32::MAX << (32 - self.prefix);
        Ipv4Addr::from(mask)
    }
}

impl Ipv6Connection {
    pub fn new(address: Ipv6Addr, prefix: u8, gateway: Gateway<Ipv6Addr>) -> Self {
        Self {
            address,
            prefix,
            gateway,
        }
    }

    pub fn address(&self) -> Ipv6Addr {
        self.address
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    pub fn gateway(&self) -> Gateway<Ipv6Addr> {
        self.gateway
    }

    pub fn cidr(&self) -> String {
        format!("{}/{}", self.address, self.prefix)
    }
}

impl Serialize for Ipv4Connection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("Ipv4Connection", 2)?;
        s.serialize_field("address", &self.cidr())?;
        s.serialize_field("gateway", &self.gateway)?;
        s.end()
    }
}

impl Serialize for Ipv6Connection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("Ipv6Connection", 2)?;
        s.serialize_field("address", &self.cidr())?;
        s.serialize_field("gateway", &self.gateway)?;
        s.end()
    }
}

impl TryFrom<&str> for WifiStatus {
    type Error = WifiError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "idle" => Ok(Self::Idle),
            "association" => Ok(Self::Association),
            "configuration" => Ok(Self::Configuration),
            "ready" => Ok(Self::Ready),
            "online" => Ok(Self::Online),
            "disconnect" => Ok(Self::Disconnect),
            "failure" => Ok(Self::Failure),
            other => Err(WifiError::InvalidServiceState(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod ipv4 {
        use super::*;
        use serde_json::json;
        use std::net::Ipv4Addr;

        #[test]
        fn host_cidr_is_correct() {
            let ipv4 = Ipv4Connection {
                address: Ipv4Addr::new(192, 168, 1, 42),
                prefix: 24,
                gateway: Gateway::Known(Ipv4Addr::new(192, 168, 1, 1)),
            };

            assert_eq!(ipv4.cidr(), "192.168.1.42/24");
        }

        #[test]
        fn netmask_is_correct() {
            let ipv4 = Ipv4Connection {
                address: Ipv4Addr::new(10, 0, 0, 5),
                prefix: 16,
                gateway: Gateway::Unknown,
            };

            assert_eq!(ipv4.netmask(), Ipv4Addr::new(255, 255, 0, 0));
        }

        #[test]
        fn serialization_shape_is_stable() {
            let ipv4 = Ipv4Connection {
                address: Ipv4Addr::new(192, 168, 0, 10),
                prefix: 16,
                gateway: Gateway::Known(Ipv4Addr::new(192, 168, 0, 1)),
            };

            let value = serde_json::to_value(&ipv4).unwrap();

            assert_eq!(
                value,
                json!({
                    "address": "192.168.0.10/16",
                    "gateway": "192.168.0.1"
                })
            );
        }
    }

    mod ipv6 {
        use super::*;
        use serde_json::json;
        use std::net::Ipv6Addr;

        #[test]
        fn host_cidr_is_correct() {
            let ipv6 = Ipv6Connection {
                address: Ipv6Addr::new(
                    0x2001, 0x0db8, 0x0000, 0x0000, 0x0000, 0x0000, 0xdead, 0xbeef,
                ),
                prefix: 64,
                gateway: Gateway::None,
            };

            assert_eq!(ipv6.cidr(), "2001:db8::dead:beef/64");
        }

        #[test]
        fn serialization_shape_is_stable() {
            let ipv6 = Ipv6Connection {
                address: Ipv6Addr::new(
                    0xfe80, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0001,
                ),
                prefix: 64,
                gateway: Gateway::Known(Ipv6Addr::new(
                    0xfe80, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x00ff,
                )),
            };

            let value = serde_json::to_value(&ipv6).unwrap();

            assert_eq!(
                value,
                json!({
                    "address": "fe80::1/64",
                    "gateway": "fe80::ff"
                })
            );
        }
    }

    mod service_state {
        use super::*;
        #[test]
        fn parses_valid_states() {
            assert_eq!(WifiStatus::try_from("online").unwrap(), WifiStatus::Online);

            assert_eq!(WifiStatus::try_from("idle").unwrap(), WifiStatus::Idle);
        }

        #[test]
        fn rejects_invalid_state() {
            let err = WifiStatus::try_from("nonsense").unwrap_err();

            match err {
                WifiError::InvalidServiceState(s) => assert_eq!(s, "nonsense"),
                _ => panic!("wrong error variant"),
            }
        }
    }
}
