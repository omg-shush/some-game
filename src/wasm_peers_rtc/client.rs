use std::{rc::Rc, cell::RefCell, collections::HashMap};

use bevy::prelude::*;
use bevy_inspector_egui::quick::StateInspectorPlugin;
use bevy_replicon::replicon_core::NetworkChannels;
use renet::{RenetClient, ConnectionConfig};
use wasm_bindgen_futures::spawn_local;

use crate::wasm_peers_rtc::{js_warn, util::{console_log, js_log}};
use super::{webrtc::{AsyncWebRtcBrowser, AsyncWebRtcClient}, util::console_warn, signaling::{ServerEntry, ConnectionId}, callback_channel::SendRecvCallbackChannel};

pub struct WebRtcClientPlugin {
    pub is_headless: bool
}

impl Plugin for WebRtcClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<WebRtcBrowserState>();
        app.add_state::<WebRtcClientState>();
        #[cfg(debug_assertions)]
        if !self.is_headless {
            app.add_plugins(StateInspectorPlugin::<WebRtcBrowserState>::new());
            app.add_plugins(StateInspectorPlugin::<WebRtcClientState>::new());
        }
        app.add_systems(PreUpdate, (Self::update_browser_state, Self::update_client_state));
        app.add_systems(OnEnter(WebRtcClientState::Connected), Self::client_connected);
        app.add_systems(Update, Self::update_client_packets.run_if(in_state(WebRtcClientState::Connected))); // TODO fix 1 frame delay by moving to PreUpdate, using system sets to run after update_client_state but before renet handles packets
    }
}

type BrowserExists = bool;
type ClientExists = bool;

impl WebRtcClientPlugin {
    fn update_browser_state(
        browser: Option<NonSendMut<WebRtcBrowser>>,
        mut exists: Local<BrowserExists>,
        state: Res<State<WebRtcBrowserState>>,
        mut next_state: ResMut<NextState<WebRtcBrowserState>>
    ) {
        let removed = browser.is_none() && *exists;
        let changed = browser.as_ref().map_or(false, |b| b.is_changed());
        if removed || changed || *state == WebRtcBrowserState::Connecting {
            let next = match &browser {
                Some(b) if b.servers().is_some() => WebRtcBrowserState::Connected,
                Some(_) => WebRtcBrowserState::Connecting,
                None => WebRtcBrowserState::Disconnected,
            };
            if *state.get() != next {
                next_state.set(next);
            }
        }
        // console_log!("WebRtcBrowser state: {:?}, next state: {:?}, servers: {:?}", state, next_state, browser.as_ref().map(|b| b.servers()));
        *exists = browser.is_some();
    }

    fn update_client_state(
        client: Option<NonSendMut<WebRtcClient>>,
        mut exists: Local<ClientExists>,
        state: Res<State<WebRtcClientState>>,
        mut next_state: ResMut<NextState<WebRtcClientState>>
    ) {
        let removed = client.is_none() && *exists;
        let changed = client.as_ref().map_or(false, |b| b.is_changed());
        if removed || changed || *state == WebRtcClientState::Connecting {
            let next = match &client {
                Some(c) if c.channel().is_some() => WebRtcClientState::Connected,
                Some(_) => WebRtcClientState::Connecting,
                None => WebRtcClientState::Disconnected,
            };
            if *state.get() != next {
                next_state.set(next);
            }
        }
        *exists = client.is_some();
    }

    fn client_connected(world: &mut World) {
        let network_channels = world.resource::<NetworkChannels>();
        let connection_config = ConnectionConfig {
            server_channels_config: network_channels.get_server_configs(),
            client_channels_config: network_channels.get_client_configs(),
            ..Default::default()
        };
        let client = RenetClient::new(connection_config);
        world.insert_resource(client);
    }

    fn update_client_packets(
        rtc_client: NonSendMut<WebRtcClient>,
        mut renet_client: ResMut<RenetClient>
    ) {
        // TODO handle .set_connecting()?
        renet_client.set_connected();

        // TODO handle transport-disconnect
        
        // Handle incoming packets
        let packets: Vec<Vec<u8>> = rtc_client.channel().unwrap().drain().unwrap();
        for packet in packets {
            renet_client.process_packet(&packet);
        }

        // Handle outgoing packets
        let packets = renet_client.get_packets_to_send();
        for packet in packets {
            rtc_client.channel().unwrap().send(packet).unwrap();
        }
    }
}

#[derive(States, Debug, Default, Hash, Eq, PartialEq, Clone, Reflect)]
pub enum WebRtcBrowserState {
    #[default]
    Disconnected,
    Connecting,
    Connected
}

#[derive(States, Debug, Default, Hash, Eq, PartialEq, Clone, Reflect)]
pub enum WebRtcClientState {
    #[default]
    Disconnected,
    Connecting,
    Connected
}

#[derive(Clone)]
pub struct WebRtcBrowser {
    browser: Rc<RefCell<Option<AsyncWebRtcBrowser>>>
}

impl WebRtcBrowser {
    pub fn new(signaling_url: String) -> WebRtcBrowser {
        let browser = WebRtcBrowser { browser: Rc::new(RefCell::new(None)) };
        let browser_clone = browser.clone();
        spawn_local(async move {
            match AsyncWebRtcBrowser::new(&signaling_url).await {
                Ok(b) => {console_log!("Storing Browser result"); *browser_clone.browser.borrow_mut() = Some(b)},
                Err(e) => console_warn!("Error creating AsyncWebRtcBrowser: {:?}", e),
            }
        });
        browser
    }

    pub fn servers(&self) -> Option<HashMap<ConnectionId, ServerEntry>> {
        self.browser.borrow().as_ref().map(|browser| browser.servers())
    }

    pub fn connect(self, server_id: ConnectionId) -> WebRtcClient {
        let client = WebRtcClient { client: Rc::new(RefCell::new(None)) };
        let client_clone = client.clone();
        self.browser.take().map(|browser| {
            spawn_local(async move {
                match browser.connect(server_id).await {
                    Ok(c) => *client_clone.client.borrow_mut() = Some(c),
                    Err(e) => console_warn!("Error creating AsyncWebRtcClient: {:?}", e),
                }
            })
        });
        client
    }
}

#[derive(Clone)]
pub struct WebRtcClient {
    client: Rc<RefCell<Option<AsyncWebRtcClient>>>
}

impl WebRtcClient {
    pub fn channel(&self) -> Option<SendRecvCallbackChannel> {
        self.client.borrow().as_deref().cloned()
    }
}
