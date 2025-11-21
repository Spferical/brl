use std::time::Duration;

use bevy::{
    color::palettes::tailwind::GRAY_500, ecs::schedule::ScheduleLabel,
    platform::collections::HashMap, prelude::*,
};
use bevy_lit::prelude::Lighting2dPlugin;
use rand::{Rng as _, seq::IndexedRandom};

use crate::{
    asset_tracking::LoadResource as _,
    game::{input::MoveIntent, map::MapPos},
    screens::Screen,
};

mod assets;
mod camera;
mod input;
pub mod lighting;
mod map;
mod mapgen;

const PLAYER_Z: f32 = 10.0;
const TILE_Z: f32 = 0.0;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(Lighting2dPlugin);
    app.insert_resource(ClearColor(Color::from(GRAY_500)));
    app.load_resource::<assets::WorldAssets>();
    app.init_resource::<map::WalkBlockedMap>();
    app.init_resource::<PlayerMoved>();
    app.add_systems(
        Update,
        (
            lighting::on_add_occluder,
            lighting::on_add_player,
            input::handle_input,
            move_sprites,
            camera::update_camera,
        )
            .run_if(in_state(Screen::Gameplay))
            .chain(),
    );
    app.init_schedule(Turn);
    app.add_systems(
        Turn,
        (
            map::update_walk_blocked_map,
            handle_player_move,
            (process_spawners, process_mob_turn)
                .chain()
                .run_if(player_moved),
            prune_dead,
        )
            .chain(),
    );
}

#[derive(Component)]
struct GameWorld;

#[derive(Component)]
struct Player;

#[derive(Clone)]
struct MobTemplate {
    mob: Mob,
    sprite: Sprite,
    mask: Handle<Image>,
}

#[derive(Component)]
struct MobSpawner {
    spawns: Vec<MobTemplate>,
    odds: f64,
}

#[derive(Component)]
struct Bullet {
    direction: IVec2,
    damage: i32,
}

#[derive(Component, Clone, Debug)]
struct Mob {
    hp: i32,
    faction: i32,
    strength: i32,
    ranged: bool,
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
struct Turn;

#[derive(Resource, Default)]
struct PlayerMoved(bool);

fn player_moved(moved: Res<PlayerMoved>) -> bool {
    moved.0
}

fn handle_player_move(
    mut commands: Commands,
    player: Single<(Entity, &mut MapPos, &MoveIntent), With<Player>>,
    mut walk_blocked_map: ResMut<map::WalkBlockedMap>,
    mut moved: ResMut<PlayerMoved>,
) {
    let (player_entity, mut pos, intent) = player.into_inner();
    commands.entity(player_entity).remove::<MoveIntent>();

    let old_pos = *pos;
    let new_pos = pos.0 + intent.0;
    if walk_blocked_map.contains(&new_pos) {
        moved.0 = false;
        return;
    }

    // Move the player
    pos.0 = new_pos;
    commands
        .entity(player_entity)
        .remove::<MoveIntent>()
        .insert(MoveAnimation {
            from: old_pos.to_vec3(PLAYER_Z),
            to: pos.to_vec3(PLAYER_Z),
            timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

            ease: EaseFunction::SineInOut,
            rotation: None,
        });
    walk_blocked_map.remove(&old_pos.0);
    walk_blocked_map.insert(new_pos);
    moved.0 = true;
}

fn process_spawners(
    mut commands: Commands,
    walk_blocked_map: Res<map::WalkBlockedMap>,
    world: Single<Entity, With<GameWorld>>,
    q_spawners: Query<(&MapPos, &MobSpawner)>,
) {
    let world_entity = world.into_inner();
    let rng = &mut rand::rng();
    for (pos, spawner) in q_spawners {
        if !walk_blocked_map.contains(&pos.0) && rng.random_bool(spawner.odds) {
            let spawn = spawner.spawns.choose(rng).expect("Spawner has no spawns");
            let transform = Transform::from_translation(pos.to_vec3(TILE_Z));
            let new_mob = commands
                .spawn((
                    spawn.sprite.clone(),
                    spawn.mob.clone(),
                    *pos,
                    transform,
                    // lighting::Occluder,
                ))
                .id();
            commands.entity(world_entity).add_child(new_mob);
        }
    }
}

fn process_mob_turn(
    mut mobs: Query<(Entity, &mut MapPos, &mut Mob), (Without<Player>, Without<GameWorld>)>,
    mut walk_blocked_map: ResMut<map::WalkBlockedMap>,
    mut commands: Commands,
) {
    let rng = &mut rand::rng();
    // Process enemies.
    let mut mobs = mobs.iter_mut().collect::<Vec<_>>();
    let mut pos_to_mob_idx = HashMap::new();
    for (i, (_entity, pos, _mob)) in mobs.iter().enumerate() {
        pos_to_mob_idx.insert(pos.0, i);
    }

    // Determine mob intentions.
    let mut mob_moves = vec![];
    // For each enemy, target their nearest enemy
    for (i, (_entity, pos, mob)) in mobs.iter().enumerate() {
        let starts = &[pos.0.into()];
        let maxdist = 20;
        let reachable = |p: rogue_algebra::Pos| {
            rogue_algebra::DIRECTIONS
                .map(|o| p + o)
                .into_iter()
                // Avoid walls
                .filter(|rogue_algebra::Pos { x, y }| {
                    !walk_blocked_map.contains(&IVec2::new(*x, *y))
                })
                // Avoid friendlies
                .filter(|pos| {
                    pos_to_mob_idx
                        .get(&IVec2::from(*pos))
                        .filter(|i| mobs[**i].2.faction == mob.faction)
                        .is_none()
                })
                .collect()
        };
        // let mut target_dest = None;
        let mut target_move = None;
        for path in rogue_algebra::path::bfs_paths(starts, maxdist, reachable) {
            let last = *path.last().unwrap();
            if let Some(other_mob_idx) = pos_to_mob_idx.get(&IVec2::from(last))
                && mobs[*other_mob_idx].2.faction != mob.faction
            {
                // target this mob
                // target_dest = Some(last);
                target_move = path.get(1).cloned();
                break;
            }
        }
        if let Some(target_move) = target_move {
            // move
            mob_moves.push((i, target_move));
            walk_blocked_map.insert(target_move.into());
        }
    }

    // Apply moves.
    for (i, dest) in mob_moves.into_iter() {
        let old_pos = *mobs[i].1;
        let new_pos = MapPos(IVec2::from(dest));
        if let Some(enemy_idx) = pos_to_mob_idx.get(&new_pos.0) {
            // attack
            mobs[*enemy_idx].2.hp -= mobs[i].2.strength;
        } else if mobs[i].2.ranged && rng.random_bool(0.5) {
            // fire weapon
        } else {
            // move
            *mobs[i].1 = new_pos;
            commands.entity(mobs[i].0).insert(MoveAnimation {
                from: old_pos.to_vec3(PLAYER_Z),
                to: new_pos.to_vec3(PLAYER_Z),
                timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                ease: EaseFunction::SineInOut,
                rotation: None,
            });
        }
    }
}

fn prune_dead(mut commands: Commands, q_mobs: Query<(Entity, &Mob)>) {
    for (entity, mob) in q_mobs {
        if mob.hp <= 0 {
            commands.entity(entity).despawn();
        }
    }
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

pub fn enter(
    mut commands: Commands,
    assets: Res<assets::WorldAssets>,
    q_camera: Single<Entity, With<Camera2d>>,
) {
    lighting::enable_lighting(&mut commands, *q_camera);
    mapgen::gen_map(commands, assets);
}

pub fn exit(mut commands: Commands, q_camera: Single<Entity, With<Camera2d>>) {
    lighting::disable_lighting(&mut commands, *q_camera);
}
