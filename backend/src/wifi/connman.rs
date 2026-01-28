mod consts {
    pub const CONNMAN_PATH: &str = "/net/connman";
    pub const CONNMAN_IFACE: &str = "net.connman";
    pub const CONNMAN_SERVICE_IFACE: &str = "net.connman.Service";
    pub const DBUS_PROPERTIES_IFACE: &str = "org.freedesktop.DBus.Properties";

    pub const PROP_STATE: &str = "State";
    pub const PROP_STRENGTH: &str = "Strength";
    pub const PROP_IPV4: &str = "IPv4";
    pub const PROP_IPV6: &str = "IPv6";

    pub const IP_ADDRESS: &str = "Address";
    pub const IP_GATEWAY: &str = "Gateway";
    pub const IP_PREFIX: &str = "Prefix";
}

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum ConnManError {
        #[error("DBus error: {0}")]
        DBus(#[from] zbus::Error),

        #[error("Missing property: {0}")]
        MissingProperty(&'static str),

        #[error("Invalid property type: {0}")]
        InvalidProperty(&'static str),

        #[error("Invalid IP address in property: {0}")]
        InvalidAddress(&'static str),
    }

    pub type Result<T> = std::result::Result<T, ConnManError>;
}

use std::net::{Ipv4Addr, Ipv6Addr};
use crate::wifi::{Wifi, error::WifiError, model::{Ipv4Connection, Ipv6Connection, ServiceState, ServiceStateKind}};
use error::{ConnManError, Result};

/// Thin connman wrapper
mod client {
    todo!();
}

/// Translates dbus values into `ServiceState`
mod translation {
    todo!();
}

pub struct ConnManConnection {
    connection: zbus::Connection,
}

impl ConnManConnection {
    fn new() -> Self {
        todo!()
    }
}

impl Wifi for ConnManConnection {
    async fn get_available(&self) -> WifiResult<Vec<ServiceState>> {
        todo!();
    }

    async fn connect(&self, wifi_uid: &str, passphrase: Option<&str>) -> WifiResult<()> {
        todo!();
    }

    async fn disconnect(&self, wifi_uid: &str) -> WifiResult<()> {
        todo!();
    }
}