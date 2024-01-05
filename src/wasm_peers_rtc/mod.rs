use std::{rc::Rc, cell::RefCell, collections::HashMap};

use bevy::{prelude::*, utils::HashSet};
use bevy_replicon::prelude::*;
use js_sys::Promise;
use renet::{RenetServer, ConnectionConfig, RenetClient, ClientId};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use webrtc::{AsyncWebRtcBrowser, AsyncWebRtcServer};
use util::*;

use self::{webrtc::AsyncWebRtcClient, signaling::ConnectionId};

mod callback_channel;
mod deque_channel;
mod webrtc;
mod signaling;
pub mod util;
pub mod client;
pub mod server;

pub struct WasmPeersRtcPlugin {
    pub is_server: bool,
    pub game_name: String,
    pub server_name: String,
}

impl Plugin for WasmPeersRtcPlugin {
    fn build(&self, app: &mut App) {
        if self.is_server {
            app.add_plugins(WasmPeersRtcServerPlugin {game_name: self.game_name.to_owned()});
        }
    }
}

struct WasmPeersRtcServerPlugin {
    game_name: String
}

impl Plugin for WasmPeersRtcServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup(self.game_name.to_owned()));
        app.add_systems(Update, Self::update);
        let network_channels = app.world.resource::<NetworkChannels>();
        let connection_config = ConnectionConfig {
            server_channels_config: network_channels.get_server_configs(),
            client_channels_config: network_channels.get_client_configs(),
            ..Default::default()
        };
        let server = RenetServer::new(connection_config);
        app.insert_resource(server);
    }
}

impl WasmPeersRtcServerPlugin {
    fn setup(game_name: String) -> impl FnMut(&mut World) {
        move |world: &mut World| {
            let rtc_server = Rc::new(RefCell::new(None));
            let rtc_server_clone = rtc_server.clone();
            let game_name = game_name.clone();
            spawn_local(async {
                server(game_name, "my-server".to_owned(), rtc_server_clone).await.expect("Server not OK");
            });
            world.insert_non_send_resource(rtc_server);
            world.insert_non_send_resource::<HashMap<ClientId, ConnectionId>>(HashMap::new());
            world.insert_non_send_resource::<HashMap<ConnectionId, ClientId>>(HashMap::new());
        }
    }

    fn update(
        rtc_server: NonSendMut<Rc<RefCell<Option<AsyncWebRtcServer>>>>,
        mut renet_server: ResMut<RenetServer>,
        mut client_to_connection: NonSendMut<HashMap<ClientId, ConnectionId>>,
        mut connection_to_client: NonSendMut<HashMap<ConnectionId, ClientId>>
    ) {
        if let Some(rtc_server) = rtc_server.borrow_mut().as_mut() {
            // Handle new clients
            for new_client in rtc_server.new_clients.borrow_mut().drain(..) {
                let client_id = ClientId::from_raw(renet_server.connected_clients() as u64 + 1);
                client_to_connection.insert(client_id, new_client.clone());
                connection_to_client.insert(new_client, client_id);
                renet_server.add_connection(client_id);
            }

            // Handle transport-disconnected clients
            let mut disconnect = HashSet::new();
            for (connection, channel) in rtc_server.clients.borrow_mut().iter() {
                if channel.is_closed() {
                    disconnect.insert(connection.to_owned());
                }
            }

            // Handle incoming packets
            for (connection, channel) in rtc_server.clients.borrow_mut().iter_mut() {
                let client_id = connection_to_client.get(connection).unwrap();
                let packets: Vec<Vec<u8>> = channel.drain().unwrap();
                for packet in packets {
                    renet_server.process_packet_from(&packet, *client_id).unwrap();
                }
            }

            // Handle outgoing packets
            for client_id in renet_server.clients_id() {
                let connection_id = client_to_connection.get(&client_id).unwrap();
                let packets = renet_server.get_packets_to_send(client_id).unwrap();
                if let Some(connection) = rtc_server.clients.borrow_mut().get_mut(connection_id) {
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
                rtc_server.clients.borrow_mut().remove(&connection);
                let client = *connection_to_client.get(&connection).unwrap();
                renet_server.remove_connection(client);
                connection_to_client.remove(&connection);
                client_to_connection.remove(&client);
            }
        }
    }
}

async fn server(game_name: String, server_name: String, result: Rc<RefCell<Option<AsyncWebRtcServer>>>) -> Result<JsValue, JsValue> {
    console_log!("\t\tServer: Registering... game: {}, name: {}", game_name, server_name);
    let server = AsyncWebRtcServer::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers", &game_name, &server_name).await?;
    *result.borrow_mut() = Some(server);
    console_log!("\t\tServer: Registered.");
    Ok(JsValue::undefined())
}
