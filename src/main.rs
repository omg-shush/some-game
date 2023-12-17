use bevy::{prelude::*, sprite::Anchor};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use enemy::EnemySpawner;
use player::PlayerBundle;
use player_controller::{Cursor, CursorSprite, PlayerController};
use projectile::{Projectile, ProjectileHits};

use crate::{
    enemy::EnemyPlugin, player::PlayerPlugin, player_controller::PlayerControllerPlugin,
    world::WorldPlugin, projectile::ProjectilePlugin,
};

mod enemy;
mod player;
mod player_controller;
mod projectile;
mod world;

fn main() {
    println!("Hello, world!");
    App::new()
        .add_plugins((DefaultPlugins, WorldInspectorPlugin::new()))
        .add_plugins((
            PlayerPlugin {},
            WorldPlugin {},
            PlayerControllerPlugin {},
            EnemyPlugin {},
            ProjectilePlugin {},
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, player_shoot)
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
            velocity: direction * 100.,
            hits: ProjectileHits::Enemy,
            initial_position: position,
        });
    }
}
