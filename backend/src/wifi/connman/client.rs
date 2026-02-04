//! Thin connman dbus wrapper
//! Acts like connmanctl

use super::consts::*;
use super::error::Result;
use std::collections::HashMap;
use zbus::{Connection, Proxy};
use zvariant::{OwnedObjectPath, OwnedValue, Value};

type DBusDict = HashMap<String, OwnedValue>;

async fn manager_proxy(conn: &Connection) -> Result<Proxy<'_>> {
    Ok(Proxy::new(
        conn,
        CONNMAN_DEST,
        CONNMAN_MANAGER_PATH,
        CONNMAN_MANAGER_IFACE,
    )
    .await?)
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
    let technologies: Vec<(OwnedObjectPath, DBusDict)> = proxy.call("GetTechnologies", &()).await?;

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
pub async fn service_disconnect(conn: &Connection, service_path: &OwnedObjectPath) -> Result<()> {
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
        super::error::ConnManError::MissingProperty(
            "ConnMan service missing State property".to_string(),
        )
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
pub async fn technology_enable(conn: &Connection, technology_path: &OwnedObjectPath) -> Result<()> {
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
pub async fn technology_scan(conn: &Connection, technology_path: &OwnedObjectPath) -> Result<()> {
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
