use bevy::prelude::*;

pub struct PlayerControllerPlugin {}

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, update);
    }
}

#[derive(Component)]
pub struct PlayerController {
    pub speed: f32
}

fn update(mut player: Query<(&mut PlayerController, &mut Transform)>, time: Res<Time>, keys: Res<Input<KeyCode>>) {
    let (controller, mut transform) = player.single_mut();
    let mut delta = Vec3::ZERO;
    if keys.pressed(KeyCode::W) {
        delta += Vec3::Y;
    }
    if keys.pressed(KeyCode::S) {
        delta -= Vec3::Y;
    }
    if keys.pressed(KeyCode::D) {
        delta += Vec3::X;
    }
    if keys.pressed(KeyCode::A) {
        delta -= Vec3::X;
    }
    delta = delta.normalize_or_zero() * time.delta_seconds() * controller.speed;
    transform.translation += delta;
}
