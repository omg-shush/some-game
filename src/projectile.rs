use bevy::{prelude::*, sprite::Anchor};

use crate::{enemy::Enemy, player::Player};

pub struct ProjectilePlugin {}

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (added_projectile, step, collide));
    }
}

pub enum ProjectileHits {
    Friendly,
    Enemy,
}

#[derive(Component)]
pub struct Projectile {
    pub src: Entity,
    pub velocity: Vec3,
    pub hits: ProjectileHits,
    pub initial_position: Vec3,
}

fn added_projectile(
    mut commands: Commands,
    added: Query<(Entity, &Projectile), Added<Projectile>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, projectile) in added.iter() {
        commands.get_entity(entity).unwrap().insert(SpriteBundle {
            transform: Transform::from_translation(projectile.initial_position),
            texture: asset_server.load("spark.png"),
            sprite: Sprite {
                anchor: Anchor::Center,
                ..default()
            },
            ..default()
        });
    }
}

fn step(mut commands: Commands, mut projectiles: Query<(Entity, &Projectile, &mut Transform)>, time: Res<Time>) {
    for (entity, projectile, mut transform) in projectiles.iter_mut() {
        transform.translation += projectile.velocity * time.delta_seconds();
        if (transform.translation - projectile.initial_position).length() > 300. {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn collide(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile, &Transform)>,
    players: Query<(Entity, &Player, &Transform)>,
    enemies: Query<(Entity, &Enemy, &Transform)>,
) {
    for (projectile_entity, projectile, projectile_transform) in projectiles.iter() {
        match projectile.hits {
            ProjectileHits::Friendly => {
                for (player_entity, player, player_transform) in players.iter() {
                    if projectile_transform
                        .translation
                        .distance(player_transform.translation)
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
                    if projectile_transform.translation.distance(enemy_transform.translation) < 30. {
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
