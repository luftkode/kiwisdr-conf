use crate::wifi::error::WifiError;
use serde::Serialize;
use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr},
};

#[derive(Debug, Clone, Serialize)]
pub struct ServiceState {
    ssid: Option<String>,
    uid: String,
    state: ServiceStateKind,
    strength: Option<u8>,
    ipv4: Option<Ipv4Connection>,
    ipv6: Option<Ipv6Connection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStateKind {
    Idle,
    Association,
    Configuration,
    Ready,
    Online,
    Disconnect,
    Failure,
}

#[derive(Debug, Clone)]
pub struct Ipv4Connection {
    address: Ipv4Addr,
    prefix: u8,
    gateway: Ipv4Addr,
}

#[derive(Debug, Clone)]
pub struct Ipv6Connection {
    address: Ipv6Addr,
    prefix: u8,
    gateway: Option<Ipv6Addr>,
}

impl ServiceState {
    pub fn new(
        ssid: Option<String>,
        uid: String,
        state: ServiceStateKind,
        strength: Option<u8>,
        ipv4: Option<Ipv4Connection>,
        ipv6: Option<Ipv6Connection>,
    ) -> Self {
        Self {
            ssid,
            uid,
            state,
            strength,
            ipv4,
            ipv6,
        }
    }

    pub fn ssid(&self) -> Option<&str> {
        self.ssid.as_deref()
    }

    pub fn uid(&self) -> &str {
        &self.uid
    }

    pub fn state(&self) -> ServiceStateKind {
        self.state
    }

    pub fn strength(&self) -> Option<u8> {
        self.strength
    }

    pub fn ipv4(&self) -> Option<&Ipv4Connection> {
        self.ipv4.as_ref()
    }

    pub fn ipv6(&self) -> Option<&Ipv6Connection> {
        self.ipv6.as_ref()
    }
}

impl Ipv4Connection {
    pub fn new(address: Ipv4Addr, prefix: u8, gateway: Ipv4Addr) -> Self {
        Self {
            address,
            prefix,
            gateway,
        }
    }

    pub fn new_from_netmask(
        address: Ipv4Addr,
        netmask: Ipv4Addr,
        gateway: Ipv4Addr,
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

    pub fn gateway(&self) -> Ipv4Addr {
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
    pub fn new(address: Ipv6Addr, prefix: u8, gateway: Option<Ipv6Addr>) -> Self {
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

    pub fn gateway(&self) -> Option<Ipv6Addr> {
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

impl TryFrom<&str> for ServiceStateKind {
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
                gateway: Ipv4Addr::new(192, 168, 1, 1),
            };

            assert_eq!(ipv4.cidr(), "192.168.1.42/24");
        }

        #[test]
        fn netmask_is_correct() {
            let ipv4 = Ipv4Connection {
                address: Ipv4Addr::new(10, 0, 0, 5),
                prefix: 16,
                gateway: Ipv4Addr::new(10, 0, 0, 1),
            };

            assert_eq!(ipv4.netmask(), Ipv4Addr::new(255, 255, 0, 0));
        }

        #[test]
        fn serialization_shape_is_stable() {
            let ipv4 = Ipv4Connection {
                address: Ipv4Addr::new(192, 168, 0, 10),
                prefix: 16,
                gateway: Ipv4Addr::new(192, 168, 0, 1),
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
                gateway: None,
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
                gateway: Some(Ipv6Addr::new(
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
            assert_eq!(
                ServiceStateKind::try_from("online").unwrap(),
                ServiceStateKind::Online
            );

            assert_eq!(
                ServiceStateKind::try_from("idle").unwrap(),
                ServiceStateKind::Idle
            );
        }

        #[test]
        fn rejects_invalid_state() {
            let err = ServiceStateKind::try_from("nonsense").unwrap_err();

            match err {
                WifiError::InvalidServiceState(s) => assert_eq!(s, "nonsense"),
                _ => panic!("wrong error variant"),
            }
        }
    }
}
