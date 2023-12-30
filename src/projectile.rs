use bevy::{prelude::*, sprite::Anchor};
use bevy_replicon::{network_event::{EventType, client_event::{ClientEventAppExt, FromClient}}, replicon_core::replication_rules::{Replication, AppReplicationExt}};
use serde::{Deserialize, Serialize};

use crate::{enemy::Enemy, player::Player, PlayerShootEvent, position::Position};

pub struct ProjectilePlugin {
    pub is_server: bool
}

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_client_event::<PlayerShootEvent>(EventType::Ordered);
        app.replicate::<Projectile>();
        if self.is_server {
            app.add_systems(Update, (player_shoot, step, collide));
        } else {
            app.add_systems(Update, added_projectile);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Reflect, Default)]
pub enum ProjectileHits {
    #[default]
    Friendly,
    Enemy,
}

#[derive(Component, Serialize, Deserialize, Clone, Reflect, Default)]
#[reflect(Component)]
pub struct Projectile {
    // pub src: Entity,
    pub velocity: Vec3,
    pub hits: ProjectileHits,
    pub initial_position: Vec3,
}

fn player_shoot(mut commands: Commands, mut reader: EventReader<FromClient<PlayerShootEvent>>) {
    for evt in reader.read() {
        let projectile = evt.event.projectile.clone();
        commands.spawn((projectile, Position::from_translation(evt.event.projectile.initial_position.clone()), Replication));
    }
}

fn added_projectile(
    mut commands: Commands,
    added: Query<(Entity, &Projectile), Added<Projectile>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, projectile) in added.iter() {
        commands.get_entity(entity).unwrap().insert((
            asset_server.load::<Image>("spark.png"),
            Sprite {
                anchor: Anchor::Center,
                ..default()
            },
            VisibilityBundle::default()
        ));
    }
}

fn step(mut commands: Commands, mut projectiles: Query<(Entity, &Projectile, &mut Position)>, time: Res<Time>) {
    for (entity, projectile, mut transform) in projectiles.iter_mut() {
        transform.translation += projectile.velocity * time.delta_seconds();
        if (transform.translation - projectile.initial_position).length() > 300. {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn collide(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile, &Position)>,
    players: Query<(Entity, &Player, &Position)>,
    enemies: Query<(Entity, &Enemy, &Position)>,
) {
    for (projectile_entity, projectile, projectile_position) in projectiles.iter() {
        match projectile.hits {
            ProjectileHits::Friendly => {
                for (player_entity, player, player_position) in players.iter() {
                    if projectile_position
                        .translation
                        .distance(player_position.translation)
                        < 10.
                    {
                        // Hit player
                        commands.entity(player_entity).despawn_recursive();
                        commands.entity(projectile_entity).despawn_recursive();
                        break;
                    }
                }
            }
            ProjectileHits::Enemy => {
                for (enemy_entity, enemy, enemy_transform) in enemies.iter() {
                    if projectile_position.translation.distance(enemy_transform.translation) < 30. {
                        // Hit enemy
                        commands.entity(enemy_entity).despawn_recursive();
                        commands.entity(projectile_entity).despawn_recursive();
                        break;
                    }
                }
            }
        }
    }
}
