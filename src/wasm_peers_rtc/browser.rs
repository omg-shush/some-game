use std::collections::HashMap;

use bevy::{ecs::system::SystemState, prelude::*};
use bevy_egui::{egui, EguiContexts};

use crate::wasm_peers_rtc::client::WebRtcBrowser;

use super::{client::{WebRtcBrowserState, WebRtcClientState}, signaling::ServerEntry};

pub struct WebRtcBrowserPlugin {}

#[derive(Resource)]
struct Servers {
    servers: HashMap<String, ServerEntry>
}

#[derive(Event)]
struct ConnectEvent {
    conn: String
}

impl Plugin for WebRtcBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ConnectEvent>();
        app.add_systems(OnEnter(WebRtcBrowserState::Connected), Self::setup);
        app.add_systems(Update, Self::browser_get_servers.run_if(in_state(WebRtcBrowserState::Connected)));
        app.add_systems(Update, Self::browser_show_servers.run_if(resource_exists::<Servers>()));
        app.add_systems(OnEnter(WebRtcClientState::Connected), Self::teardown);
    }
}

impl WebRtcBrowserPlugin {
    fn setup() {

    }

    fn teardown(world: &mut World) {
        world.remove_resource::<Servers>();
    }

    fn browser_get_servers(world: &mut World) {
        if world.contains_resource::<Servers>() {
            let mut events = SystemState::<EventReader<ConnectEvent>>::new(world);
            let mut reader = events.get_mut(world);
            if let Some(e) = reader.read().last() {
                let server_id = e.conn.to_owned();
                let browser = world.remove_non_send_resource::<WebRtcBrowser>().unwrap();
                let servers = browser.servers().unwrap();
                let entry = servers.get(&server_id).unwrap();
                info!("Client: connecting to server {:?} @ {}", entry, server_id);
                let client = browser.connect(server_id);
                world.insert_non_send_resource(client);
            }
            return;
        }
        let mut browser = world.non_send_resource_mut::<WebRtcBrowser>();
        let servers = browser.servers().unwrap();
        if servers.len() > 0 {
            world.insert_resource(Servers { servers });
            return;
        }
        info!("No servers online yet...");
        *browser = WebRtcBrowser::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers".to_owned()); // TODO proper refresh() with delay
    }

    fn browser_show_servers(servers: Res<Servers>, mut contexts: EguiContexts, mut writer: EventWriter<ConnectEvent>) {
        contexts.ctx_mut().set_visuals(egui::Visuals {
            window_rounding: 0.0.into(),
            ..default()
        });
        egui::Window::new("Servers").show(contexts.ctx_mut(), |ui| {
            egui::Grid::new("serverlist").show(ui, |ui| {
                for (conn, server) in servers.servers.iter() {
                    ui.label(&server.game);
                    ui.label(&server.name);
                    ui.label(conn);
                    if ui.button("Connect").clicked() {
                        writer.send(ConnectEvent { conn: conn.to_owned() })
                    }
                    ui.end_row();
                }
            });
        });
    }
}
