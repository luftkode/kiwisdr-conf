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

use crate::wifi::{
    Wifi,
    error::{Result as WifiResult, WifiError},
    model::{Ipv4Connection, Ipv6Connection, ServiceState, ServiceStateKind},
};
use error::{ConnManError, Result};
use std::net::{Ipv4Addr, Ipv6Addr};

/// Thin connman dbus wrapper
/// Acts like connmanctl
mod client {
    use super::consts::*;
    use super::error::Result;
    use std::collections::HashMap;
    use zbus::{Connection, Proxy};
    use zvariant::{OwnedObjectPath, OwnedValue, Value};

    type DBusDict = HashMap<String, OwnedValue>;

    async fn manager_proxy(conn: &Connection) -> Result<Proxy<'_>> {
        Ok(Proxy::new(conn, CONNMAN_DEST, CONNMAN_ROOT_PATH, CONNMAN_MANAGER_IFACE).await?)
    }

    async fn service_proxy<'a>(
        conn: &'a Connection,
        service_path: &'a OwnedObjectPath,
    ) -> Result<Proxy<'a>> {
        Ok(Proxy::new(conn, CONNMAN_DEST, service_path, CONNMAN_SERVICE_IFACE).await?)
    }

    async fn technology_proxy<'a>(
        conn: &'a Connection,
        technology_path: &'a OwnedObjectPath,
    ) -> Result<Proxy<'a>> {
        Ok(Proxy::new(conn, CONNMAN_DEST, technology_path, CONNMAN_TECH_IFACE).await?)
    }

    /// List all ConnMan services.
    ///
    /// This is a thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   / \
    ///   net.connman.Manager.GetServices
    /// ```
    ///
    /// Each entry contains:
    /// - the object path of the service
    ///   (e.g. `/net/connman/service/wifi_<uid>_managed_psk`)
    /// - a property dictionary as returned by ConnMan (`a{sv}`)
    ///
    /// This corresponds closely to `connmanctl services`.
    ///
    /// No interpretation, filtering, or validation is performed here.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the system D-Bus is unavailable
    /// - ConnMan is not running
    /// - the reply does not match `a(oa{sv})`
    pub async fn services(conn: &Connection) -> Result<Vec<(OwnedObjectPath, DBusDict)>> {
        let proxy = manager_proxy(conn).await?;

        // ConnMan Manager.GetServices → a(oa{sv})
        let services: Vec<(OwnedObjectPath, DBusDict)> = proxy.call("GetServices", &()).await?;

        Ok(services)
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
    pub async fn service_connect(conn: &Connection, service_path: &OwnedObjectPath) -> Result<()> {
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

    /// Set a property on a ConnMan service (Wi-Fi, Ethernet, etc.).
    ///
    /// # Example
    ///
    /// ```text
    /// # dbus-send example equivalent
    /// dbus-send --system --dest=net.connman /net/connman/service/wifi_<uid>_managed_psk \
    ///   net.connman.Service.SetProperty string:"AutoConnect" variant:boolean:true
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the service path is invalid
    /// - D-Bus fails
    /// - ConnMan rejects the request
    pub async fn service_config(
        conn: &Connection,
        service_path: &OwnedObjectPath,
        key: &str,
        value: OwnedValue,
    ) -> Result<()> {
        let proxy = service_proxy(conn, service_path).await?;

        proxy
            .call::<&str, (&str, Value), ()>("SetProperty", &(key, Value::from(value)))
            .await?;

        Ok(())
    }

    /// Remove a ConnMan service (Wi-Fi, Ethernet, etc.).
    ///
    /// Thin wrapper around:
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/service/<service> \
    ///   net.connman.Service.Remove
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the service path is invalid
    /// - D-Bus communication fails
    /// - ConnMan rejects the request
    pub async fn service_remove(
        conn: &zbus::Connection,
        service_path: &zvariant::OwnedObjectPath,
    ) -> Result<()> {
        let proxy = service_proxy(conn, service_path).await?;
        proxy.call::<&str, (), ()>("Remove", &()).await?;
        Ok(())
    }

    /// Get the current state of a ConnMan service.
    ///
    /// This is a convenience wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/service/<service> \
    ///   net.connman.Service.GetProperties
    /// ```
    ///
    /// The ConnMan D-Bus API does not expose a dedicated `GetState` method.
    /// Instead, the service state is provided as the `"State"` entry in the
    /// property dictionary returned by `GetProperties`.
    ///
    /// This function:
    /// - calls `net.connman.Service.GetProperties`
    /// - extracts the `"State"` property
    /// - returns it as a raw `OwnedValue`
    pub async fn service_state(
        conn: &Connection,
        service_path: &OwnedObjectPath,
    ) -> Result<OwnedValue> {
        let proxy = service_proxy(conn, service_path).await?;
        let props: DBusDict = proxy.call("GetProperties", &()).await?;

        props.get("State").cloned().ok_or_else(|| {
            super::error::ConnManError::MissingProperty("ConnMan service missing State property")
        })
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
            .call::<&str, (&str, Value), ()>("SetProperty", &("Powered", Value::from(true)))
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
            .call::<&str, (&str, Value), ()>("SetProperty", &("Powered", Value::from(false)))
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

    /// Enable or disable tethering on a ConnMan technology (e.g. Wi-Fi).
    ///
    /// Thin wrapper around:
    ///
    /// ```text
    /// dbus-send --system --print-reply \
    ///   --dest=net.connman \
    ///   /net/connman/technology/<tech> \
    ///   net.connman.Technology.SetProperty \
    ///   string:"Tethering" variant:boolean:<enabled>
    /// ```
    ///
    /// No validation or policy checks are performed here. In particular:
    /// - the technology must already be powered on
    /// - ConnMan may reject the request depending on configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the technology path is invalid
    /// - D-Bus communication fails
    /// - ConnMan rejects the request
    pub async fn technology_tether(
        conn: &Connection,
        technology_path: &OwnedObjectPath,
        enabled: bool,
    ) -> Result<()> {
        let proxy = technology_proxy(conn, technology_path).await?;

        proxy
            .call::<&str, (&str, Value), ()>("SetProperty", &("Tethering", Value::from(enabled)))
            .await?;

        Ok(())
    }

    /// Set a property on a ConnMan technology (Wi-Fi, Ethernet, etc.).
    ///
    /// # Example
    ///
    /// ```text
    /// # dbus-send example equivalent
    /// dbus-send --system --dest=net.connman /net/connman/technology/wifi \
    ///   net.connman.Technology.SetProperty string:"Powered" variant:boolean:true
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the technology path is invalid
    /// - D-Bus communication fails
    /// - ConnMan rejects the request
    pub async fn technology_set(
        conn: &Connection,
        technology_path: &OwnedObjectPath,
        key: &str,
        value: OwnedValue,
    ) -> Result<()> {
        let proxy = technology_proxy(conn, technology_path).await?;

        proxy
            .call::<&str, (&str, Value), ()>("SetProperty", &(key, Value::from(value)))
            .await?;

        Ok(())
    }
}

/// Translates dbus values into `ServiceState`
mod translation {
    use std::collections::HashMap;
    use std::net::{Ipv4Addr, Ipv6Addr};

    use zvariant::OwnedValue;

    use crate::wifi::connman::consts::*;
    use crate::wifi::connman::error::{ConnManError, Result};
    use crate::wifi::model::{Ipv4Connection, Ipv6Connection, ServiceState, ServiceStateKind};

    type DBusDict = HashMap<String, OwnedValue>;

    fn parse_state(props: &DBusDict) -> Result<ServiceStateKind> {
        let raw = get_string(props, PROP_STATE)?;
        ServiceStateKind::try_from(raw.as_str())
            .map_err(|_| ConnManError::InvalidProperty(PROP_STATE))
    }

    fn parse_strength(props: &DBusDict) -> Result<Option<u8>> {
        match props.get(PROP_STRENGTH) {
            None => Ok(None),
            Some(v) => v
                .downcast_ref::<u8>()
                .copied()
                .map(Some)
                .ok_or(ConnManError::InvalidProperty(PROP_STRENGTH)),
        }
    }

    fn parse_ipv4(props: &DBusDict) -> Result<Option<Ipv4Connection>> {
        let value = match props.get(PROP_IPV4) {
            None => return Ok(None),
            Some(v) => v,
        };

        let dict = downcast_dict(value, PROP_IPV4)?;
        Ok(Some(parse_ipv4_dict(dict)?))
    }

    fn parse_ipv6(props: &DBusDict) -> Result<Option<Ipv6Connection>> {
        let value = match props.get(PROP_IPV6) {
            None => return Ok(None),
            Some(v) => v,
        };

        let dict = downcast_dict(value, PROP_IPV6)?;
        Ok(Some(parse_ipv6_dict(dict)?))
    }

    fn parse_ipv4_dict(dict: &DBusDict) -> Result<Ipv4Connection> {
        let address: Ipv4Addr = get_string(dict, IP_ADDRESS)?
            .parse()
            .map_err(|_| ConnManError::InvalidAddress(IP_ADDRESS))?;

        let gateway: Ipv4Addr = get_string(dict, IP_GATEWAY)?
            .parse()
            .map_err(|_| ConnManError::InvalidAddress(IP_GATEWAY))?;

        let prefix = get_u32(dict, IP_PREFIX)? as u8;

        Ok(Ipv4Connection::new(address, prefix, gateway))
    }

    fn parse_ipv6_dict(dict: &DBusDict) -> Result<Ipv6Connection> {
        let address: Ipv6Addr = get_string(dict, IP_ADDRESS)?
            .parse()
            .map_err(|_| ConnManError::InvalidAddress(IP_ADDRESS))?;

        let prefix = get_u32(dict, IP_PREFIX)? as u8;

        let gateway = match dict.get(IP_GATEWAY) {
            None => None,
            Some(v) => {
                let s = v
                    .downcast_ref::<String>()
                    .ok_or(ConnManError::InvalidProperty(IP_GATEWAY))?;

                if s.is_empty() {
                    return Err(ConnManError::InvalidAddress(IP_GATEWAY));
                }

                Some(
                    s.parse()
                        .map_err(|_| ConnManError::InvalidAddress(IP_GATEWAY))?,
                )
            }
        };

        Ok(Ipv6Connection::new(address, prefix, gateway))
    }

    fn downcast_dict<'a>(value: &'a OwnedValue, name: &'static str) -> Result<&'a DBusDict> {
        value
            .downcast_ref::<DBusDict>()
            .ok_or(ConnManError::InvalidProperty(name))
    }

    fn get_string(props: &DBusDict, key: &'static str) -> Result<String> {
        props
            .get(key)
            .and_then(|v| v.downcast_ref::<String>())
            .cloned()
            .ok_or(ConnManError::MissingProperty(key))
    }

    fn get_u32(props: &DBusDict, key: &'static str) -> Result<u32> {
        props
            .get(key)
            .and_then(|v| v.downcast_ref::<u32>())
            .copied()
            .ok_or(ConnManError::MissingProperty(key))
    }

    pub fn service_state_from_properties(
        wifi_uid: String,
        props: &DBusDict,
    ) -> Result<ServiceState> {
        let state = parse_state(props)?;
        let strength = parse_strength(props)?;
        let ipv4 = parse_ipv4(props)?;
        let ipv6 = parse_ipv6(props)?;

        Ok(ServiceState::new(
            wifi_uid,
            state,
            strength,
            ipv4,
            ipv6,
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use zvariant::{OwnedValue, Str};

        fn ov<T: Into<OwnedValue>>(v: T) -> OwnedValue {
            v.into()
        }

        fn ovs(s: &str) -> OwnedValue {
            Str::from(s).into()
        }

        fn empty_props() -> DBusDict {
            DBusDict::new()
        }

        mod parse_state {
            use super::*;

            #[test]
            fn parses_valid_states() {
                let mut props = empty_props();
                props.insert(PROP_STATE.into(), ovs("online"));

                let state = parse_state(&props).unwrap();
                assert_eq!(state, ServiceStateKind::Online);
            }

            #[test]
            fn rejects_unknown_state() {
                let mut props = empty_props();
                props.insert(PROP_STATE.into(), ovs("nonsense"));

                let err = parse_state(&props).unwrap_err();
                matches!(err, ConnManError::InvalidProperty(PROP_STATE));
            }

            #[test]
            fn missing_state_is_error() {
                let props = empty_props();
                let err = parse_state(&props).unwrap_err();
                matches!(err, ConnManError::MissingProperty(PROP_STATE));
            }
        }

        mod parse_strength {
            use super::*;

            #[test]
            fn missing_strength_is_none() {
                let props = empty_props();
                assert_eq!(parse_strength(&props).unwrap(), None);
            }

            #[test]
            fn parses_strength() {
                let mut props = empty_props();
                props.insert(PROP_STRENGTH.into(), ov(73u8));

                assert_eq!(parse_strength(&props).unwrap(), Some(73));
            }

            #[test]
            fn invalid_strength_type_is_error() {
                let mut props = empty_props();
                props.insert(PROP_STRENGTH.into(), ovs("loud"));

                let err = parse_strength(&props).unwrap_err();
                matches!(err, ConnManError::InvalidProperty(PROP_STRENGTH));
            }

            #[test]
            fn strength_above_100_is_preserved() {
                let mut props = empty_props();
                props.insert(PROP_STATE.into(), ovs("online"));
                props.insert(PROP_STRENGTH.into(), ov(255u8));

                let state = service_state_from_properties("wifi0".into(), &props).unwrap();
                assert_eq!(state.strength(), Some(255));
            }
        }

        mod parse_ip_blocks {
            use super::*;

            #[test]
            fn missing_ipv4_block_is_none() {
                let props = empty_props();
                assert!(parse_ipv4(&props).unwrap().is_none());
            }

            #[test]
            fn invalid_ipv4_block_type_is_error() {
                let mut props = empty_props();
                props.insert(PROP_IPV4.into(), ovs("not a dict"));

                let err = parse_ipv4(&props).unwrap_err();
                matches!(err, ConnManError::InvalidProperty(PROP_IPV4));
            }

            #[test]
            fn invalid_ipv6_block_type_is_error() {
                let mut props = empty_props();
                props.insert(PROP_IPV6.into(), ov(123u32));

                let err = parse_ipv6(&props).unwrap_err();
                matches!(err, ConnManError::InvalidProperty(PROP_IPV6));
            }
        }

        mod parse_ipv4_dict {
            use super::*;

            #[test]
            fn parses_valid_ipv4_dict() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("192.168.1.10"));
                dict.insert(IP_GATEWAY.into(), ovs("192.168.1.1"));
                dict.insert(IP_PREFIX.into(), ov(24u32));

                let ipv4 = parse_ipv4_dict(&dict).unwrap();
                assert_eq!(ipv4.cidr(), "192.168.1.10/24");
                assert_eq!(ipv4.gateway(), Ipv4Addr::new(192, 168, 1, 1));
            }

            #[test]
            fn missing_address_is_error() {
                let mut dict = DBusDict::new();
                dict.insert(IP_PREFIX.into(), ov(24u32));

                let err = parse_ipv4_dict(&dict).unwrap_err();
                matches!(err, ConnManError::MissingProperty(IP_ADDRESS));
            }
        }

        mod parse_ipv4_edge_cases {
            use super::*;

            #[test]
            fn rejects_invalid_ipv4_address() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("999.999.999.999"));
                dict.insert(IP_GATEWAY.into(), ovs("192.168.1.1"));
                dict.insert(IP_PREFIX.into(), ov(24u32));

                let err = parse_ipv4_dict(&dict).unwrap_err();
                matches!(err, ConnManError::InvalidAddress(IP_ADDRESS));
            }

            #[test]
            fn rejects_invalid_gateway_address() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("192.168.1.10"));
                dict.insert(IP_GATEWAY.into(), ovs("nope"));
                dict.insert(IP_PREFIX.into(), ov(24u32));

                let err = parse_ipv4_dict(&dict).unwrap_err();
                matches!(err, ConnManError::InvalidAddress(IP_GATEWAY));
            }

            #[test]
            fn prefix_zero_is_allowed() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("0.0.0.0"));
                dict.insert(IP_GATEWAY.into(), ovs("0.0.0.0"));
                dict.insert(IP_PREFIX.into(), ov(0u32));

                let ipv4 = parse_ipv4_dict(&dict).unwrap();
                assert_eq!(ipv4.cidr(), "0.0.0.0/0");
            }
        }

        mod parse_ipv6_dict {
            use super::*;

            #[test]
            fn parses_ipv6_without_gateway() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("fe80::1"));
                dict.insert(IP_PREFIX.into(), ov(64u32));

                let ipv6 = parse_ipv6_dict(&dict).unwrap();
                assert_eq!(ipv6.cidr(), "fe80::1/64");
                assert!(ipv6.gateway().is_none());
            }

            #[test]
            fn parses_ipv6_with_gateway() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("fe80::1"));
                dict.insert(IP_GATEWAY.into(), ovs("fe80::ff"));
                dict.insert(IP_PREFIX.into(), ov(64u32));

                let ipv6 = parse_ipv6_dict(&dict).unwrap();
                assert_eq!(
                    ipv6.gateway().unwrap(),
                    Ipv6Addr::from([0xfe80, 0, 0, 0, 0, 0, 0, 0xff])
                );
            }
        }

        mod parse_ipv6_edge_cases {
            use super::*;

            #[test]
            fn rejects_invalid_ipv6_address() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("this:is:garbage"));
                dict.insert(IP_PREFIX.into(), ov(64u32));

                let err = parse_ipv6_dict(&dict).unwrap_err();
                matches!(err, ConnManError::InvalidAddress(IP_ADDRESS));
            }

            #[test]
            fn empty_gateway_is_error() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("fe80::1"));
                dict.insert(IP_GATEWAY.into(), ovs(""));
                dict.insert(IP_PREFIX.into(), ov(64u32));

                let err = parse_ipv6_dict(&dict).unwrap_err();
                matches!(err, ConnManError::InvalidAddress(IP_GATEWAY));
            }

            #[test]
            fn missing_prefix_is_error() {
                let mut dict = DBusDict::new();
                dict.insert(IP_ADDRESS.into(), ovs("fe80::1"));

                let err = parse_ipv6_dict(&dict).unwrap_err();
                matches!(err, ConnManError::MissingProperty(IP_PREFIX));
            }
        }

        mod downcast_dict {
            use super::*;

            #[test]
            fn downcasts_dictionary() {
                let mut inner = DBusDict::new();
                inner.insert("Key".into(), ovs("Value"));

                let value = ov(inner);
                let dict = downcast_dict(&value, "Test").unwrap();

                assert!(dict.contains_key("Key"));
            }

            #[test]
            fn rejects_non_dictionary() {
                let value = ovs("not a dict");
                let err = downcast_dict(&value, "Test").unwrap_err();
                matches!(err, ConnManError::InvalidProperty("Test"));
            }
        }

        mod service_state_from_properties {
            use super::*;

            #[test]
            fn builds_minimal_service_state() {
                let mut props = empty_props();
                props.insert(PROP_STATE.into(), ovs("ready"));
                props.insert(PROP_STRENGTH.into(), ov(55u8));

                let state = service_state_from_properties("wifi0".into(), &props).unwrap();

                assert_eq!(state.wifi_uid(), "wifi0");
                assert_eq!(state.state(), ServiceStateKind::Ready);
                assert_eq!(state.strength(), Some(55));
                assert!(state.ipv4().is_none());
                assert!(state.ipv6().is_none());
            }
        }

        mod service_state_partial_failures {
            use super::*;

            #[test]
            fn invalid_ipv4_does_not_hide_state_error() {
                let mut ipv4 = DBusDict::new();
                ipv4.insert(IP_ADDRESS.into(), ovs("broken"));
                ipv4.insert(IP_PREFIX.into(), ov(24u32));

                let mut props = empty_props();
                props.insert(PROP_STATE.into(), ovs("online"));
                props.insert(PROP_IPV4.into(), ov(ipv4));

                let err = service_state_from_properties("wifi0".into(), &props).unwrap_err();
                matches!(err, ConnManError::InvalidAddress(IP_ADDRESS));
            }

            #[test]
            fn missing_strength_does_not_fail_service() {
                let mut props = empty_props();
                props.insert(PROP_STATE.into(), ovs("online"));

                let state = service_state_from_properties("wifi0".into(), &props).unwrap();
                assert_eq!(state.strength(), None);
            }
        }
    }
}

pub struct ConnManConnection {
    connection: zbus::Connection,
}

impl ConnManConnection {
    /// Opens a connection to the system D-Bus.
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
