//! ConnMan Wi-Fi Agent implementation.
//!
//! This module implements a minimal ConnMan Agent that provides Wi-Fi
//! credentials (passphrases) to ConnMan on demand via D-Bus.
//!
//! ConnMan does **not** allow setting the Wi-Fi passphrase via
//! `net.connman.Service.SetProperty`. Instead, it requests secrets
//! asynchronously through the Agent interface.
//!
//! This agent:
//! - stores secrets temporarily in memory
//! - matches them by service object path
//! - supplies them only when requested
//! - consumes them once (one-shot semantics)
//!
//! This mirrors ConnMan's design:
//! > "Credentials are requested, not pushed."

use std::collections::{BTreeMap, HashMap};
use tokio::sync::Mutex;
use zbus::{Connection, interface};
use zvariant::OwnedValue;

use crate::wifi::connman::error::Result;

/// In-memory store for Wi-Fi secrets.
///
/// Maps:
/// ```text
/// service_path -> passphrase
/// ```
///
/// Secrets are **consumed once** and then deleted.
/// This prevents accidental reuse, leakage, and stale credentials.
#[derive(Debug, Default)]
pub struct WifiSecrets {
    map: BTreeMap<String, String>,
}

impl WifiSecrets {
    /// Create a new empty secret store.
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Insert a Wi-Fi passphrase for a service.
    ///
    /// # Arguments
    /// * `service` - Full ConnMan service object path
    /// * `passphrase` - WPA/WPA2 passphrase
    pub fn insert(&mut self, service: String, passphrase: String) {
        self.map.insert(service, passphrase);
    }

    /// Take (consume) a passphrase for a service.
    ///
    /// This removes the secret from memory after retrieval.
    pub fn take(&mut self, service: &str) -> Option<String> {
        self.map.remove(service)
    }
}

/// ConnMan Agent.
///
/// Implements the `net.connman.Agent` D-Bus interface.
///
/// This agent is responsible for providing credentials
/// when ConnMan requests them during connection attempts.
pub struct ConnManAgent {
    secrets: Mutex<WifiSecrets>,
}

impl ConnManAgent {
    /// Create a new agent.
    pub fn new() -> Self {
        Self {
            secrets: Mutex::new(WifiSecrets::new()),
        }
    }

    /// Insert a Wi-Fi secret for a service.
    ///
    /// This is called by your API layer before `Service.Connect`.
    pub async fn insert_secret(&self, service: &str, passphrase: &str) {
        let mut secrets = self.secrets.lock().await;
        secrets.insert(service.to_string(), passphrase.to_string());
    }

    /// Register the agent on the system bus.
    ///
    /// This must be called once during startup.
    ///
    /// # Arguments
    /// * `conn` - System D-Bus connection
    pub async fn register(conn: &Connection) -> Result<()> {
        let proxy = zbus::Proxy::new(conn, "net.connman", "/", "net.connman.Manager").await?;

        // RegisterAgent("/net/connman/agent")
        proxy
            .call::<&str, (&str,), ()>("RegisterAgent", &("/net/connman/agent",))
            .await?;

        Ok(())
    }
}

#[interface(name = "net.connman.Agent")]
impl ConnManAgent {
    /// RequestInput callback.
    ///
    /// Called by ConnMan when credentials are required.
    ///
    /// # Arguments
    /// * `service` - Service object path
    /// * `fields` - Requested input fields
    ///
    /// # Returns
    /// Dictionary of provided fields (`a{sv}`)
    ///
    /// Example request:
    /// ```text
    /// Fields:
    ///   Passphrase (type: string)
    /// ```
    async fn request_input(
        &self,
        service: String,
        _fields: HashMap<String, OwnedValue>,
    ) -> HashMap<String, OwnedValue> {
        let mut secrets = self.secrets.lock().await;

        let mut reply = HashMap::new();

        if let Some(passphrase) = secrets.take(&service) {
            reply.insert(
                "Passphrase".to_string(),
                zvariant::Str::from(passphrase).into(),
            );
        }

        reply
    }

    /// ReportError callback.
    ///
    /// Called by ConnMan when a connection error occurs.
    async fn report_error(&self, service: String, error: String) {
        eprintln!("ConnMan error on {}: {}", service, error);
    }

    /// Cancel callback.
    ///
    /// Called if a request is cancelled.
    async fn cancel(&self) {
        // No-op
    }

    /// Release callback.
    ///
    /// Called when ConnMan unregisters the agent.
    async fn release(&self) {
        // No-op
    }
}
