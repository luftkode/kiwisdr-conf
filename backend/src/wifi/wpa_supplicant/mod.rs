use tokio::spawn;
use wifi_ctrl::sta;

use crate::wifi::{Wifi, WifiAuth, error::WifiResult, model::WifiNetwork};

pub struct WpaWifi {
    requester: sta::RequestClient,
}

impl WpaWifi {
    pub async fn new(interface: &str) -> WifiResult<Self> {
        let wpa_path = format!("/var/run/wpa_supplicant/{}", interface);

        let mut setup = sta::WifiSetup::new()?;
        setup.set_socket_path(wpa_path);

        let broadcast = setup.get_broadcast_receiver();
        let requester = setup.get_request_client();
        let runtime = setup.complete();

        spawn(Self::broadcast_listener(broadcast));
        spawn(Self::runtime(runtime));

        Ok(Self { requester })
    }

    async fn runtime(runtime: sta::WifiStation) {
        if let Err(e) = runtime.run().await {
            eprintln!("WpaWifi::runtime: {e}");
        }
    }

    async fn broadcast_listener(mut broadcast_receiver: sta::BroadcastReceiver) {
        while let Ok(broadcast) = broadcast_receiver.recv().await {
            match broadcast {
                sta::Broadcast::Connected => {
                    println!("WiFi: Connected to a network");
                }
                sta::Broadcast::Disconnected => {
                    println!("WiFi: Disconnected");
                }
                sta::Broadcast::WrongPsk => {
                    eprintln!("WiFi Error: Incorrect Password");
                }
                sta::Broadcast::NetworkNotFound => {
                    eprintln!("WiFi Error: Network Not Found");
                }
                sta::Broadcast::Ready => {
                    println!("WiFi: wpa_supplicant control interface ready");
                }
                sta::Broadcast::Unknown(msg) => {
                    println!("WiFi: Other: {}", msg);
                }
            }
        }
        eprintln!("WpaWifi::broadcast_listener: Event stream closed.");
    }
}

