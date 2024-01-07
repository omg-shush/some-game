use std::collections::HashMap;

use bevy::{prelude::*, sprite::Anchor, text::Text2dBounds};
use bevy_replicon::{replicon_core::replication_rules::{AppReplicationExt, Replication}, network_event::{client_event::{ClientEventAppExt, FromClient}, EventType, server_event::{ServerEventAppExt, ToClients, SendMode}}};
use renet::{RenetClient, ClientId, ServerEvent};
use serde::{Serialize, Deserialize};

use crate::{Params, player_controller::PlayerController, wasm_peers_rtc::client::WebRtcClientState, position::Position};

pub struct PlayerPlugin {
    pub is_server: bool
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Player>();
        app.replicate::<Score>();
        app.add_client_event::<PlayerJoinEvent>(EventType::Ordered);
        app.add_server_event::<PlayerSpawnEvent>(EventType::Ordered);
        app.add_client_event::<PlayerMoveEvent>(EventType::Ordered);
        if self.is_server {
            app.add_systems(Update, (player_joined, player_moved, handle_events_system));
            app.init_resource::<ClientPlayers>();
        } else {
            app.add_systems(Update, (added_players, update, player_spawned, my_player));
            app.add_systems(Update, join_server.run_if(resource_exists::<RenetClient>()));
            app.init_resource::<ResClientId>();
        }
    }
}

#[derive(Component, Serialize, Deserialize)]
pub struct Player {
    client_id: u32,
    username: String
}

#[derive(Event, Serialize, Deserialize)]
struct PlayerJoinEvent {
    username: String
}

#[derive(Event, Serialize, Deserialize)]
struct PlayerSpawnEvent {
    client_id: u32
}

fn join_server(client: Res<RenetClient>, mut connected: Local<bool>, mut writer: EventWriter<PlayerJoinEvent>, params: Res<Params>) {
    if !*connected && client.is_connected() {
        *connected = true;
        info!("Sending PlayerJoinEvent!");
        writer.send(PlayerJoinEvent { username: params.username.to_owned() });
    }
}

fn player_joined(mut commands: Commands, mut reader: EventReader<FromClient<PlayerJoinEvent>>, mut writer: EventWriter<ToClients<PlayerSpawnEvent>>, mut mapping: ResMut<ClientPlayers>) {
    for evt in reader.read() {
        let username = evt.event.username.to_owned();
        let client_id = evt.client_id.raw() as u32;
        let entity = commands.spawn((
            Player {client_id, username},
            Score::default(),
            Position::from_translation(Vec3::Z),
            Replication
        )).id();
        mapping.client_to_player.insert(evt.client_id, entity);
        mapping.player_to_client.insert(entity, evt.client_id);
        writer.send(ToClients { mode: SendMode::Direct(evt.client_id), event: PlayerSpawnEvent { client_id } });
    }
}

#[derive(Resource)]
struct ResClientId {
    client_id: ClientId
}

impl Default for ResClientId {
    fn default() -> Self {
        Self { client_id: ClientId::from_raw(1000000) }
    }
}

fn player_spawned(mut reader: EventReader<PlayerSpawnEvent>, mut client_id: ResMut<ResClientId>) {
    for evt in reader.read() {
        client_id.client_id = ClientId::from_raw(evt.client_id as u64);
    }
}

fn my_player(mut commands: Commands, players: Query<(Entity, &Player), Without<PlayerController>>, client_id: Res<ResClientId>) {
    for (entity, player) in players.iter() {
        if player.client_id == client_id.client_id.raw() as u32 {
            commands.entity(entity).insert(PlayerController { speed: 100. });
        }
    }
}

fn handle_events_system(mut commands: Commands, mut mapping: ResMut<ClientPlayers>, mut server_events: EventReader<ServerEvent>) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Client {client_id} connected");
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Client {client_id} disconnected: {reason}");
                let player = mapping.client_to_player.get(client_id).map(|e| *e);
                if let Some(player) = player {
                    if let Some(entity) = commands.get_entity(player) {
                        entity.despawn_recursive();
                    }
                    mapping.player_to_client.remove(&player);
                }
                mapping.client_to_player.remove(client_id);
            }
        }
    }
}

#[derive(Component, Default, Serialize, Deserialize, Reflect)]
pub struct Score {
    pub score: usize
}

#[derive(Component)]
struct ScoreText {}

fn added_players(mut commands: Commands, query: Query<(Entity, &Player), Added<Player>>, asset_server: ResMut<AssetServer>) {
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
                    text: Text::from_section(player.username.to_owned(), TextStyle { font: asset_server.load("OpenSans-Regular.ttf"), font_size: 32., color: Color::WHITE }),
                    text_anchor: Anchor::BottomCenter,
                    text_2d_bounds: Text2dBounds::UNBOUNDED,
                    transform: Transform::from_translation(Vec3::Z),
                    ..default()
                });
                parent.spawn((ScoreText {}, Text2dBundle {
                    text: Text::from_section("0".to_owned(), TextStyle { font: asset_server.load("OpenSans-Regular.ttf"), font_size: 24., color: Color::WHITE }),
                    text_anchor: Anchor::TopCenter,
                    text_2d_bounds: Text2dBounds::UNBOUNDED,
                    transform: Transform::from_translation(Vec3::Z),
                    ..default()
                }));
            });
        }
    }
}

fn update(scores: Query<&Score>, mut texts: Query<(&Parent, &mut Text), (With<Parent>, With<ScoreText>)>) {
    for (parent, mut text) in texts.iter_mut() {
        if let Ok(score) = scores.get_component::<Score>(**parent) {
            text.sections[0].value = format!("{}", score.score);
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
            warn!("Failed to handle PlayerMoveEvent");
        };
    }
}
