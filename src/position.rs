use bevy::prelude::*;
use bevy_replicon::replicon_core::replication_rules::AppReplicationExt;
use serde::{Serialize, Deserialize};

use crate::Multiplayer;

pub struct PositionPlugin {}

impl Plugin for PositionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Self::added, Self::update).run_if(Multiplayer::state_is_playable()));

        app.replicate::<Position>();
    }
}

impl PositionPlugin {
    fn added(mut commands: Commands, query: Query<(Entity, &Position), Added<Position>>) {
        for (entity, pos) in query.iter() {
            commands.entity(entity).try_insert(
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
