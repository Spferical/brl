use std::time::Duration;

use bevy::prelude::*;
use rand::Rng;

use crate::asset_tracking::LoadResource as _;

mod camera;

const MAP_WIDTH: i32 = 50;
const MAP_HEIGHT: i32 = 16;
const TILE_WIDTH: f32 = 24.0;
const TILE_HEIGHT: f32 = 24.0;
const PLAYER_Z: f32 = 10.0;
const TILE_Z: f32 = 0.0;

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<WorldAssets>();
    app.add_systems(
        Update,
        (
            handle_input,
            move_player,
            move_sprites,
            camera::update_camera,
        )
            .chain(),
    );
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct WorldAssets {
    #[dependency]
    urizen: Handle<Image>,
    urizen_layout: Handle<TextureAtlasLayout>,
}

impl WorldAssets {
    fn get_urizen_sprite(&self, index: usize) -> Sprite {
        let mut player_sprite = Sprite::from_atlas_image(
            self.urizen.clone(),
            TextureAtlas {
                layout: self.urizen_layout.clone(),
                index,
            },
        );
        player_sprite.custom_size = Some(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        player_sprite
    }
}

impl FromWorld for WorldAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        let urizen = assets.load("urizen_onebit_tileset__v2d0.png");
        let mut tals = world.resource_mut::<Assets<TextureAtlasLayout>>();
        let urizen_layout = tals.add(TextureAtlasLayout::from_grid(
            UVec2::splat(12),
            206,
            50,
            Some(UVec2::splat(1)),
            Some(UVec2::splat(1)),
        ));
        Self {
            urizen,
            urizen_layout,
        }
    }
}

#[derive(Component)]
struct GameWorld;

#[derive(Component)]
struct Player;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapPos(pub IVec2);

impl MapPos {
    pub fn to_vec2(self) -> Vec2 {
        Vec2 {
            x: TILE_WIDTH * self.0.x as f32,
            y: TILE_HEIGHT * self.0.y as f32,
        }
    }
    pub fn to_vec3(self, z: f32) -> Vec3 {
        self.to_vec2().extend(z)
    }
    #[allow(unused)]
    pub fn from_vec3(vec3: Vec3) -> Self {
        Self::from_vec2(vec3.xy())
    }
    #[allow(unused)]
    pub fn from_vec2(vec2: Vec2) -> Self {
        Self(IVec2 {
            x: (vec2.x / TILE_WIDTH) as i32,
            y: (vec2.y / TILE_HEIGHT) as i32,
        })
    }
    #[allow(unused)]
    pub fn corners(&self) -> [Vec2; 4] {
        [
            self.to_vec2(),
            self.to_vec2() + Vec2::new(0.0, 1.0),
            self.to_vec2() + Vec2::new(1.0, 0.0),
            self.to_vec2() + Vec2::new(1.0, 1.0),
        ]
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveIntent(pub IVec2);

pub fn enter(mut commands: Commands, assets: Res<WorldAssets>) {
    let game_world = (
        GameWorld,
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        InheritedVisibility::VISIBLE,
    );

    let player_sprite = assets.get_urizen_sprite(104);
    let map_pos = MapPos(IVec2::new(0, 0));
    let player = (
        Player,
        camera::CameraFollow,
        player_sprite,
        map_pos,
        Transform::from_translation(map_pos.to_vec3(PLAYER_Z)),
    );

    let mut tiles = vec![];
    for x in 0..=MAP_WIDTH {
        for y in 0..=MAP_HEIGHT {
            let rng = &mut rand::rng();
            let sprite = assets.get_urizen_sprite(rng.random_range(1857..=1872));
            let map_pos = MapPos(IVec2::new(x, y));
            let tile = (
                sprite,
                map_pos,
                Transform::from_translation(map_pos.to_vec3(TILE_Z)),
            );
            tiles.push(tile);
        }
    }

    commands
        .spawn(game_world)
        .with_child(player)
        .with_children(|commands| {
            for t in tiles {
                commands.spawn(t);
            }
        });
}

fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    q_player: Query<Entity, With<Player>>,
) {
    let Ok(player_entity) = q_player.single() else {
        return;
    };
    let mut intent = IVec2::ZERO;
    for (key, dir) in [
        (KeyCode::KeyW, IVec2::new(0, 1)),
        (KeyCode::KeyA, IVec2::new(-1, 0)),
        (KeyCode::KeyS, IVec2::new(0, -1)),
        (KeyCode::KeyD, IVec2::new(1, 0)),
        (KeyCode::KeyK, IVec2::new(0, 1)),
        (KeyCode::KeyJ, IVec2::new(-1, 0)),
        (KeyCode::KeyH, IVec2::new(0, -1)),
        (KeyCode::KeyL, IVec2::new(1, 0)),
    ] {
        if keyboard_input.just_pressed(key) {
            intent += dir;
        }
    }
    if intent != IVec2::ZERO {
        commands.entity(player_entity).insert(MoveIntent(intent));
    }
}

fn move_player(
    mut q_player: Query<(Entity, &mut MapPos, &MoveIntent), With<Player>>,
    mut commands: Commands,
) {
    let Ok((player_entity, mut pos, intent)) = q_player.single_mut() else {
        return;
    };
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

pub fn exit() {}
