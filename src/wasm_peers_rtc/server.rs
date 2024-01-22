use std::{cell::RefCell, collections::HashMap, rc::Rc};

use bevy::{prelude::*, utils::HashSet};
use bevy_inspector_egui::quick::StateInspectorPlugin;
use bevy_replicon::replicon_core::NetworkChannels;
use renet::{ClientId, ConnectionConfig, RenetServer};
use wasm_bindgen_futures::spawn_local;

use crate::wasm_peers_rtc::{console_log, js_log};

use super::{callback_channel::SendRecvCallbackChannel, signaling::ConnectionId, webrtc::AsyncWebRtcServer};

pub struct WebRtcServerPlugin {
    pub is_headless: bool
}

impl Plugin for WebRtcServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<WebRtcServerState>();
        #[cfg(debug_assertions)]
        if !self.is_headless {
            app.add_plugins(StateInspectorPlugin::<WebRtcServerState>::new());
        }
        app.add_systems(PreUpdate, Self::update_server_state);
        app.add_systems(OnExit(WebRtcServerState::Offline), Self::server_online);
        app.add_systems(Update, Self::update_server_clients.run_if(not(in_state(WebRtcServerState::Offline)))); // TODO fix 1 frame delay by moving to PreUpdate, using system sets to run after update_server_state but before renet handles packets
    }
}

type ServerExists = bool;

impl WebRtcServerPlugin {
    fn update_server_state(
        server: Option<NonSendMut<WebRtcServer>>,
        mut exists: Local<ServerExists>,
        state: Res<State<WebRtcServerState>>,
        mut next_state: ResMut<NextState<WebRtcServerState>>
    ) {
        let removed = server.is_none() && *exists;
        let changed = server.as_ref().map_or(false, |s| s.is_changed());
        if removed || changed || *state == WebRtcServerState::Registering || *state == WebRtcServerState::Registered {
            let next = match &server {
                Some(s) if s.is_listening() => WebRtcServerState::Listening,
                Some(s) if s.server.borrow().is_some() => WebRtcServerState::Registered,
                Some(_) => WebRtcServerState::Registering,
                None => WebRtcServerState::Offline,
            };
            if *state.get() != next {
                next_state.set(next);
            }
        }
        *exists = server.is_some();
    }

    fn server_online(world: &mut World) {
        let network_channels = world.resource::<NetworkChannels>();
        let connection_config = ConnectionConfig {
            server_channels_config: network_channels.get_server_configs(),
            client_channels_config: network_channels.get_client_configs(),
            ..Default::default()
        };
        let server = RenetServer::new(connection_config);
        world.insert_resource(server);
    }

    fn update_server_clients(
        mut rtc_server: NonSendMut<WebRtcServer>,
        mut renet_server: ResMut<RenetServer>,
    ) {
        let mut client_change = false;

        // Handle new clients
        for new_client in rtc_server.new_clients() {
            let client_id: ClientId = ClientId::from_raw(rtc_server.next_client_id); // TODO reuse disconnected IDs if we don't expect them to reconnect
            rtc_server.next_client_id += 1;
            rtc_server.client_to_connection.borrow_mut().insert(client_id, new_client.clone());
            rtc_server.connection_to_client.borrow_mut().insert(new_client, client_id);
            renet_server.add_connection(client_id);
            client_change = true;
        }

        // Handle transport-disconnected clients
        let mut disconnect = HashSet::new();
        for (connection, channel) in rtc_server.clients() {
            if channel.is_closed() {
                disconnect.insert(connection.to_owned());
                client_change = true;
            }
        }

        // Handle incoming packets
        for (connection, mut channel) in rtc_server.clients() {
            let client_id = rtc_server.connection_to_client.borrow().get(&connection).unwrap().to_owned();
            let packets: Vec<Vec<u8>> = channel.drain().unwrap();
            for packet in packets {
                renet_server.process_packet_from(&packet, client_id).unwrap();
            }
        }

        // Handle outgoing packets
        for client_id in renet_server.clients_id() {
            let connection_id = rtc_server.client_to_connection.borrow().get(&client_id).unwrap().to_owned();
            let packets = renet_server.get_packets_to_send(client_id).unwrap();
            if let Some(connection) = rtc_server.clients().get_mut(&connection_id) {
                if !connection.is_closed() {
                    for packet in packets {
                        if let Err(_) = connection.send(packet) {
                            disconnect.insert(connection_id.to_owned());
                            break;
                        }
                    }
                }
            }
        }

        if disconnect.len() > 0 {
            console_log!("Closing {:?}", disconnect);
        }
        for connection in disconnect {
            rtc_server.remove_client(&connection);
            let client = *rtc_server.connection_to_client.borrow().get(&connection).unwrap();
            renet_server.remove_connection(client);
            rtc_server.connection_to_client.borrow_mut().remove(&connection);
            rtc_server.client_to_connection.borrow_mut().remove(&client);
        }

        if client_change {
            console_log!("Current connections: {:?}", rtc_server.clients().keys().collect::<Vec<_>>());
        }
    }
}

#[derive(States, Debug, Default, Hash, Eq, PartialEq, Clone, Reflect)]
pub enum WebRtcServerState {
    #[default]
    Offline,
    Registering,
    Registered,
    Listening
}

#[derive(Clone)]
pub struct WebRtcServer {
    server: Rc<RefCell<Option<AsyncWebRtcServer>>>,
    client_to_connection: Rc<RefCell<HashMap<ClientId, ConnectionId>>>,
    connection_to_client: Rc<RefCell<HashMap<ConnectionId, ClientId>>>,
    next_client_id: u64,
}

impl WebRtcServer {
    pub fn new(signaling_url: String, game_name: String, server_name: String) -> WebRtcServer {
        let server = WebRtcServer {
            server: Rc::new(RefCell::new(None)),
            client_to_connection: Rc::new(RefCell::new(HashMap::new())),
            connection_to_client: Rc::new(RefCell::new(HashMap::new())),
            next_client_id: 1,
        };
        let server_clone = server.clone();
        spawn_local(async move {
            match AsyncWebRtcServer::new(&signaling_url, &game_name, &server_name).await {
                Ok(s) => *server_clone.server.borrow_mut() = Some(s),
                Err(e) => warn!("Error creating AsyncWebRtcServer: {:?}", e),
            }
        });
        server
    }

    pub fn is_listening(&self) -> bool {
        self.server.borrow().is_some() // TODO: is_some_and(AsyncServer.is_listening)
    }

    pub fn clients(&self) -> HashMap<ConnectionId, SendRecvCallbackChannel> {
        self.server.borrow().as_ref().map_or(HashMap::new(), |s| s.clients.borrow().clone())
    }

    pub fn remove_client(&mut self, connection: &str) {
        if let Some(s) = self.server.borrow_mut().as_mut() {
            s.clients.borrow_mut().remove(connection);
        }
    }

    pub fn new_clients(&mut self) -> Vec<ConnectionId> {
        self.server.borrow().as_ref().map_or(Vec::new(), |s| s.new_clients.borrow_mut().drain(..).collect())
    }
}
