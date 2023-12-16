use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use player::PlayerBundle;
use player_controller::PlayerController;

use crate::{player::PlayerPlugin, world::WorldPlugin, player_controller::PlayerControllerPlugin};

mod player;
mod player_controller;
mod world;

fn main() {
    println!("Hello, world!");
    App::new()
        .add_plugins((DefaultPlugins, WorldInspectorPlugin::new()))
        .add_plugins((PlayerPlugin {}, WorldPlugin {}, PlayerControllerPlugin {}))
        .add_systems(Startup, setup)
        .run()
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        PlayerBundle::new("chell".to_owned(), asset_server.load("chell.png"), Transform::from_translation(Vec3::Z)),
        PlayerController { speed: 100. }));
}
