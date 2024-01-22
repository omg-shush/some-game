use bevy::prelude::*;

use crate::wasm_peers_rtc::client::WebRtcBrowser;

use super::client::{WebRtcBrowserState, WebRtcClientState};

pub struct WebRtcBrowserPlugin {}

impl Plugin for WebRtcBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WebRtcBrowserState::Connected), Self::setup);
        app.add_systems(Update, Self::client_open_client.run_if(in_state(WebRtcBrowserState::Connected)));
        app.add_systems(OnEnter(WebRtcClientState::Connected), Self::teardown);
    }
}

impl WebRtcBrowserPlugin {
    fn setup() {

    }

    fn teardown() {

    }

    fn client_open_client(world: &mut World) {
        let mut browser = world.non_send_resource_mut::<WebRtcBrowser>();
        let servers = browser.servers().unwrap();
        if servers.len() > 0 {
            let (server_connection, server_entry) = servers.into_iter().next().unwrap(); // Pick first server in the list for now
            info!("Client: connecting to server {:?} @ {}", server_entry, server_connection);
            let browser = world.remove_non_send_resource::<WebRtcBrowser>().unwrap();
            let client = browser.connect(server_connection);
            world.insert_non_send_resource(client);
            return;
        }
        info!("No servers online yet...");
        *browser = WebRtcBrowser::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers".to_owned()); // TODO proper refresh()
    }
}
