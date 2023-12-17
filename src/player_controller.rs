use bevy::{prelude::*, window::CursorGrabMode};

pub struct PlayerControllerPlugin {}

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Cursor { pos: Vec2::ZERO });
        app.add_systems(Startup, setup);
        app.add_systems(Update, (update_keys, update_mouse));
    }
}

#[derive(Component)]
pub struct PlayerController {
    pub speed: f32,
}

#[derive(Component)]
pub struct CursorSprite {}

pub struct Cursor {
    pub pos: Vec2
}

impl Resource for Cursor {}

fn update_keys(
    mut player: Query<(&mut PlayerController, &mut Transform)>,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
) {
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut window: Query<&mut Window>) {
    commands.spawn((
        CursorSprite {},
        SpriteBundle {
            sprite: Sprite::default(),
            texture: asset_server.load("cursor.png"),
            visibility: Visibility::Hidden,
            ..default()
        }
    ));
    window.single_mut().cursor.grab_mode = CursorGrabMode::None;
    window.single_mut().cursor.visible = false;
}

fn update_mouse(
    mut cursor_sprite: Query<(&mut Transform, &mut Visibility), With<CursorSprite>>,
    mut cursor_res: ResMut<Cursor>,
    mut cursor_events: EventReader<CursorMoved>,
    window: Query<&Window>
) {
    if let Some(window_pos) = cursor_events.read().last() {
        let (mut cursor_transform, mut cursor_visibility) = cursor_sprite.single_mut();
        *cursor_visibility = Visibility::Inherited;

        let window = window.single();
        let (width, height) = (window.width(), window.height());

        let pos = Vec2::new(window_pos.position.x - width / 2., -window_pos.position.y + height / 2.);
        cursor_transform.translation = Vec3::new(pos.x, pos.y, 100.);
        cursor_res.pos = pos;
    }
}
