use bevy::prelude::*;

use crate::player::Player;

pub struct EnemyPlugin {}

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_spawners, update_enemies));
    }
}

#[derive(Component)]
pub struct EnemySpawner {
    pub sprite: Sprite,
    pub image: Handle<Image>,
    pub timer: Timer
}

#[derive(Component)]
struct Enemy {}

fn update_spawners(mut commands: Commands, mut spawners: Query<(&mut EnemySpawner, &Transform)>, time: Res<Time>) {
    for (mut spawner, transform) in spawners.iter_mut() {
        spawner.timer.tick(time.delta());
        if spawner.timer.just_finished() {
            commands.spawn((
                Enemy {},
                SpriteBundle {
                    sprite: spawner.sprite.clone(),
                    transform: transform.clone(),
                    texture: spawner.image.clone(),
                    ..default()
                }
            ));
        }
    }
}

fn update_enemies(mut commands: Commands, mut enemies: Query<(&mut Enemy, &mut Transform)>, players: Query<(Entity, &Transform), (With<Player>, Without<Enemy>)>, time: Res<Time>) {
    for (mut enemy, mut enemy_transform) in enemies.iter_mut() {
        // Target nearest player
        let mut nearest = None;
        for (player, player_transform) in players.iter() {
            let dist_squared = player_transform.translation.distance_squared(enemy_transform.translation);
            nearest = match nearest {
                None => Some((player, player_transform, dist_squared)),
                Some((_, _, other_dist_squared)) if dist_squared > other_dist_squared => Some((player, player_transform, dist_squared)),
                Some(_) => nearest
            };
        }
        
        // Attack player
        if let Some((player, player_transform, _)) = nearest {
            let direction = (player_transform.translation - enemy_transform.translation).normalize();
            let delta = direction * time.delta_seconds() * 50.;
            enemy_transform.translation += delta;
        }
    }
}
