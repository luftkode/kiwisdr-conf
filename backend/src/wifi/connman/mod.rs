#![allow(unused)]

mod client;
mod translation;

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
    connman::{consts::*, error::Result},
    error::{WifiError, WifiResult},
    model::ServiceState,
};
use zbus::Connection;
use zvariant::{ObjectPath, OwnedObjectPath};

pub struct ConnManConnection {
    connection: zbus::Connection,
}

impl ConnManConnection {
    /// Opens a connection to the system D-Bus.
    pub async fn new() -> Result<Self> {
        let connection = zbus::Connection::system().await?;
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
        let wifi_tech: OwnedObjectPath = OwnedObjectPath::from(
            ObjectPath::try_from(format!("{}/technology/wifi", CONNMAN_ROOT_PATH))
                .map_err(|_| WifiError::NotFound("Invalid wifi technology path".into()))?,
        );

        client::technology_scan(self.connection(), &wifi_tech).await?;

        let services = client::services(self.connection()).await?;

        let mut out = Vec::new();

        for (path, props) in services {
            // Only wifi services
            if !path.as_str().contains("/service/wifi_") {
                continue;
            }

            let uid = path.as_str().rsplit('/').next().unwrap_or("").to_string();

            match translation::service_state_from_properties(uid, &props) {
                Ok(state) => out.push(state),
                Err(e) => {
                    return Err(WifiError::InvalidServiceState(format!(
                        "Skipping invalid service {}: {}",
                        path, e
                    )));
                }
            }
        }

        Ok(out)
    }

    async fn connect(&self, wifi_uid: &str, passphrase: Option<&str>) -> WifiResult<()> {
        let path = Self::service_path(wifi_uid)?;

        if let Some(psk) = passphrase {
            client::service_config(
                self.connection(),
                &path,
                "Passphrase",
                zvariant::Str::from(psk).into(),
            )
            .await
            .map_err(|e| {
                WifiError::OperationFailed(format!(
                    "Failed to set passphrase for '{}': {}",
                    wifi_uid, e
                ))
            })?;
        }

        client::service_connect(self.connection(), &path)
            .await
            .map_err(|e| {
                WifiError::OperationFailed(format!("Failed to connect to '{}': {}", wifi_uid, e))
            })?;

        Ok(())
    }

    async fn disconnect(&self, wifi_uid: &str) -> WifiResult<()> {
        let path = Self::service_path(wifi_uid)?;

        client::service_disconnect(self.connection(), &path)
            .await
            .map_err(|e| {
                WifiError::OperationFailed(format!(
                    "Failed to disconnect from '{}': {}",
                    wifi_uid, e
                ))
            })?;

        Ok(())
    }
}
