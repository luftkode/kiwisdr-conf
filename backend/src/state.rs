use crate::job::Job;
use crate::wifi::error::WifiResult;
use crate::wifi::wpa_supplicant::WpaWifi;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedJob = Arc<Mutex<Job>>;
pub type JobMap = HashMap<u32, SharedJob>;
pub type SharedJobMap = Arc<Mutex<JobMap>>;

pub type SharedWpaWifi = Arc<Mutex<WpaWifi>>;

#[derive(Clone)]
pub struct AppState {
    pub jobs: SharedJobMap,
    pub wpa_wifi: SharedWpaWifi,
}

impl AppState {
    pub async fn new(wifi_interface: &str) -> WifiResult<Self> {
        Ok(Self {
            jobs: SharedJobMap::default(),
            wpa_wifi: Arc::new(Mutex::new(WpaWifi::new(wifi_interface).await?)),
        })
    }
}
