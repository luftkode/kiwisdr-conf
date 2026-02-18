use crate::job::Job;
use crate::wifi::connman::agent::ConnManAgent;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedJob = Arc<Mutex<Job>>;
pub type JobMap = HashMap<u32, SharedJob>;
pub type SharedJobMap = Arc<Mutex<JobMap>>;

#[derive(Clone, Default)]
pub struct AppState {
    /// Active recorder jobs
    pub jobs: SharedJobMap,

    /// Shared ConnMan agent (DBus object)
    ///
    /// This agent:
    /// - receives credential requests from ConnMan
    /// - serves secrets via RequestInput
    /// - must be a singleton
    pub wifi_agent: Arc<ConnManAgent>,
}
