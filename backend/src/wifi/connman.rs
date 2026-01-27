pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum ConnManError {
        #[error("DBus error: {0}")]
        DBus(#[from] zbus::Error),

        #[error("Invalid object path")]
        InvalidPath,

        #[error("Missing property: {0}")]
        MissingProperty(String),

        #[error("Invalid IP address: {0}")]
        InvalidAddress(String),
    }

    pub type Result<T> = std::result::Result<T, ConnManError>;
}
