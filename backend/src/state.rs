use crate::job::Job;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use actix_web::web;

pub type SharedJob = Arc<Mutex<Job>>;
pub type SharedJobMap = HashMap<u32, SharedJob>;

#[derive(Clone)]
pub struct AppState {
    pub jobs: web::Data<Arc<Mutex<SharedJobMap>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            jobs: web::Data::new(
                Arc::new(
                    Mutex::new(
                        HashMap::new()
                    )
                )
            )
        }
    }
}