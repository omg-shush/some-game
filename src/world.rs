use bevy::{prelude::*, utils::HashMap, sprite::{MaterialMesh2dBundle, Mesh2d}};

use crate::Multiplayer;

const CHUNK_SIZE: usize = 64;

pub struct WorldPlugin {}

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_server.run_if(Multiplayer::state_is_server()));

        app.add_systems(Startup, setup_client.run_if(Multiplayer::state_is_client()));
    }
}

#[derive(Component)]
struct World {
    chunks: HashMap<ChunkCoord, Chunk>
}

#[derive(PartialEq, Eq, Hash)]
struct ChunkCoord {
    x: i32,
    y: i32
}

impl ChunkCoord {
    pub fn from_global(x: i32, y: i32) -> ChunkCoord {
        ChunkCoord { x: x / CHUNK_SIZE as i32, y: y / CHUNK_SIZE as i32 }
    }
}

struct Chunk {
    image: Handle<Image>
}

fn setup_server(mut commands: Commands) {
    // TODO
}

fn setup_client(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = asset_server.load("chunk_0_0.png");
    let mut chunks = HashMap::new();
    chunks.insert(ChunkCoord {x: 0, y: 0}, Chunk { image: image.clone() });
    let world = World { chunks };

    let sprite = SpriteBundle {
        sprite: Sprite { ..default() },
        texture: image,
        ..default()
    };

    // commands.spawn((world, sprite));
}
