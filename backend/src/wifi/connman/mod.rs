#![allow(unused)]

mod client;
mod translation;

mod consts {
    use zvariant::{ObjectPath, OwnedObjectPath};

    /// D-Bus well-known name owned by the ConnMan daemon
    pub const CONNMAN_DEST: &str = "net.connman";

    /// ConnMan manager object path (global object)
    pub const CONNMAN_MANAGER_PATH: &str = "/";

    /// ConnMan manager interface (global operations)
    pub const CONNMAN_MANAGER_IFACE: &str = "net.connman.Manager";

    /// ConnMan service interface (Wi-Fi, Ethernet, etc.)
    pub const CONNMAN_SERVICE_IFACE: &str = "net.connman.Service";

    /// ConnMan technology interface (wifi, ethernet, p2p)
    pub const CONNMAN_TECH_IFACE: &str = "net.connman.Technology";

    /// ConnMan Wi-Fi technology object path
    pub const CONNMAN_WIFI_TECH_PATH: &str = "/net/connman/technology/wifi";

    pub const CONNMAN_SERVICE_PATH_PREFIX: &str = "/net/connman/service/";

    pub fn service_path(sufix: &str) -> Result<OwnedObjectPath, zvariant::Error> {
        OwnedObjectPath::try_from(format!("{}{}", CONNMAN_SERVICE_PATH_PREFIX, sufix))
    }

    pub const PROP_NAME: &str = "Name";
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
        MissingProperty(String),

        #[error("Invalid property: {0}")]
        InvalidProperty(String),

        #[error("Invalid address: {0}")]
        InvalidAddress(String),

        #[error("Operation failed: {0}")]
        OperationFailed(String),

        #[error("Not found: {0}")]
        NotFound(String),
    }

    pub type Result<T> = std::result::Result<T, ConnManError>;
}

use crate::wifi::{
    Wifi,
    connman::{
        consts::*,
        error::{ConnManError, Result},
    },
    error::{WifiError, WifiResult},
    model::{ServiceState, ServiceStateKind},
};
use zbus::Connection;
use zvariant::{ObjectPath, OwnedObjectPath};

pub struct ConnManConnection {
    connection: zbus::Connection,
}

impl ConnManConnection {
    /// Opens a connection to the system D-Bus.
    pub async fn new() -> WifiResult<Self> {
        let connection = zbus::Connection::system()
            .await
            .map_err(ConnManError::from)?;
        Ok(Self { connection })
    }

    fn connection(&self) -> &Connection {
        &self.connection
    }

    fn service_path(uid: &str) -> WifiResult<OwnedObjectPath> {
        let full = format!("{}/service/{}", CONNMAN_ROOT_PATH, uid);
        Ok(OwnedObjectPath::from(
            ObjectPath::try_from(full.clone()).map_err(|e| {
                WifiError::OperationFailed(format!(
                    "Failed to resolve service path for '{}': '{}', {}",
                    uid, full, e
                ))
            })?,
        ))
    }
}

impl Wifi for ConnManConnection {
    async fn get_available(&self) -> WifiResult<Vec<ServiceState>> {
        client::technology_scan(
            self.connection(),
            &OwnedObjectPath::try_from(format!("{}/technology/wifi", CONNMAN_ROOT_PATH))
                .expect("Input is const so it shouldn't fail"),
        )
        .await
        .map_err(|e| WifiError::OperationFailed(format!("Scan failed: {}", e)))?;

        let services = client::services(self.connection()).await?;
        let mut out = Vec::new();

        for (path, props) in services {
            if path
                .as_str()
                .starts_with(&format!("{}/service/wifi_", CONNMAN_ROOT_PATH))
            {
                let state =
                    translation::service_state_from_properties(path.as_str().to_string(), &props)
                        .map_err(|e| {
                        WifiError::OperationFailed(format!(
                            "Failed to parse service {}: {}",
                            path, e
                        ))
                    })?;
                out.push(state);
            }
        }

        Ok(out)
    }

    async fn connect(&self, wifi_uid: &str, passphrase: Option<&str>) -> WifiResult<()> {
        let service_path = OwnedObjectPath::try_from(wifi_uid)
            .map_err(|_| WifiError::OperationFailed("Invalid service path".into()))?;

        if let Some(psk) = passphrase {
            client::service_config(
                self.connection(),
                &service_path,
                "Passphrase",
                zvariant::Str::from(psk).into(),
            )
            .await
            .map_err(|e| WifiError::OperationFailed(format!("Failed to set passphrase: {}", e)))?;
        }

        client::service_connect(self.connection(), &service_path)
            .await
            .map_err(|e| WifiError::OperationFailed(format!("Failed to connect: {}", e)))?;

        for _ in 0..20 {
            let services = client::services(self.connection()).await?;
            if let Some((path, props)) = services.into_iter().find(|(p, _)| p.as_str() == wifi_uid)
            {
                let state =
                    translation::service_state_from_properties(path.as_str().to_string(), &props)?;
                match state.state() {
                    ServiceStateKind::Online => return Ok(()),
                    ServiceStateKind::Failure => {
                        return Err(WifiError::OperationFailed("Connection failed".into()));
                    }
                    _ => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
                }
            }
        }

        Err(WifiError::OperationFailed("Timed out connecting".into()))
    }

    async fn disconnect(&self, wifi_uid: &str) -> WifiResult<()> {
        let service_path = OwnedObjectPath::try_from(wifi_uid)
            .map_err(|_| WifiError::OperationFailed("Invalid service path".into()))?;

        client::service_disconnect(self.connection(), &service_path)
            .await
            .map_err(|e| WifiError::OperationFailed(format!("Failed to disconnect: {}", e)))?;

        for _ in 0..20 {
            let services = client::services(self.connection()).await?;
            if let Some((path, props)) = services.into_iter().find(|(p, _)| p.as_str() == wifi_uid)
            {
                let state =
                    translation::service_state_from_properties(path.as_str().to_string(), &props)?;
                match state.state() {
                    ServiceStateKind::Idle => return Ok(()),
                    _ => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
                }
            }
        }

        Err(WifiError::OperationFailed("Timed out disconnecting".into()))
    }
}
