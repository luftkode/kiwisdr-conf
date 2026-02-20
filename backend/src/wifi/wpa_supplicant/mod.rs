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

impl Wifi for WpaWifi {
    async fn get_available(&self) -> WifiResult<Vec<WifiNetwork>> {
        let scan_results = self.requester.get_scan().await?;

        // .iter() provides &ScanResult, matching the new From impl above
        let mut networks: Vec<WifiNetwork> = scan_results
            .iter()
            .cloned()
            .map(WifiNetwork::from)
            .collect();

        if let Ok(status) = self.requester.get_status().await {
            if let Some(active_bssid) = status.get("bssid") {
                for net in &mut networks {
                    if net.has_bssid(active_bssid) {
                        net.set_online();
                    }
                }
            }
        }
        Ok(networks)
    }

    async fn connect(&self, auth: WifiAuth) -> WifiResult<()> {
        // 1. Clean up existing profiles
        // wpa_supplicant persists networks; we clear them to ensure a fresh connection
        let existing = self.requester.get_networks().await?;
        for net in existing {
            // The field is specifically 'network_id' in wifi_ctrl
            let _ = self.requester.remove_network(net.network_id).await;
        }

        // 2. Create a new network block
        let id = self.requester.add_network().await?;

        // 3. Configure SSID
        // We use the auth.ssid() helper which returns Option<&str> or &str
        if let Some(ssid) = auth.ssid() {
            self.requester.set_network_ssid(id, ssid.into()).await?;
        }

        // 4. Configure Security
        if let Some(psk) = auth.psk() {
            // wpa_supplicant requires the PSK to be wrapped in quotes (handled by crate)
            self.requester.set_network_psk(id, psk.into()).await?;
        } else {
            // For Open networks, we must explicitly set key management to NONE
            self.requester
                .set_network_keymgmt(id, sta::KeyMgmt::None)
                .await?;
        }

        // 5. Connect
        // select_network disables all other blocks and triggers the association
        self.requester.select_network(id).await?;

        Ok(())
    }

    async fn disconnect(&self) -> WifiResult<()> {
        // Fetch all known networks and remove them.
        // This forces wpa_supplicant to disconnect.
        let networks = self.requester.get_networks().await?;
        for net in networks {
            let _ = self.requester.remove_network(net.network_id).await;
        }

        Ok(())
    }
}
