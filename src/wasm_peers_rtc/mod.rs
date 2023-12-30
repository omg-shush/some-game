use std::{rc::Rc, cell::RefCell, collections::HashMap};

use bevy::{prelude::*, utils::HashSet};
use bevy_replicon::prelude::*;
use js_sys::Promise;
use renet::{RenetServer, ConnectionConfig, RenetClient, ClientId};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use webrtc::{WebRtcBrowser, WebRtcServer};
use util::*;

use self::{webrtc::WebRtcClient, signaling::ConnectionId};

mod callback_channel;
mod deque_channel;
mod webrtc;
mod signaling;
pub mod util;

pub struct WasmPeersRtcPlugin {
    pub is_server: bool
}

impl Plugin for WasmPeersRtcPlugin {
    fn build(&self, app: &mut App) {
        if self.is_server {
            app.add_plugins(WasmPeersRtcServerPlugin {});
        } else { // is_client
            app.add_plugins(WasmPeersRtcClientPlugin {});
        }
    }
}

struct WasmPeersRtcServerPlugin {}

impl Plugin for WasmPeersRtcServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup);
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
    fn setup(world: &mut World) {
        let rtc_server = Rc::new(RefCell::new(None));
        let rtc_server_clone = rtc_server.clone();
        spawn_local(async move {
            server(rtc_server_clone).await.expect("Server not OK");
        });
        world.insert_non_send_resource(rtc_server);
        world.insert_non_send_resource::<HashMap<ClientId, ConnectionId>>(HashMap::new());
        world.insert_non_send_resource::<HashMap<ConnectionId, ClientId>>(HashMap::new());
    }

    fn update(
        rtc_server: NonSendMut<Rc<RefCell<Option<WebRtcServer>>>>,
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

struct WasmPeersRtcClientPlugin {}

impl Plugin for WasmPeersRtcClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup);
        app.add_systems(Update, Self::update);
        let network_channels = app.world.resource::<NetworkChannels>();
        let connection_config = ConnectionConfig {
            server_channels_config: network_channels.get_server_configs(),
            client_channels_config: network_channels.get_client_configs(),
            ..Default::default()
        };
        let client = RenetClient::new(connection_config);
        app.insert_resource(client);
    }
}

impl WasmPeersRtcClientPlugin {
    fn setup(world: &mut World) {
        let rtc_client = Rc::new(RefCell::new(None));
        let rtc_client_clone = rtc_client.clone();
        spawn_local(async move {
            client(rtc_client_clone).await.expect("Client not OK");
        });
        world.insert_non_send_resource(rtc_client);
    }

    fn update(
        rtc_client: NonSendMut<Rc<RefCell<Option<WebRtcClient>>>>,
        mut renet_client: ResMut<RenetClient>
    ) {
        // TODO handle .set_connecting()
        if let Some(rtc_client) = rtc_client.borrow_mut().as_mut() {
            renet_client.set_connected();

            // TODO handle transport-disconnect
            
            // Handle incoming packets
            let packets: Vec<Vec<u8>> = rtc_client.channel().drain().unwrap();
            for packet in packets {
                renet_client.process_packet(&packet);
            }

            // Handle outgoing packets
            let packets = renet_client.get_packets_to_send();
            for packet in packets {
                rtc_client.channel().send(packet).unwrap();
            }
        }
    }
}

async fn client(result: Rc<RefCell<Option<WebRtcClient>>>) -> Result<JsValue, JsValue> {
    let (browser, server_id) = loop {
        let browser = WebRtcBrowser::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers").await?;
        let next = browser.iter().next();
        if let Some((server_id, server_entry)) = next {
            console_log!("Client: connecting to server {:?} @ {}", server_entry, server_id);
            let server_id = server_id.to_owned();
            break (browser, server_id);
        }
        console_log!("No servers available yet...");
        JsFuture::from(Promise::new(&mut |resolve, _| setTimeout(resolve, 5000))).await.unwrap();
    };
    let client = browser.connect(server_id.to_owned()).await?;
    *result.borrow_mut() = Some(client);
    console_log!("Client: connected!");
    Ok(JsValue::undefined())
}

async fn server(result: Rc<RefCell<Option<WebRtcServer>>>) -> Result<JsValue, JsValue> {
    let game = "wasm-rtc-test";
    let name = "my-server";
    console_log!("\t\tServer: Registering... game: {}, name: {}", game, name);
    let server = WebRtcServer::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers", game, name).await?;
    *result.borrow_mut() = Some(server);
    console_log!("\t\tServer: Registered.");
    Ok(JsValue::undefined())
}
