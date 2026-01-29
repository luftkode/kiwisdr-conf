mod consts {
    /// D-Bus well-known name owned by the ConnMan daemon
    pub const CONNMAN_DEST: &str = "net.connman";

    /// Root object path of ConnMan
    pub const CONNMAN_ROOT_PATH: &str = "/net/connman";

    /// ConnMan manager interface (global operations)
    pub const CONNMAN_MANAGER_IFACE: &str = "net.connman.Manager";

    /// ConnMan service interface (Wi-Fi, Ethernet, etc.)
    pub const CONNMAN_SERVICE_IFACE: &str = "net.connman.Service";

    /// ConnMan technology interface (wifi, ethernet, p2p)
    pub const CONNMAN_TECH_IFACE: &str = "net.connman.Technology";

    /// Standard D-Bus properties interface
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

/// Thin connman dbus wrapper
/// Acts like connmanctl
mod client {
    use super::error::Result;
    use super::consts::*;
    use zbus::{Connection, Proxy};
    use zvariant::{Value, OwnedObjectPath};

    type DBusDict = HashMap<String, Value>;
    
    pub async fn manager_proxy(conn: &Connection) -> Result<Proxy<'_>> {
        Ok(
            Proxy::new(
                conn,
                CONNMAN_DEST,
                CONNMAN_ROOT_PATH,
                CONNMAN_MANAGER_IFACE,
            )
            .await?
        )
    }

    pub async fn services(conn: &Connection) -> Result<Vec<(OwnedObjectPath, DBusDict)>> {

    }

    pub async fn technologies(conn: &Connection) -> Result<Vec<(OwnedObjectPath, DBusDict)>> {
        
    }

    pub async fn service_connect(conn: &Connection, service_path: &str) -> Result<()> {
        
    }

    pub async fn service_disconnect(conn: &Connection, service_path: &str) -> Result<()> {
        
    }

    pub async fn service_config(conn: &Connection, service_path: &str, key: &str, value: Value) -> Result<()> {
        
    }

    pub async fn service_remove(conn: &Connection, service_path: &str) -> Result<()> {
        
    }

    pub async fn service_state(conn: &Connection, service_path: &str) -> Result<Value> {
        
    }

    pub async fn service_show(conn: &Connection, service_path: &str) -> Result<DBusDict> {
        
    }

    pub async fn technology_enable(conn: &Connection, technology_path: &str) -> Result<()> {
        
    }

    pub async fn technology_disable(conn: &Connection, technology_path: &str) -> Result<()> {
        
    }

    pub async fn technology_scan(conn: &Connection, technology_path: &str) -> Result<()> {
        
    }

    pub async fn technology_show(conn: &Connection, technology_path: &str) -> Result<DBusDict> {
        
    }

    pub async fn technology_tether(conn: &Connection, technology_path: &str, enabled: bool) -> Result<()> {
        
    }

    pub async fn technology_set(conn: &Connection, technology_path: &str, key: &str, value: Value) -> Result<()> {
        
    }
}

/// Translates dbus values into `ServiceState`
mod translation {
    
}

pub struct ConnManConnection {
    connection: zbus::Connection,
}

impl ConnManConnection {
    /// Connects to the system D-Bus.
    ///
    /// This does not talk to ConnMan yet — it only opens the bus.
    pub async fn new() -> Result<Self> {
        let connection = zbus::Connection::system().await?;
        Ok(Self { connection })
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