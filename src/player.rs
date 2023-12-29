use std::{io::Cursor, time::Duration};

use bevy::{prelude::*, sprite::Anchor, ptr::Ptr};
use bevy_replicon::{replicon_core::{replication_rules::{AppReplicationExt, self, Replication}, replicon_tick::RepliconTick}, bincode, network_event::client_event::{ClientEventAppExt, FromClient}};
use bevy_replicon::client::client_mapper::ServerEntityMap;
use renet::{SendType, RenetClient};
use serde::{Serialize, Deserialize};

use crate::player_controller::PlayerController;

pub struct PlayerPlugin {
    pub is_server: bool
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Player>();
        app.replicate_with::<Transform>(serialize_transform, deserialize_transform, replication_rules::remove_component::<Transform>);
        app.add_client_event::<PlayerJoinEvent>(SendType::ReliableOrdered { resend_time: Duration::from_secs(1) });
        if self.is_server {
            app.add_systems(Update, player_joined);
        } else {
            app.add_systems(Update, (join_server, added_players));
        }
    }
}

/// Serializes only translation.
fn serialize_transform(
    component: Ptr,
    cursor: &mut Cursor<Vec<u8>>,
) -> bincode::Result<()> {
    // SAFETY: Function called for registered `ComponentId`.
    let transform: &Transform = unsafe { component.deref() };
    bincode::serialize_into(cursor, &transform.translation)
}

/// Deserializes translation and creates [`Transform`] from it.
fn deserialize_transform(
    entity: &mut EntityWorldMut,
    _entity_map: &mut ServerEntityMap,
    cursor: &mut Cursor<&[u8]>,
    _replicon_tick: RepliconTick,
) -> bincode::Result<()> {
    let translation: Vec3 = bincode::deserialize_from(cursor)?;
    entity.insert(Transform::from_translation(translation));

    Ok(())
}

#[derive(Component, Serialize, Deserialize)]
pub struct Player {
    username: String
}

#[derive(Event, Serialize, Deserialize)]
struct PlayerJoinEvent {
    username: String
}

fn join_server(client: Res<RenetClient>, mut connected: Local<bool>, mut writer: EventWriter<PlayerJoinEvent>) {
    if !*connected && client.is_connected() {
        *connected = true;
        writer.send(PlayerJoinEvent { username: "username here".to_owned() });
    }
}

fn player_joined(mut commands: Commands, mut reader: EventReader<FromClient<PlayerJoinEvent>>) {
    for evt in reader.read() {
        let username = evt.event.username;
        commands.spawn(Player {username});
    }
}

fn added_players(mut commands: Commands, query: Query<(Entity, &Player), Added<Player>>, asset_server: ResMut<AssetServer>) {
    for (entity, player) in query.iter() {
        if let Some(mut entity) = commands.get_entity(entity) {
            entity.insert((
                SpriteBundle {
                    sprite: Sprite {
                        anchor: Anchor::Center,
                        ..default()
                    },
                    texture: asset_server.load("chell.png"),
                    transform: Transform::from_translation(Vec3::Z), ..default()
                },
            ));
        }
    }
}
