use std::fmt::Display;

use bevy_replicon::{ReplicationPlugins, client::ClientPlugin, server::ServerPlugin, replicon_core::replication_rules::MapNetworkEntities};
use position::Position;
use serde::{Deserialize, Serialize};
use bevy::{prelude::*, log::{LogPlugin, Level}, window::CursorGrabMode};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use enemy::EnemySpawner;
use player_controller::{Cursor, CursorSprite, PlayerController};
use projectile::{Projectile, ProjectileHits};
use wasm_peers_rtc::client::WebRtcBrowser;
#[cfg(target_arch = "wasm32")]
use web_sys::window;
use clap::{Parser, ValueEnum};

use crate::{
    enemy::EnemyPlugin, player::PlayerPlugin, player_controller::PlayerControllerPlugin,
    world::WorldPlugin, projectile::ProjectilePlugin, wasm_peers_rtc::{browser::WebRtcBrowserPlugin, client::{WebRtcBrowserState, WebRtcClientPlugin}, server::{WebRtcServer, WebRtcServerPlugin}}, position::PositionPlugin,
};

mod enemy;
mod player;
mod player_controller;
mod projectile;
mod world;
mod wasm_peers_rtc;
mod position;

#[derive(Serialize, Deserialize, Debug, Default, Resource, Parser, Clone, ValueEnum)]
enum MultiplayerType {
    DedicatedServer,
    Server,
    Client,
    #[default]
    Singleplayer
}

impl Display for MultiplayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiplayerType::DedicatedServer => f.write_str("Dedicated Server"),
            MultiplayerType::Server => f.write_str("Local Server"),
            MultiplayerType::Client => f.write_str("Local Client"),
            MultiplayerType::Singleplayer => f.write_str("Singleplayer"),
        }
    }
}

impl MultiplayerType {
    pub fn is_playable(&self) -> bool {
        match self {
            MultiplayerType::DedicatedServer => false,
            MultiplayerType::Server | MultiplayerType::Client | MultiplayerType::Singleplayer => true,
        }
    }
    pub fn is_client(&self) -> bool {
        match self {
            MultiplayerType::DedicatedServer | MultiplayerType::Server | MultiplayerType::Singleplayer => false,
            MultiplayerType::Client => true,
        }
    }
    pub fn is_server(&self) -> bool {
        match self {
            MultiplayerType::DedicatedServer | MultiplayerType::Server => true,
            MultiplayerType::Client | MultiplayerType::Singleplayer => false,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Resource, Parser)]
struct Params {
    #[clap(default_value = "singleplayer", long = "type")]
    #[serde(default = "default_type")]
    r#type: MultiplayerType,
    #[clap(long = "username")]
    username: String
}

fn default_type() -> MultiplayerType { MultiplayerType::Singleplayer }

fn main() {
    println!("Hello, world!");

    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    #[cfg(not(target_arch = "wasm32"))]
    let params = {
        let params = Params::parse();
        println!("Params: {:?}", params);
        params
    };

    #[cfg(target_arch = "wasm32")]
    let params = {
        let params: Params = serde_urlencoded::from_str(window().unwrap().document().unwrap().location().unwrap().search().unwrap().split_at(1).1).unwrap();
        info!("Params: {:?}", params);
        params
    };

    #[cfg(debug_assertions)]
    let canvas = None;
    #[cfg(not(debug_assertions))]
    let canvas = Some("#canvas".to_string());

    let is_server = params.r#type.is_server();
    let mut app = App::new();
    app.add_plugins(LogPlugin {filter: "wgpu_hal=off,wgpu_core=off,some-game=info".to_string(), level: Level::INFO});
    if !params.r#type.is_playable() {
        app.add_plugins(MinimalPlugins);
    } else {
        app.add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    cursor: bevy::window::Cursor { visible: false, grab_mode: CursorGrabMode::None, ..default() },
                    canvas,
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .disable::<LogPlugin>());
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }
        app.add_plugins(PlayerControllerPlugin {});

        #[cfg(debug_assertions)]
        app.add_plugins(WorldInspectorPlugin::new());
    }
    if params.r#type.is_server() {
        app.add_plugins(ReplicationPlugins.build().disable::<ClientPlugin>());
    } else if params.r#type.is_client() {
        app.add_plugins(ReplicationPlugins.build().disable::<ServerPlugin>());
    }
    app.add_plugins((
        PlayerPlugin {is_server},
        WorldPlugin {is_server},
        EnemyPlugin {is_server},
        ProjectilePlugin {is_server},
        PositionPlugin {is_server},
    ));
    if params.r#type.is_server() {
        app.add_plugins(WebRtcServerPlugin {is_headless: is_server});
    } else if params.r#type.is_client() {
        app.add_plugins(WebRtcClientPlugin {is_headless: !params.r#type.is_playable()});
        app.add_plugins(WebRtcBrowserPlugin {});
    }
    app.insert_resource(params);
    if is_server {
        app.add_systems(Startup, setup_server);
        app.add_systems(Startup, server_open_server);
    } else {
        app.add_systems(Startup, setup_client);
        app.add_systems(Update, player_shoot);
        app.add_systems(Startup, client_open_browser);
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

fn setup_client(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn client_open_browser(world: &mut World) {
    info!("Opening browser...");
    world.insert_non_send_resource(WebRtcBrowser::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers".to_owned()));
}

fn server_open_server(world: &mut World) {
    info!("Opening server...");
    world.insert_non_send_resource(WebRtcServer::new("wss://rose-signalling.webpubsub.azure.com/client/hubs/onlineservers".to_owned(), "my-server".to_owned(), "some-game".to_owned()));
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
