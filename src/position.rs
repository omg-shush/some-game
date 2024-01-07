use bevy::prelude::*;
use bevy_replicon::replicon_core::replication_rules::AppReplicationExt;
use serde::{Serialize, Deserialize};

pub struct PositionPlugin {
    pub is_server: bool
}

impl Plugin for PositionPlugin {
    fn build(&self, app: &mut App) {
        if !self.is_server {
            app.add_systems(Update, (Self::added, Self::update));
        }
        app.replicate::<Position>();
    }
}

impl PositionPlugin {
    fn added(mut commands: Commands, query: Query<(Entity, &Position), Added<Position>>) {
        for (entity, pos) in query.iter() {
            commands.entity(entity).insert(
                TransformBundle::from_transform(Transform::from_translation(pos.translation))
            );
        }
    }

    fn update(mut query: Query<(&Position, &mut Transform)>) {
        for (pos, mut trans) in query.iter_mut() {
            trans.translation = pos.translation;
        }
    }
}

#[derive(Component, Reflect, Default, Serialize, Deserialize, Clone)]
#[reflect(Component)]
pub struct Position {
    pub translation: Vec3
}

impl Position {
    pub fn from_translation(translation: Vec3) -> Position {
        Position { translation }
    }
}
