pub mod connman;
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

    pub type Result<T> = std::result::Result<T, WifiError>;
}

use crate::wifi::error::Result as WifiResult;
use crate::wifi::model::ServiceState;

#[allow(async_fn_in_trait)] // I will never use "dyn Wifi"
pub trait Wifi {
    /// List available Wi-Fi networks (ServiceState for each)
    async fn get_available(&self) -> WifiResult<Vec<ServiceState>>;

    /// Connect to a Wi-Fi network by uid
    /// Optional passphrase for open vs WPA/WPA2 networks
    async fn connect(&self, wifi_uid: &str, passphrase: Option<&str>) -> WifiResult<()>;

    /// Disconnect from a Wi-Fi network by uid
    async fn disconnect(&self, wifi_uid: &str) -> WifiResult<()>;
}
