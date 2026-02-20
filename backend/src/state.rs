use crate::job::Job;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedJob = Arc<Mutex<Job>>;
pub type JobMap = HashMap<u32, SharedJob>;
pub type SharedJobMap = Arc<Mutex<JobMap>>;

#[derive(Clone, Default)]
pub struct AppState {
    pub jobs: SharedJobMap,
}
