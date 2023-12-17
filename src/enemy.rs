use bevy::prelude::*;

pub struct EnemyPlugin {}

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update);
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

fn update(mut commands: Commands, mut spawners: Query<(&mut EnemySpawner, &Transform)>, time: Res<Time>) {
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
