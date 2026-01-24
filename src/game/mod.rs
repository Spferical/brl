use std::{f32::consts::PI, time::Duration};

use bevy::{
    color::palettes::tailwind::GRAY_500, ecs::schedule::ScheduleLabel,
    platform::collections::HashMap, prelude::*,
};
use bevy_lit::prelude::Lighting2dPlugin;
use rand::{Rng as _, seq::IndexedRandom};

use crate::{
    asset_tracking::LoadResource as _,
    game::{assets::WorldAssets, input::MoveIntent, map::MapPos},
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
                .spawn((spawn.sprite.clone(), spawn.mob.clone(), *pos, transform))
                .id();
            commands.entity(world_entity).add_child(new_mob);
        }
    }
}

fn process_mob_turn(
    assets: Res<WorldAssets>,
    mut bullets: Query<(Entity, &mut MapPos, &Bullet)>,
    mut mobs: Query<(Entity, &mut MapPos, &mut Mob), Without<Bullet>>,
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

    // Move bullets.
    for (entity, mut pos, bullet) in bullets.iter_mut() {
        // Check for collision.
        if let Some(mob_idx) = pos_to_mob_idx.get(&pos.0) {
            mobs[*mob_idx].2.hp -= bullet.damage;
        }
        if pos_to_mob_idx.contains_key(&pos.0) || walk_blocked_map.0.contains(&pos.0) {
            commands.entity(entity).despawn();
        } else {
            // Move bullet and check for collision again.
            let old_pos = *pos;
            pos.0 += bullet.direction;
            if let Some(mob_idx) = pos_to_mob_idx.get(&pos.0) {
                mobs[*mob_idx].2.hp -= bullet.damage;
            }
            if pos_to_mob_idx.contains_key(&pos.0) || walk_blocked_map.0.contains(&pos.0) {
                commands.entity(entity).despawn();
            } else {
                commands.entity(entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                    ease: EaseFunction::Linear,
                    rotation: None,
                });
            }
        }
    }

    // Determine mob intentions.
    let mut mob_moves = vec![];
    // For each enemy, target their nearest enemy
    // Build a dijkstra map _from_ each faction, towards all enemies of that faction.
    let mut positions_per_faction = HashMap::<i32, Vec<rogue_algebra::Pos>>::new();
    for (_entity, pos, mob) in mobs.iter() {
        positions_per_faction
            .entry(mob.faction)
            .or_default()
            .push(rogue_algebra::Pos::from(pos.0));
    }
    let mut dijkstra_map_per_faction =
        HashMap::<i32, std::collections::HashMap<rogue_algebra::Pos, usize>>::new();
    let reachable = |p: rogue_algebra::Pos,
                     faction: i32,
                     walk_blocked_map: &map::WalkBlockedMap|
     -> Vec<rogue_algebra::Pos> {
        rogue_algebra::DIRECTIONS
            .map(|o| p + o)
            .into_iter()
            // Avoid walls
            .filter(|rogue_algebra::Pos { x, y }| !walk_blocked_map.contains(&IVec2::new(*x, *y)))
            // Avoid friendlies
            .filter(|pos| {
                pos_to_mob_idx
                    .get(&IVec2::from(*pos))
                    .filter(|i| mobs[**i].2.faction == faction)
                    .is_none()
            })
            .collect()
    };
    for faction in positions_per_faction.keys().copied() {
        let enemy_positions = positions_per_faction
            .iter()
            .filter(|(f, _positions)| **f != faction)
            .flat_map(|(_f, positions)| positions)
            .copied()
            .collect::<Vec<_>>();
        let reachable_cb = |p| reachable(p, faction, &walk_blocked_map);
        let maxdist = 20;
        dijkstra_map_per_faction.insert(
            faction,
            rogue_algebra::path::build_dijkstra_map(&enemy_positions, maxdist, reachable_cb),
        );
    }

    for (i, (_entity, pos, mob)) in mobs.iter().enumerate() {
        // follow dijkstra map
        let dijkstra_map = dijkstra_map_per_faction.get(&mob.faction).unwrap();
        let adj = reachable(pos.0.into(), mob.faction, &walk_blocked_map);
        let target_move = adj
            .iter()
            .min_by_key(|p| dijkstra_map.get(p).cloned().unwrap_or(0))
            .cloned();
        if let Some(target_move) = target_move {
            // move
            mob_moves.push((i, target_move));
            walk_blocked_map.insert(IVec2::from(target_move));
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
            let direction = new_pos.0 - old_pos.0;
            let (sprite_idx, rotation) = match (direction.x, direction.y) {
                (1, 1) => (2093, 0.0),
                (-1, 1) => (2093, PI / 2.0),
                (-1, -1) => (2093, PI),
                (1, -1) => (2093, PI * 1.5),
                (0, 1) => (2094, 0.0),
                (-1, 0) => (2094, PI / 2.0),
                (0, -1) => (2094, PI),
                (1, 0) => (2094, PI * 1.5),
                _ => panic!("Unexpected bullet direction: {direction:?}"),
            };
            let bullet_sprite = assets.get_urizen_sprite(sprite_idx);
            let transform = Transform::from_translation(new_pos.to_vec3(PLAYER_Z))
                .with_rotation(Quat::from_rotation_z(rotation));
            let bullet = Bullet {
                direction,
                damage: 5,
            };
            commands.spawn((bullet, bullet_sprite, new_pos, transform));
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
