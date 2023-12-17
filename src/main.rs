use bevy::{prelude::*, sprite::Anchor};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use enemy::EnemySpawner;
use player::PlayerBundle;
use player_controller::PlayerController;

use crate::{player::PlayerPlugin, player_controller::PlayerControllerPlugin, world::WorldPlugin, enemy::EnemyPlugin};

mod enemy;
mod player;
mod player_controller;
mod world;

fn main() {
    println!("Hello, world!");
    App::new()
        .add_plugins((DefaultPlugins, WorldInspectorPlugin::new()))
        .add_plugins((PlayerPlugin {}, WorldPlugin {}, PlayerControllerPlugin {}, EnemyPlugin {}))
        .add_systems(Startup, setup)
        .run()
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        PlayerBundle::new(
            "chell".to_owned(),
            asset_server.load("chell.png"),
            Transform::from_translation(Vec3::Z),
        ),
        PlayerController { speed: 100. },
    ));
    commands.spawn((
        EnemySpawner {
            sprite: Sprite {
                anchor: Anchor::Center,
                ..default()
            },
            image: asset_server.load("enemy.png"),
            timer: Timer::from_seconds(3., TimerMode::Once),
        },
        TransformBundle {
            local: Transform::from_translation(Vec3::new(100., 0., 0.5)),
            ..default()
        },
    ));
}
