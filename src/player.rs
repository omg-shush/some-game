use std::{io::Cursor, collections::HashMap};

use bevy::{prelude::*, sprite::Anchor, ptr::Ptr, text::Text2dBounds};
use bevy_replicon::{replicon_core::{replication_rules::{AppReplicationExt, self, Replication}, replicon_tick::RepliconTick}, bincode, network_event::{client_event::{ClientEventAppExt, FromClient}, EventType}};
use renet::{RenetClient, ClientId};
use serde::{Serialize, Deserialize};

use crate::{Params, player_controller::PlayerController, wasm_peers_rtc::util::console_warn, position::Position};
use crate::wasm_peers_rtc::util::js_warn;

pub struct PlayerPlugin {
    pub is_server: bool
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Player>();
        app.add_client_event::<PlayerJoinEvent>(EventType::Ordered);
        app.add_client_event::<PlayerMoveEvent>(EventType::Ordered);
        if self.is_server {
            app.add_systems(Update, (player_joined, player_moved));
            app.init_resource::<ClientPlayers>();
        } else {
            app.add_systems(Update, (join_server, added_players));
        }
    }
}

#[derive(Component, Serialize, Deserialize)]
pub struct Player {
    username: String
}

#[derive(Event, Serialize, Deserialize)]
struct PlayerJoinEvent {
    username: String
}

fn join_server(client: Res<RenetClient>, mut connected: Local<bool>, mut writer: EventWriter<PlayerJoinEvent>, params: Res<Params>) {
    if !*connected && client.is_connected() {
        *connected = true;
        writer.send(PlayerJoinEvent { username: params.username.to_owned() });
    }
}

fn player_joined(mut commands: Commands, mut reader: EventReader<FromClient<PlayerJoinEvent>>, mut mapping: ResMut<ClientPlayers>) {
    for evt in reader.read() {
        let username = evt.event.username.to_owned();
        let entity = commands.spawn((
            Player {username},
            Position::from_translation(Vec3::Z),
            Replication
        )).id();
        mapping.client_to_player.insert(evt.client_id, entity);
        mapping.player_to_client.insert(entity, evt.client_id);
    }
}

fn added_players(mut commands: Commands, query: Query<(Entity, &Player), Added<Player>>, asset_server: ResMut<AssetServer>, params: Res<Params>) {
    for (entity, player) in query.iter() {
        if let Some(mut entity) = commands.get_entity(entity) {
            entity.insert((
                Sprite {
                        anchor: Anchor::Center,
                        ..default()
                },
                asset_server.load::<Image>("chell.png"),
                VisibilityBundle::default()
            ));
            entity.with_children(|parent| {
                parent.spawn(Text2dBundle {
                    text: Text::from_section(player.username.to_owned(), TextStyle { font: asset_server.load("OpenSans-Regular.ttf"), font_size: 16., color: Color::BLACK }),
                    text_anchor: Anchor::BottomCenter,
                    text_2d_bounds: Text2dBounds::UNBOUNDED,
                    ..default()
                });
            });
            // TODO make this more robust
            if params.username == player.username {
                entity.insert(PlayerController { speed: 100. });
            }
        }
    }
}

#[derive(Default, Resource)]
struct ClientPlayers {
    client_to_player: HashMap<ClientId, Entity>,
    player_to_client: HashMap<Entity, ClientId>
}

#[derive(Event, Serialize, Deserialize)]
pub struct PlayerMoveEvent {
    pub delta: Vec3
}

fn player_moved(mut reader: EventReader<FromClient<PlayerMoveEvent>>, mapping: Res<ClientPlayers>, mut players: Query<&mut Position, With<Player>>) {
    for evt in reader.read() {
        fn player_move(mapping: &ClientPlayers, client: ClientId, players: &mut Query<&mut Position, With<Player>>, delta: Vec3) -> Option<()> {
            let e = *mapping.client_to_player.get(&client)?;
            let mut position = players.get_mut(e).ok()?;
            position.translation += delta;
            Some(())
        }
        if player_move(&mapping, evt.client_id, &mut players, evt.event.delta).is_none() {
            console_warn!("Failed to handle PlayerMoveEvent");
        };
    }
}
