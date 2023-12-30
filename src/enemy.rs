use bevy::{prelude::*, sprite::Anchor};
use bevy_replicon::replicon_core::replication_rules::{AppReplicationExt, Replication};
use serde::{Serialize, Deserialize};
use rand::prelude::*;

use crate::{player::Player, position::Position};

pub struct EnemyPlugin {
    pub is_server: bool
}

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        if self.is_server {
            app.add_systems(Update, (update_enemies, update_spawners));
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

fn update_spawners(mut commands: Commands, mut spawners: Query<(&mut EnemySpawner, &Position)>, time: Res<Time>, players: Query<&Player>, enemies: Query<&Enemy>) {
    for (mut spawner, position) in spawners.iter_mut() {
        let mut position = position.clone();
        position.translation += Vec3::new(random(), random(), 0.).normalize_or_zero() * random::<f32>() * 200.;
        spawner.timer.tick(time.delta());
        if spawner.timer.just_finished() {
            if enemies.iter().len() >= players.iter().len() * 100 {
                return; // too many enemies
            }
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

fn update_enemies(mut commands: Commands, mut enemies: Query<&mut Position, With<Enemy>>, players: Query<(Entity, &Position), (With<Player>, Without<Enemy>)>, time: Res<Time>) {
    for mut enemy_position in enemies.iter_mut() {
        // Target nearest player
        let mut nearest = None;
        for (player, player_position) in players.iter() {
            let dist_squared = player_position.translation.xy().distance_squared(enemy_position.translation.xy());
            nearest = match nearest {
                None => Some((player, player_position, dist_squared)),
                Some((_, _, other_dist_squared)) if dist_squared < other_dist_squared => Some((player, player_position, dist_squared)),
                Some(_) => nearest
            };
        }
        
        // Attack player
        if let Some((player, player_position, _)) = nearest {
            let direction = (player_position.translation.xy() - enemy_position.translation.xy()).normalize();
            let delta = direction * time.delta_seconds() * 50.;
            enemy_position.translation += Vec3::new(delta.x, delta.y, 0.);
        }

        // Wander
        let delta = Vec3::new(random(), random(), 0.).normalize_or_zero();
        enemy_position.translation += delta * time.delta_seconds() * 20.;
    }
    // Enemies stay away from each other to not get clumped
    let mut pairs = enemies.iter_combinations_mut();
    while let Some([mut pos_1, mut pos_2]) = pairs.fetch_next() {
        let one_to_two = pos_2.translation - pos_1.translation;
        if one_to_two.length() < 100. {
            pos_1.translation -= one_to_two * time.delta_seconds() * 1.;
            pos_2.translation += one_to_two * time.delta_seconds() * 1.;
        }
    }
}
