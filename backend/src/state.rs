use crate::job::Job;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedJob = Arc<Mutex<Job>>;
pub type JobMap = HashMap<u32, SharedJob>;
pub type SharedJobMap = Arc<Mutex<JobMap>>;

#[derive(Clone)]
pub struct AppState {
    pub jobs: SharedJobMap,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
