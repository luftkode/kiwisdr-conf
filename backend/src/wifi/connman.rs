#![allow(unused)]

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
use crate::wifi::{Wifi, error::{WifiError, Result as WifiResult}, model::{Ipv4Connection, Ipv6Connection, ServiceState, ServiceStateKind}};
use error::{ConnManError, Result};

/// Thin connman dbus wrapper
/// Acts like connmanctl
mod client {
    use super::error::Result;
    use super::consts::*;
    use std::collections::HashMap;
    use zbus::{Connection, Proxy};
    use zvariant::{OwnedObjectPath, OwnedValue, Value};

    type DBusDict = HashMap<String, OwnedValue>;
    
    async fn manager_proxy(conn: &Connection) -> Result<Proxy<'_>> {
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

    async fn service_proxy<'a>(
        conn: &'a Connection,
        service_path: &'a OwnedObjectPath,
    ) -> Result<Proxy<'a>> {
        Ok(
            Proxy::new(
                conn,
                CONNMAN_DEST,
                service_path,
                CONNMAN_SERVICE_IFACE,
            )
            .await?
        )
    }

    async fn technology_proxy<'a>(
        conn: &'a Connection,
        technology_path: &'a OwnedObjectPath,
    ) -> Result<Proxy<'a>> {
        Ok(
            Proxy::new(
                conn,
                CONNMAN_DEST,
                technology_path,
                CONNMAN_TECH_IFACE,
            )
            .await?
        )
    }

    pub async fn services(conn: &Connection) -> Result<Vec<(OwnedObjectPath, DBusDict)>> {
        todo!()
    }

    /// List all ConnMan technologies.
    ///
    /// This is a thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   / \
    ///   net.connman.Manager.GetTechnologies
    /// ```
    ///
    /// Each entry contains:
    /// - the object path of the technology (e.g. `/net/connman/technology/wifi`)
    /// - a property dictionary as returned by ConnMan
    ///
    /// No interpretation or validation is performed here.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the system D-Bus is unavailable
    /// - ConnMan is not running
    /// - the reply does not match `a(oa{sv})`
    pub async fn technologies(conn: &Connection) -> Result<Vec<(OwnedObjectPath, DBusDict)>> {
        let proxy = manager_proxy(conn).await?;

        // ConnMan Manager.GetTechnologies → a(oa{sv})
        let technologies: Vec<(OwnedObjectPath, DBusDict)> =
            proxy.call("GetTechnologies", &()).await?;

        Ok(technologies)
    }

    /// Connect to a ConnMan service (Wi-Fi, Ethernet, etc.).
    ///
    /// Thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/service/<wifi_uid> \
    ///   net.connman.Service.Connect
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the service path is invalid
    /// - D-Bus fails
    /// - ConnMan rejects the request
    pub async fn service_connect(
        conn: &Connection,
        service_path: &OwnedObjectPath,
    ) -> Result<()> {
        let proxy = service_proxy(conn, service_path).await?;

        // Connect has no arguments and no return value
        proxy.call::<&str, (), ()>("Connect", &()).await?;

        Ok(())
    }

    /// Disconnect from a ConnMan service (Wi-Fi, Ethernet, etc.).
    ///
    /// Thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/service/<wifi_uid> \
    ///   net.connman.Service.Disconnect
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the service path is invalid
    /// - D-Bus fails
    /// - ConnMan rejects the request
    pub async fn service_disconnect(
        conn: &Connection,
        service_path: &OwnedObjectPath,
    ) -> Result<()> {
        let proxy = service_proxy(conn, service_path).await?;

        // Disconnect has no arguments and no return value
        proxy.call::<&str, (), ()>("Disconnect", &()).await?;

        Ok(())
    }

    pub async fn service_config(
        conn: &Connection,
        service_path: &OwnedObjectPath,
        key: &str,
        value: OwnedValue,
    ) -> Result<()> {
        todo!()
    }

    pub async fn service_remove(conn: &Connection, service_path: &OwnedObjectPath) -> Result<()> {
        todo!()
    }

    pub async fn service_state(conn: &Connection, service_path: &OwnedObjectPath) -> Result<OwnedValue> {
        todo!()
    }

    /// Get the properties of a ConnMan service (Wi-Fi, Ethernet, etc.).
    ///
    /// Thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/service/<wifi_uid> \
    ///   net.connman.Service.GetProperties
    /// ```
    ///
    /// Returns a dictionary mapping property names to zvariant values (`a{sv}`).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the service path is invalid
    /// - D-Bus communication fails
    /// - ConnMan rejects the request
    pub async fn service_properties(
        conn: &Connection,
        service_path: &OwnedObjectPath,
    ) -> Result<DBusDict> {
        let proxy = service_proxy(conn, service_path).await?;

        // GetProperties → a{sv}
        let props: DBusDict = proxy.call("GetProperties", &()).await?;
        Ok(props)
    }

    /// Enable a ConnMan technology (Wi-Fi, Ethernet, etc.).
    ///
    /// Thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/technology/wifi \
    ///   net.connman.Technology.SetProperty \
    ///   string:"Powered" variant:boolean:true
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the technology is invalid
    /// - D-Bus fails
    /// - ConnMan rejects the request
    pub async fn technology_enable(
        conn: &Connection,
        technology_path: &OwnedObjectPath,
    ) -> Result<()> {
        let proxy = technology_proxy(conn, technology_path).await?;
        // SetProperty("Powered", true)
        proxy
            .call::<&str, (&str, Value), ()>(
                "SetProperty",
                &("Powered", Value::from(true)),
            )
            .await?;
        Ok(())
    }

    /// Disable a ConnMan technology (Wi-Fi, Ethernet, etc.).
    ///
    /// Thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/technology/wifi \
    ///   net.connman.Technology.SetProperty \
    ///   string:"Powered" variant:boolean:false
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the technology is invalid
    /// - D-Bus fails
    /// - ConnMan rejects the request
    pub async fn technology_disable(
        conn: &Connection,
        technology_path: &OwnedObjectPath,
    ) -> Result<()> {
        let proxy = technology_proxy(conn, technology_path).await?;
        // SetProperty("Powered", false)
        proxy
            .call::<&str, (&str, Value), ()>(
                "SetProperty",
                &("Powered", Value::from(false)),
            )
            .await?;
        Ok(())
    }

    /// Trigger a scan on a ConnMan technology (e.g. Wi-Fi).
    ///
    /// This is a thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/technology/wifi \
    ///   net.connman.Technology.Scan
    /// ```
    ///
    /// The method takes no arguments and returns no value.
    /// A successful call merely indicates that the scan was
    /// accepted by ConnMan.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the technology is powered off
    /// - ConnMan rejects the request
    /// - D-Bus communication fails
    pub async fn technology_scan(
        conn: &Connection,
        technology_path: &OwnedObjectPath,
    ) -> Result<()> {
        let proxy = technology_proxy(conn, technology_path).await?;

        // net.connman.Technology.Scan has no args and no reply body
        proxy.call::<&str, (), ()>("Scan", &()).await?;
        Ok(())
    }

    /// Get the properties of a ConnMan technology (Wi-Fi, Ethernet, etc.).
    ///
    /// Thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/technology/wifi \
    ///   net.connman.Technology.GetProperties
    /// ```
    ///
    /// Returns a dictionary mapping property names to zvariant values (`a{sv}`).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the technology path is invalid
    /// - D-Bus communication fails
    /// - ConnMan rejects the request
    pub async fn technology_properties(
        conn: &Connection,
        technology_path: &OwnedObjectPath,
    ) -> Result<DBusDict> {
        let proxy = technology_proxy(conn, technology_path).await?;

        // GetProperties → a{sv}
        let props: DBusDict = proxy.call("GetProperties", &()).await?;
        Ok(props)
    }

    pub async fn technology_tether(
        conn: &Connection,
        technology_path: &OwnedObjectPath,
        enabled: bool,
    ) -> Result<()> {
        todo!()
    }

    pub async fn technology_set(
        conn: &Connection,
        technology_path: &OwnedObjectPath,
        key: &str,
        value: OwnedValue,
    ) -> Result<()> {
        todo!()
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