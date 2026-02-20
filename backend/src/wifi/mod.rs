pub mod model;
pub mod wpa_supplicant;

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum WifiError {
        #[error("Wifi-Ctrl error: {0}")]
        WifiCtrl(#[from] wifi_ctrl::error::Error),

        #[error("Wi-Fi operation failed: {0}")]
        OperationFailed(String),

        #[error("Wi-Fi network not found: {0}")]
        NotFound(String),

        #[error("Invalid service state: {0}")]
        InvalidServiceState(String),
    }

    pub type WifiResult<T> = std::result::Result<T, WifiError>;
}

use crate::wifi::error::WifiResult;
use crate::wifi::model::WifiNetwork;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiAuth {
    Open { ssid: String },
    Psk { ssid: String, psk: String },
}

impl WifiAuth {
    pub fn ssid(&self) -> Option<&str> {
        match self {
            Self::Open { ssid } => Some(ssid),
            Self::Psk { ssid, .. } => Some(ssid),
        }
    }
    
    pub fn psk(&self) -> Option<&str> {
        match self {
            Self::Open { .. } => None,
            Self::Psk { psk, .. } => Some(psk),
        }
    }
}

/// Interface for managing Wi-Fi connectivity.
///
/// Implementors provide methods to list networks and manage connections.
/// All operations are asynchronous.
#[allow(async_fn_in_trait)] // only used with concrete types, never dyn
pub trait Wifi {
    /// Returns a list of available Wi-Fi networks.
    ///
    /// Each network is represented as a [`WifiNetwork`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use backend::wifi::{Wifi, model::WifiNetwork, error::WifiResult};
    /// # async fn example(wifi: impl Wifi) -> WifiResult<()> {
    ///     let networks: Vec<WifiNetwork> = wifi.get_available().await?;
    ///     # Ok(())
    /// # }
    /// ```
    async fn get_available(&self) -> WifiResult<Vec<WifiNetwork>>;

    /// Connects to a Wi-Fi network identified by `auth`.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails, the network does not exist,
    /// or the credentials are invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use backend::wifi::{Wifi, error::WifiResult, WifiAuth};
    /// # async fn example(wifi: impl Wifi) -> WifiResult<()> {
    /// wifi.connect(WifiAuth::Open{ssid: "SomeWifi07".to_string()}).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn connect(&self, auth: WifiAuth) -> WifiResult<()>;

    /// Disconnects from the curently connected a Wi-Fi network.
    ///
    /// # Errors
    ///
    /// Returns an error if the network is not connected or the operation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use backend::wifi::{Wifi, error::WifiResult};
    /// # async fn example(wifi: impl Wifi) -> WifiResult<()> {
    /// wifi.disconnect().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn disconnect(&self) -> WifiResult<()>;
}
