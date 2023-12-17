use bevy::{prelude::*, sprite::Anchor};

pub struct PlayerPlugin {}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
    }
}

#[derive(Component)]
pub struct Player {
    username: String
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    sprite: SpriteBundle
}

impl PlayerBundle {
    pub fn new(name: String, image: Handle<Image>, transform: Transform) -> PlayerBundle {
        PlayerBundle {
            player: Player { username: name },
            sprite: SpriteBundle { sprite: Sprite { anchor: Anchor::Center, ..default() }, texture: image, transform, ..default() } }
    }
}
