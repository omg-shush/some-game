use std::env;
use bevy_replicon::{ReplicationPlugins, client::ClientPlugin, server::{ServerPlugin, ServerSet}, replicon_core::replication_rules::MapNetworkEntities};
use player::Player;
use position::Position;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use bevy::{prelude::*, log::{LogPlugin, Level}, window::CursorGrabMode};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use enemy::EnemySpawner;
use player_controller::{Cursor, CursorSprite, PlayerController};
use projectile::{Projectile, ProjectileHits};
use wasm_peers_rtc::client::WebRtcBrowser;
use web_sys::window;
use clap::Parser;

use crate::{
    enemy::EnemyPlugin, player::PlayerPlugin, player_controller::PlayerControllerPlugin,
    world::WorldPlugin, projectile::ProjectilePlugin, wasm_peers_rtc::{WasmPeersRtcPlugin, util::{console_log, js_log}, client::{WebRtcClient, WebRtcBrowserState, WebRtcClientPlugin}}, position::PositionPlugin,
};

mod enemy;
mod player;
mod player_controller;
mod projectile;
mod world;
mod wasm_peers_rtc;
mod position;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[derive(Serialize, Deserialize, Default, Debug, Resource, Parser)]
struct Params {
    #[clap(default_value_t = false)]
    #[serde(default = "default_is_server")]
    is_server: bool,
    username: String
}

fn default_is_server() -> bool { false }

fn main() {
    println!("Hello, world!");

    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    #[cfg(not(target_arch = "wasm32"))]
    let params = {
        let params = Params::parse();
        println!("Server: {:?}", params);
        params
    };

    #[cfg(target_arch = "wasm32")]
    let params = {
        let params: Params = serde_urlencoded::from_str(window().unwrap().document().unwrap().location().unwrap().search().unwrap().split_at(1).1).unwrap();
        log(&format!("Server: {:?}", params));
        params
    };

    #[cfg(debug_assertions)]
    let canvas = None;
    #[cfg(not(debug_assertions))]
    let canvas = Some("#canvas".to_string());

    let is_server = params.is_server;
    let mut app = App::new();
    if is_server {
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ReplicationPlugins.build().disable::<ClientPlugin>());
    } else {
        app.add_plugins(DefaultPlugins
            .set(LogPlugin {filter: "wgpu_hal=off".to_string(), level: Level::WARN})
            .set(WindowPlugin {
                primary_window: Some(Window {
                    cursor: bevy::window::Cursor { visible: false, grab_mode: CursorGrabMode::None, ..default() },
                    canvas,
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }));
        app.add_plugins(ReplicationPlugins.build().disable::<ServerPlugin>());
        #[cfg(debug_assertions)]
        app.add_plugins(WorldInspectorPlugin::new());
        app.add_plugins(PlayerControllerPlugin {});
    }
    app.add_plugins((
        PlayerPlugin {is_server},
        WorldPlugin {is_server},
        EnemyPlugin {is_server},
        ProjectilePlugin {is_server},
        PositionPlugin {is_server},
    ));
    app.add_plugins(WasmPeersRtcPlugin {is_server, game_name: "some-game".to_owned(), server_name: "my-server".to_owned()}); // After all client events registered
    app.add_plugins(WebRtcClientPlugin {is_headless: is_server});
    app.insert_resource(params);
    if is_server {
        app.add_systems(Startup, setup_server);
    } else {
        app.add_systems(Startup, setup_client);
        app.add_systems(Update, player_shoot);
        app.add_systems(Startup, client_open_browser);
        app.add_systems(Update, client_open_client.run_if(in_state(WebRtcBrowserState::Connected)));
    }

    app.run();
}

fn setup_server(mut commands: Commands) {
    commands.spawn((
        EnemySpawner {
            image: "enemy.png".to_owned(),
            timer: Timer::from_seconds(2., TimerMode::Repeating),
        },
        Position::from_translation(Vec3::new(300., 0., 0.5)),
    ));
    commands.spawn((
        EnemySpawner {
            image: "enemy.png".to_owned(),
            timer: Timer::from_seconds(3., TimerMode::Repeating),
        },
        Position::from_translation(Vec3::new(-300., 400., 0.5)),
    ));
    commands.spawn((
        EnemySpawner {
            image: "enemy.png".to_owned(),
            timer: Timer::from_seconds(4., TimerMode::Repeating),
        },
        Position::from_translation(Vec3::new(400., -300., 0.5)),
    ));
    commands.spawn((
        EnemySpawner {
            image: "enemy.png".to_owned(),
            timer: Timer::from_seconds(5., TimerMode::Repeating),
        },
        Position::from_translation(Vec3::new(-600., -100., 0.5)),
    ));
}

fn setup_client(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
}

fn client_open_browser(world: &mut World) {
    console_log!("Opening browser...");
    world.insert_non_send_resource(WebRtcBrowser::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers".to_owned()));
}

fn client_open_client(world: &mut World) {
    let mut browser = world.non_send_resource_mut::<WebRtcBrowser>();
    let servers = browser.servers().unwrap();
    if servers.len() > 0 {
        let (server_connection, server_entry) = servers.into_iter().next().unwrap();
        console_log!("Client: connecting to server {:?} @ {}", server_entry, server_connection);
        let browser = world.remove_non_send_resource::<WebRtcBrowser>().unwrap();
        let client = browser.connect(server_connection);
        world.insert_non_send_resource(client);
        return;
    }
    console_log!("No servers online yet...");
    *browser = WebRtcBrowser::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers".to_owned()); // TODO proper refresh()
}

#[derive(Event, Serialize, Deserialize)]
struct PlayerShootEvent {
    projectile: Projectile
}

impl MapNetworkEntities for PlayerShootEvent {
    fn map_entities<T: bevy_replicon::prelude::Mapper>(&mut self, mapper: &mut T) {
        self.projectile.src = mapper.map(self.projectile.src);
    }
}

fn player_shoot(
    mut player: Query<(Entity, &mut PlayerController, &mut Position), Without<CursorSprite>>,
    cursor: Res<Cursor>,
    buttons: Res<Input<MouseButton>>,
    mut writer: EventWriter<PlayerShootEvent>
) {
    if player.is_empty() {
        return;
    }
    if buttons.just_pressed(MouseButton::Left) {
        if let Ok(player) = player.get_single_mut() {
            let position: Vec3 = player.2.translation;
            let destination = Vec3::new(cursor.pos.x, cursor.pos.y, position.z);
            let direction = (destination - position).normalize_or_zero();
            writer.send(PlayerShootEvent { projectile: Projectile {
                src: player.0,
                velocity: direction * 150.,
                hits: ProjectileHits::Enemy,
                initial_position: position,
                min_dist: -1,
            }});
        }
    }
}
