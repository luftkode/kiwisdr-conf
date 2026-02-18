use crate::job::Job;
use crate::wifi::connman::agent::ConnManAgent;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedJob = Arc<Mutex<Job>>;
pub type JobMap = HashMap<u32, SharedJob>;
pub type SharedJobMap = Arc<Mutex<JobMap>>;

/// Global application state.
///
/// Contains:
/// - Active recorder jobs
/// - Shared ConnMan agent (singleton)
/// - Single connection to system D-Bus
#[derive(Clone)]
pub struct AppState {
    /// Active recorder jobs
    pub jobs: SharedJobMap,

    /// Shared ConnMan agent for serving Wi-Fi secrets
    ///
    /// This agent:
    /// - Receives credential requests from ConnMan
    /// - Serves secrets via RequestInput
    /// - Must be a singleton to ensure only one agent handles credential requests
    pub wifi_agent: Arc<ConnManAgent>,

    /// Single connection to system dbus
    pub dbus_conn: Arc<zbus::Connection>,
}

impl AppState {
    /// Initialize a new `AppState`.
    ///
    /// This performs the following:
    /// 1. Connects to the system D-Bus.
    /// 2. Creates a singleton `ConnManAgent`.
    /// 3. Registers the agent object path on D-Bus.
    /// 4. Spawns a background task to keep the agent alive.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if:
    /// - Connecting to the system D-Bus fails.
    /// - Registering the agent object path fails.
    /// - Registering the agent with ConnMan fails.
    pub async fn new() -> io::Result<Self> {
        let dbus_connection = zbus::Connection::system().await.map_err(|e| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("Failed to connect to system DBus: {}", e),
            )
        })?;

        // Create the ConnMan agent (concrete type)
        let wifi_agent = Arc::new(ConnManAgent::new());

        Ok(Self {
            jobs: SharedJobMap::default(),
            wifi_agent,
            dbus_conn: Arc::new(dbus_connection),
        })
    }
}
