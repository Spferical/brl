use std::time::Duration;

use bevy::prelude::*;
use rand::Rng;

use crate::{
    asset_tracking::LoadResource as _,
    game::{input::MoveIntent, map::MapPos},
};

mod assets;
mod camera;
mod input;
mod map;

const PLAYER_Z: f32 = 10.0;
const TILE_Z: f32 = 0.0;

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<assets::WorldAssets>();
    app.init_resource::<map::WalkBlockedMap>();
    app.add_systems(
        Update,
        (
            input::handle_input,
            map::update_walk_blocked_map,
            move_player,
            move_sprites,
            camera::update_camera,
        )
            .chain(),
    );
}

#[derive(Component)]
struct GameWorld;

#[derive(Component)]
struct Player;

fn move_player(
    player: Single<(Entity, &mut MapPos, &MoveIntent), With<Player>>,
    mut commands: Commands,
) {
    let (player_entity, mut pos, intent) = player.into_inner();
    let old_pos = *pos;
    pos.0 += intent.0;
    commands
        .entity(player_entity)
        .remove::<MoveIntent>()
        .insert(MoveAnimation {
            from: old_pos.to_vec3(PLAYER_Z),
            to: pos.to_vec3(PLAYER_Z),
            timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

            ease: EaseFunction::CubicIn,
            rotation: None,
        });
}

#[derive(Component, Debug)]
pub struct MoveAnimation {
    pub from: Vec3,
    pub to: Vec3,
    pub timer: Timer,
    pub ease: EaseFunction,
    pub rotation: Option<f32>,
}

fn move_sprites(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut MoveAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut animation) in query.iter_mut() {
        animation.timer.tick(time.delta());
        let fraction = animation.timer.fraction();
        let Vec3 { x, y, z } =
            EasingCurve::new(animation.from, animation.to, animation.ease).sample_clamped(fraction);
        transform.translation.x = x;
        transform.translation.y = y;
        transform.translation.z = z;
        if let Some(total_rotation) = animation.rotation {
            transform.rotation = Quat::from_rotation_z(total_rotation * fraction);
        }
        if animation.timer.is_finished() {
            commands.entity(entity).try_remove::<MoveAnimation>();
        }
    }
}

pub fn enter(mut commands: Commands, assets: Res<assets::WorldAssets>) {
    let game_world = (
        GameWorld,
        Name::new("GameWorldRoot"),
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        InheritedVisibility::VISIBLE,
    );

    let player_sprite = assets.get_urizen_sprite(104);
    let map_pos = MapPos(IVec2::new(0, 0));
    let player = (
        Player,
        Name::new("Player"),
        camera::CameraFollow,
        player_sprite,
        map_pos,
        Transform::from_translation(map_pos.to_vec3(PLAYER_Z)),
    );

    let mut tiles = vec![];
    for x in 0..=map::MAP_WIDTH {
        for y in 0..=map::MAP_HEIGHT {
            let rng = &mut rand::rng();
            let map_pos = MapPos(IVec2::new(x, y));
            let transform = Transform::from_translation(map_pos.to_vec3(TILE_Z));
            let mut tile = commands.spawn((map_pos, transform));
            if rng.random_bool(0.1) {
                let sprite = assets.get_urizen_sprite(rng.random_range(412..=419));
                tile.insert((sprite, map::BlocksMovement));
            } else {
                let sprite = assets.get_urizen_sprite(rng.random_range(1857..=1872));
                tile.insert(sprite);
            }
            tiles.push(tile.id());
        }
    }

    commands
        .spawn(game_world)
        .with_child(player)
        .add_children(&tiles);
}

pub fn exit() {}
