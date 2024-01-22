use bevy::{prelude::*, sprite::Anchor, text::{Text2dBounds, TextLayoutInfo}};
use bevy_replicon::{network_event::{EventType, client_event::{ClientEventAppExt, FromClient}}, replicon_core::replication_rules::{Replication, AppReplicationExt}};
use serde::{Deserialize, Serialize};

use crate::{enemy::Enemy, player::{Player, Score}, position::Position, MultiplayerType, PlayerShootEvent};

pub struct ProjectilePlugin {}

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_mapped_client_event::<PlayerShootEvent>(EventType::Ordered);
        app.replicate::<Projectile>();

        app.add_systems(Update, (player_shoot, step, collide).run_if(MultiplayerType::state_is_authoritative()));

        app.add_systems(Update, added_projectile.run_if(MultiplayerType::state_is_playable()));
    }
}

#[derive(Serialize, Deserialize, Clone, Reflect, Default)]
pub enum ProjectileHits {
    #[default]
    Friendly,
    Enemy,
}

#[derive(Component, Serialize, Deserialize, Clone, Reflect)]
#[reflect(Component)]
pub struct Projectile {
    pub src: Entity,
    pub velocity: Vec3,
    pub hits: ProjectileHits,
    pub initial_position: Vec3,
    pub min_dist: i32,
}

impl Default for Projectile {
    fn default() -> Self {
        Self {
            src: Entity::PLACEHOLDER,
            velocity: Default::default(),
            hits: Default::default(),
            initial_position: Default::default(),
            min_dist: -1,
        }
    }
}

fn player_shoot(mut commands: Commands, mut reader: EventReader<FromClient<PlayerShootEvent>>) {
    for evt in reader.read() {
        let projectile = evt.event.projectile.clone();
        commands.spawn((projectile, Position::from_translation(evt.event.projectile.initial_position.clone() + Vec3::Z * 3.), Replication));
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
            VisibilityBundle::default(),
            Text2dBounds::default(),
            TextLayoutInfo::default(),
            Text::from_section("", TextStyle { font: asset_server.load("OpenSans-Regular.ttf"), font_size: 48., color: Color::WHITE }),
            Anchor::Center,
        ));
    }
}

fn step(mut commands: Commands, mut projectiles: Query<(Entity, &Projectile, &mut Position)>, time: Res<Time>) {
    for (entity, projectile, mut position) in projectiles.iter_mut() {
        position.translation += projectile.velocity * time.delta_seconds();
        if (position.translation - projectile.initial_position).length() > 300. {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn collide(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile, &Position)>,
    mut players: Query<(Entity, &Player, &Position, &mut Score)>,
    enemies: Query<(Entity, &Enemy, &Position)>
) {
    for (projectile_entity, mut projectile, projectile_position) in projectiles.iter_mut() {
        match projectile.hits {
            ProjectileHits::Friendly => {
                for (player_entity, player, player_position, player_score) in players.iter() {
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
                let mut min_distance = 100000;
                for (enemy_entity, enemy, enemy_position) in enemies.iter() {
                    let dist = projectile_position.translation.xy().distance(enemy_position.translation.xy());
                    if dist < 40. {
                        // Hit enemy
                        commands.entity(enemy_entity).despawn_recursive();
                        commands.entity(projectile_entity).despawn_recursive();

                        if let Ok(mut score) = players.get_component_mut::<Score>(projectile.src) {
                            score.score += 1;
                        }
                        break;
                    }
                    min_distance = min_distance.min(dist as i32);
                }
                projectile.min_dist = min_distance;
            }
        }
    }
}
