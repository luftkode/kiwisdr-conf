pub mod connman;
pub mod wpa_supplicant;
pub mod model;

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum WifiError {
        #[error("ConnMan error: {0}")]
        ConnMan(#[from] crate::wifi::connman::error::ConnManError),

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

#[allow(async_fn_in_trait)] // only used with concrete types, never dyn
/// Interface for managing Wi-Fi connectivity.
///
/// Implementors provide methods to list networks and manage connections.
/// All operations are asynchronous.
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

    /// Connects to a Wi-Fi network identified by `wifi_uid`.
    ///
    /// If the network requires a passphrase, provide it via `passphrase`.
    /// Open networks can be connected to with `None`.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails, the network does not exist,
    /// or the credentials are invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use backend::wifi::{Wifi, error::WifiResult};
    /// # async fn example(wifi: impl Wifi) -> WifiResult<()> {
    ///     wifi.connect("wifi0", Some("password")).await?;
    ///     # Ok(())
    /// # }
    /// ```
    async fn connect(&self, wifi_uid: &str, passphrase: Option<&str>) -> WifiResult<()>;

    /// Disconnects from a Wi-Fi network identified by `wifi_uid`.
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
    ///     wifi.disconnect("wifi0").await?;
    ///     # Ok(())
    /// # }
    /// ```
    async fn disconnect(&self, wifi_uid: &str) -> WifiResult<()>;
}
