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

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tokio::sync::Mutex;
use zbus::{Connection, interface};
use zvariant::OwnedValue;

use crate::wifi::connman::error::Result;

/// One-shot in-memory credential store.
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

    /// Inserts a passphrase for a service path.
    ///
    /// If a secret already exists for this service, it is overwritten.
    ///
    /// # Arguments
    /// * `service` - Full ConnMan service object path
    /// * `passphrase` - WPA/WPA2 passphrase
    pub fn insert(&mut self, service: String, passphrase: String) {
        self.map.insert(service, passphrase);
    }

    /// Consumes and returns the secret for `service`.
    ///
    /// This operation is **destructive**.
    /// Subsequent calls return `None`.
    pub fn take(&mut self, service: &str) -> Option<String> {
        self.map.remove(service)
    }

    /// Returns `true` if a secret exists for `service`.
    pub fn contains(&self, service: &str) -> bool {
        self.map.contains_key(service)
    }
}

/// ConnMan Agent.
///
/// Implements the `net.connman.Agent` D-Bus interface.
///
/// This agent is responsible for providing credentials
/// when ConnMan requests them during connection attempts.
#[derive(Default)]
pub struct ConnManAgent {
    secrets: Arc<Mutex<WifiSecrets>>,
}

impl ConnManAgent {
    // Creates a new agent backed by a shared secret store.
    pub fn new(wifi_secrets: Arc<Mutex<WifiSecrets>>) -> Self {
        Self {
            secrets: wifi_secrets,
        }
    }

    /// Registers the agent on the system bus.
    ///
    /// # Errors
    /// Returns an error if registration fails or ConnMan is unreachable.
    pub async fn register(conn: &Connection) -> Result<()> {
        let proxy = zbus::Proxy::new(conn, "net.connman", "/", "net.connman.Manager").await?;

        proxy
            .call::<&str, (&str,), ()>("RegisterAgent", &("/net/connman/agent",))
            .await?;

        Ok(())
    }
}

#[interface(name = "net.connman.Agent")]
impl ConnManAgent {
    /// Supplies requested credentials.
    ///
    /// ConnMan calls this method when authentication data is required.
    ///
    /// Only requested fields are returned.
    async fn request_input(
        &self,
        service: String,
        fields: HashMap<String, OwnedValue>,
    ) -> HashMap<String, OwnedValue> {
        let mut secrets = self.secrets.lock().await;
        let mut reply = HashMap::new();

        // Only respond to fields ConnMan explicitly requests
        if fields.contains_key("Passphrase")
            && let Some(passphrase) = secrets.take(&service)
        {
            reply.insert(
                "Passphrase".to_string(),
                zvariant::Str::from(passphrase).into(),
            );
        }

        reply
    }

    /// Reports connection errors.
    async fn report_error(&self, service: String, error: String) {
        eprintln!("ConnMan error [{}]: {}", service, error);
    }

    /// Cancels a pending request.
    async fn cancel(&self) {}

    /// Called when agent is released.
    async fn release(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_take_is_one_shot() {
        let mut s = WifiSecrets::new();
        s.insert("svc".into(), "pw".into());

        assert_eq!(s.take("svc"), Some("pw".into()));
        assert_eq!(s.take("svc"), None);
    }

    #[test]
    fn overwrite_secret() {
        let mut s = WifiSecrets::new();
        s.insert("svc".into(), "pw1".into());
        s.insert("svc".into(), "pw2".into());

        assert_eq!(s.take("svc"), Some("pw2".into()));
    }

    #[test]
    fn isolation_between_services() {
        let mut s = WifiSecrets::new();
        s.insert("a".into(), "1".into());
        s.insert("b".into(), "2".into());

        assert_eq!(s.take("a"), Some("1".into()));
        assert_eq!(s.take("b"), Some("2".into()));
    }

    #[test]
    fn contains_works() {
        let mut s = WifiSecrets::new();
        s.insert("svc".into(), "pw".into());

        assert!(s.contains("svc"));
        assert!(!s.contains("missing"));
    }
}
