use bevy::{prelude::*, sprite::Anchor};
use bevy_replicon::replicon_core::replication_rules::{AppReplicationExt, Replication};
use serde::{Serialize, Deserialize};

use crate::{player::Player, position::Position};

pub struct EnemyPlugin {
    pub is_server: bool
}

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_enemies);
        if self.is_server {
            app.add_systems(Update, update_spawners);
        } else {
            app.add_systems(Update, added_enemies);
        }
        app.replicate::<Enemy>();
    }
}

#[derive(Component)]
pub struct EnemySpawner {
    pub image: String,
    pub timer: Timer
}

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Enemy {
    image: String
}

fn update_spawners(mut commands: Commands, mut spawners: Query<(&mut EnemySpawner, &Position)>, time: Res<Time>) {
    for (mut spawner, position) in spawners.iter_mut() {
        spawner.timer.tick(time.delta());
        if spawner.timer.just_finished() {
            commands.spawn((
                Enemy {image: spawner.image.to_owned()},
                position.clone(),
                Replication
            ));
        }
    }
}

fn added_enemies(mut commands: Commands, asset_server: Res<AssetServer>, mut new_enemies: Query<(Entity, &Enemy), Added<Enemy>>) {
    for (new_entity, new_enemy) in new_enemies.iter_mut() {
        let new_image: Handle<Image> = asset_server.load(&new_enemy.image);
        if let Some(mut entity) = commands.get_entity(new_entity) {
            entity.insert((
                new_image,
                VisibilityBundle::default(),
                Sprite {
                    anchor: Anchor::Center,
                    ..default()
                }
            ));
        }
    }
}

fn update_enemies(mut commands: Commands, mut enemies: Query<(&mut Enemy, &mut Position)>, players: Query<(Entity, &Position), (With<Player>, Without<Enemy>)>, time: Res<Time>) {
    for (enemy, mut enemy_position) in enemies.iter_mut() {
        // Target nearest player
        let mut nearest = None;
        for (player, player_position) in players.iter() {
            let dist_squared = player_position.translation.distance_squared(enemy_position.translation);
            nearest = match nearest {
                None => Some((player, player_position, dist_squared)),
                Some((_, _, other_dist_squared)) if dist_squared > other_dist_squared => Some((player, player_position, dist_squared)),
                Some(_) => nearest
            };
        }
        
        // Attack player
        if let Some((player, player_position, _)) = nearest {
            let direction = (player_position.translation - enemy_position.translation).normalize();
            let delta = direction * time.delta_seconds() * 50.;
            enemy_position.translation += delta;
        }
    }
}
