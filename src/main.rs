use bevy_replicon::{ReplicationPlugins, replicon_core::replication_rules::MapNetworkEntities};
use position::Position;
use serde::{Deserialize, Serialize};
use bevy::{prelude::*, log::{LogPlugin, Level}, window::CursorGrabMode};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use enemy::EnemySpawner;
use player_controller::{Cursor, CursorSprite, PlayerController};
use projectile::{Projectile, ProjectileHits};
use wasm_peers_rtc::client::WebRtcBrowser;
#[cfg(target_arch = "wasm32")]
use web_sys::window;

use crate::{
    enemy::EnemyPlugin, main_menu::{MainMenuPlugin, MainMenuState}, player::PlayerPlugin, player_controller::PlayerControllerPlugin, position::PositionPlugin, projectile::ProjectilePlugin, wasm_peers_rtc::{browser::WebRtcBrowserPlugin, client::WebRtcClientPlugin, server::{WebRtcServer, WebRtcServerPlugin}}, world::WorldPlugin
};

mod enemy;
mod player;
mod player_controller;
mod projectile;
mod world;
mod wasm_peers_rtc;
mod position;
mod main_menu;

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
enum MultiplayerType {
    #[default]
    Undecided,
    DedicatedServer,
    Server,
    Client,
    Singleplayer
}

impl MultiplayerType {
    pub fn is_playable(&self) -> bool {
        match self {
            MultiplayerType::DedicatedServer | MultiplayerType::Undecided => false,
            MultiplayerType::Server | MultiplayerType::Client | MultiplayerType::Singleplayer => true,
        }
    }
    pub fn state_is_playable() -> impl FnMut(Option<Res<State<MultiplayerType>>>) -> bool + Clone {
        move |current_state: Option<Res<State<MultiplayerType>>>| current_state.is_some() && current_state.unwrap().is_playable()
    }
    pub fn is_client(&self) -> bool {
        match self {
            MultiplayerType::DedicatedServer | MultiplayerType::Server | MultiplayerType::Singleplayer | MultiplayerType::Undecided => false,
            MultiplayerType::Client => true,
        }
    }
    pub fn state_is_client() -> impl FnMut(Option<Res<State<MultiplayerType>>>) -> bool + Clone {
        move |current_state: Option<Res<State<MultiplayerType>>>| current_state.is_some() && current_state.unwrap().is_client()
    }
    pub fn is_server(&self) -> bool {
        match self {
            MultiplayerType::DedicatedServer | MultiplayerType::Server => true,
            MultiplayerType::Client | MultiplayerType::Singleplayer | MultiplayerType::Undecided => false,
        }
    }
    pub fn state_is_server() -> impl FnMut(Option<Res<State<MultiplayerType>>>) -> bool + Clone {
        move |current_state: Option<Res<State<MultiplayerType>>>| current_state.is_some() && current_state.unwrap().is_server()
    }
    pub fn is_authoritative(&self) -> bool {
        match self {
            MultiplayerType::DedicatedServer | MultiplayerType::Server | MultiplayerType::Singleplayer => true,
            MultiplayerType::Client | MultiplayerType::Undecided => false,
        }
    }
    pub fn state_is_authoritative() -> impl FnMut(Option<Res<State<MultiplayerType>>>) -> bool + Clone {
        move |current_state: Option<Res<State<MultiplayerType>>>| current_state.is_some() && current_state.unwrap().is_authoritative()
    }
}

#[derive(Resource)]
pub struct PlayerInfo {
    pub username: String,
}

fn main() {
    println!("Hello, world!");

    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    #[cfg(not(target_arch = "wasm32"))]
    let headless = true;

    #[cfg(target_arch = "wasm32")]
    let headless = {
        let mut headless = false;
        if let Some(window) = window() {
            if let Some(document) = window.document() {
                if let Some(location) = document.location() {
                    if let Ok(search) = location.search() {
                        if search.len() > 1 {
                            let query = search.split_at(1).1;
                            headless = query == "headless";
                        }
                    }
                }
            }
        }
        headless
    };

    #[cfg(debug_assertions)]
    let canvas = None;
    #[cfg(not(debug_assertions))]
    let canvas = Some("#canvas".to_string());

    let mut app = App::new();
    app.add_plugins(LogPlugin {filter: "wgpu_hal=off,wgpu_core=off,some-game=info".to_string(), level: Level::INFO});
    app.add_state::<MultiplayerType>();
    if headless {
        app.add_plugins(MinimalPlugins);
        app.insert_resource(State::new(MultiplayerType::DedicatedServer));
    } else {
        app.insert_resource(PlayerInfo { username: "username here".to_owned() }); // TODO user input
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

        app.add_plugins(MainMenuPlugin {});
        app.add_state::<MainMenuState>();
    }
    app.add_plugins(ReplicationPlugins);
    app.add_plugins((
        PlayerPlugin {},
        WorldPlugin {},
        EnemyPlugin {},
        ProjectilePlugin {},
        PositionPlugin {},
    ));
    app.add_plugins(WebRtcServerPlugin {is_headless: headless});
    app.add_plugins(WebRtcClientPlugin {is_headless: headless});
    app.add_plugins(WebRtcBrowserPlugin {});

    app.add_systems(Startup, setup_world.run_if(MultiplayerType::state_is_authoritative()));
    app.add_systems(Startup, server_open_server.run_if(MultiplayerType::state_is_server()));
    app.add_systems(Startup, setup_client.run_if(MultiplayerType::state_is_playable()));
    app.add_systems(Update, player_shoot.run_if(MultiplayerType::state_is_playable()));
    app.add_systems(Startup, client_open_browser.run_if(MultiplayerType::state_is_client()));

    app.run();
}

fn setup_world(mut commands: Commands) {
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
