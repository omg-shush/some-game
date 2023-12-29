use std::env;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use bevy::{prelude::*, sprite::Anchor};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use enemy::EnemySpawner;
use player_controller::{Cursor, CursorSprite, PlayerController};
use projectile::{Projectile, ProjectileHits};
use web_sys::window;

use crate::{
    enemy::EnemyPlugin, player::PlayerPlugin, player_controller::PlayerControllerPlugin,
    world::WorldPlugin, projectile::ProjectilePlugin, wasm_peers_rtc::WasmPeersRtcPlugin,
};

mod enemy;
mod player;
mod player_controller;
mod projectile;
mod world;
mod wasm_peers_rtc;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[derive(Serialize, Deserialize)]
struct Params {
    is_server: bool,
    username: String
}

fn main() {
    println!("Hello, world!");

    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    #[cfg(not(target_arch = "wasm32"))]
    let is_server = {
        let is_server = env::args().find(|x| x == "--server").is_some();
        println!("Server: {}", is_server);
        is_server
    };

    #[cfg(target_arch = "wasm32")]
    let is_server = {
        let is_server = window().unwrap().document().unwrap().location().unwrap().search().unwrap() == "?server";
        log(&format!("Server: {}", is_server));
        is_server
    };


    let mut app = App::new();
    if is_server {
        app.add_plugins(MinimalPlugins);
    } else {
        app.add_plugins(DefaultPlugins);
        app.add_plugins(WorldInspectorPlugin::new());
        app.add_plugins(PlayerControllerPlugin {});
    }
    app.add_plugins((
        WasmPeersRtcPlugin {is_server},
        PlayerPlugin {is_server},
        WorldPlugin {is_server},
        EnemyPlugin {is_server},
        ProjectilePlugin {is_server}
    ));
    if is_server {
        app.add_systems(Startup, setup_server);
    } else {
        app.add_systems(Startup, setup_client);
        app.add_systems(Update, player_shoot);
    }

    app.run();
}

fn setup_server(mut commands: Commands) {
    commands.spawn((
        EnemySpawner {
            image: "enemy.png".to_owned(),
            timer: Timer::from_seconds(2., TimerMode::Repeating),
        },
        TransformBundle {
            local: Transform::from_translation(Vec3::new(100., 0., 0.5)),
            ..default()
        },
    ));
    commands.spawn((
        EnemySpawner {
            image: "enemy.png".to_owned(),
            timer: Timer::from_seconds(3., TimerMode::Repeating),
        },
        TransformBundle {
            local: Transform::from_translation(Vec3::new(-200., 140., 0.5)),
            ..default()
        },
    ));
    commands.spawn((
        EnemySpawner {
            image: "enemy.png".to_owned(),
            timer: Timer::from_seconds(4., TimerMode::Repeating),
        },
        TransformBundle {
            local: Transform::from_translation(Vec3::new(190., -60., 0.5)),
            ..default()
        },
    ));
    commands.spawn((
        EnemySpawner {
            image: "enemy.png".to_owned(),
            timer: Timer::from_seconds(5., TimerMode::Repeating),
        },
        TransformBundle {
            local: Transform::from_translation(Vec3::new(-100., 300., 0.5)),
            ..default()
        },
    ));
}

fn setup_client(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(PlayerController { speed: 100. });
}

fn player_shoot(
    mut commands: Commands,
    mut player: Query<(Entity, &mut PlayerController, &mut Transform), Without<CursorSprite>>,
    cursor: Res<Cursor>,
    buttons: Res<Input<MouseButton>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let position = player.single_mut().2.translation;
        let destination = Vec3::new(cursor.pos.x, cursor.pos.y, position.z);
        let direction = (destination - position).normalize_or_zero();
        commands.spawn(Projectile {
            src: player.single().0,
            velocity: direction * 150.,
            hits: ProjectileHits::Enemy,
            initial_position: position,
        });
    }
}
