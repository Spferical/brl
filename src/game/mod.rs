use std::{f32::consts::PI, time::Duration};

use bevy::{
    color::palettes::tailwind::GRAY_500,
    ecs::schedule::ScheduleLabel,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use bevy_lit::prelude::Lighting2dPlugin;
use rand::{Rng as _, seq::IndexedRandom};

use crate::{
    asset_tracking::LoadResource as _,
    game::{
        animation::{DamageAnimationMessage, MoveAnimation, spawn_damage_animations},
        assets::WorldAssets,
        input::MoveIntent,
        map::{MapPos, TILE_HEIGHT, TILE_WIDTH},
        mapgen::Tile,
    },
    screens::Screen,
};

mod animation;
mod assets;
mod camera;
mod input;
pub mod lighting;
mod map;
mod mapgen;

const PLAYER_Z: f32 = 10.0;
const CORPSE_Z: f32 = 5.0;
const TILE_Z: f32 = 0.0;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(Lighting2dPlugin);
    app.insert_resource(ClearColor(Color::from(GRAY_500)));
    app.load_resource::<assets::WorldAssets>();
    app.init_resource::<map::WalkBlockedMap>();
    app.init_resource::<PendingDamage>();
    app.init_resource::<PlayerMoved>();
    app.init_resource::<FactionMap>();
    app.init_resource::<PosToCreature>();
    app.init_resource::<NearbyMobs>();
    app.add_message::<DamageAnimationMessage>();
    app.add_systems(
        Update,
        (
            lighting::on_add_occluder,
            lighting::on_add_player,
            input::handle_input,
            animation::process_move_animations,
            animation::update_damage_animations,
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
            (
                // kill mobs from any player damage
                apply_damage,
                spawn_damage_animations,
                prune_dead,
                // environment
                update_pos_to_creature,
                process_spawners,
                update_pos_to_creature,
                // bullets
                check_bullet_collision,
                move_bullets,
                check_bullet_collision,
                // mobs get a turn
                build_faction_map,
                process_mob_turn,
                update_pos_to_creature,
                check_bullet_collision,
                // damage
                apply_damage,
                spawn_damage_animations,
                prune_dead,
                update_pos_to_creature,
                // end-of-turn bookkeeping
                obscure_tiles,
                update_nearby_mobs,
            )
                .chain()
                .run_if(player_moved),
        )
            .chain(),
    );
    app.add_systems(
        EguiPrimaryContextPass,
        sidebar.run_if(in_state(Screen::Gameplay)),
    );
}

#[derive(Component)]
pub struct GameWorld;

#[derive(Component)]
#[require(ObscuresTile)]
struct Player;

#[derive(Component)]
#[require(ObscuresTile)]
struct Corpse;

#[derive(Component)]
struct DropsCorpse(Sprite);

#[derive(Clone)]
struct MobTemplate {
    hp: i32,
    faction: i32,
    strength: i32,
    ranged: bool,
    sprite: Sprite,
    corpse: Sprite,
}

#[derive(Component)]
struct MobSpawner {
    spawns: Vec<MobTemplate>,
    odds: f64,
}

#[derive(Component)]
#[require(ObscuresTile)]
struct Bullet {
    direction: IVec2,
    damage: i32,
}

/// Common fields between the player and mobs.
#[derive(Component, Clone, Debug)]
struct Creature {
    hp: i32,
    #[allow(unused)]
    max_hp: i32,
    faction: i32,
}

impl Creature {
    fn is_dead(&self) -> bool {
        self.hp <= 0
    }
}

// NPC-specific fields.
#[derive(Component, Clone, Debug)]
#[require(ObscuresTile)]
struct Mob {
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
    walk_blocked_map: Res<map::WalkBlockedMap>,
    pos_to_creature: Res<PosToCreature>,
    mut damage: ResMut<PendingDamage>,
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

    if let Some(entity) = pos_to_creature.0.get(&new_pos) {
        damage.0.push(DamageInstance {
            entity: *entity,
            hp: 2,
        });
    } else {
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
    }
    moved.0 = true;
}

fn process_spawners(
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    pos_to_mob: Res<PosToCreature>,
    q_spawners: Query<(&MapPos, &MobSpawner)>,
) {
    let world_entity = world.into_inner();
    let rng = &mut rand::rng();
    for (pos, spawner) in q_spawners {
        if !pos_to_mob.0.contains_key(&pos.0) && rng.random_bool(spawner.odds) {
            let spawn = spawner.spawns.choose(rng).expect("Spawner has no spawns");
            let transform = Transform::from_translation(pos.to_vec3(TILE_Z));
            let drops_corpse = DropsCorpse(spawn.corpse.clone());
            let new_mob = commands
                .spawn((
                    spawn.sprite.clone(),
                    Creature {
                        hp: spawn.hp,
                        max_hp: spawn.hp,
                        faction: spawn.faction,
                    },
                    Mob {
                        strength: spawn.strength,
                        ranged: spawn.ranged,
                    },
                    *pos,
                    transform,
                    drops_corpse,
                ))
                .id();
            commands.entity(world_entity).add_child(new_mob);
        }
    }
}

#[derive(Resource, Default)]
struct PosToCreature(HashMap<IVec2, Entity>);

fn update_pos_to_creature(
    mut pos_to_creature: ResMut<PosToCreature>,
    creatures: Query<(Entity, &MapPos), With<Creature>>,
) {
    pos_to_creature.0.clear();
    for (entity, pos) in creatures {
        if pos_to_creature.0.insert(pos.0, entity).is_some() {
            warn!("Overlapping mobs at {}", pos.0);
        }
    }
}

pub struct DamageInstance {
    entity: Entity,
    hp: i32,
}

#[derive(Resource, Default)]
pub struct PendingDamage(Vec<DamageInstance>);

fn check_bullet_collision(
    mut commands: Commands,
    pos_to_mob: Res<PosToCreature>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
    bullets: Query<(Entity, &MapPos, &Bullet)>,
    mut damage: ResMut<PendingDamage>,
) {
    for (entity, pos, bullet) in bullets.iter() {
        if let Some(mob) = pos_to_mob.0.get(&pos.0) {
            damage.0.push(DamageInstance {
                entity: *mob,
                hp: bullet.damage,
            });
        }
        if pos_to_mob.0.contains_key(&pos.0) || walk_blocked_map.0.contains(&pos.0) {
            commands.entity(entity).despawn();
        }
    }
}

fn apply_damage(
    mut damage: ResMut<PendingDamage>,
    mut animation: MessageWriter<DamageAnimationMessage>,
    mut creature: Query<&mut Creature>,
) {
    for DamageInstance { entity, hp } in damage.0.drain(..) {
        if let Ok(mut creature) = creature.get_mut(entity) {
            creature.hp -= hp;
        }
        animation.write(DamageAnimationMessage { entity });
    }
}

fn move_bullets(mut commands: Commands, mut bullets: Query<(Entity, &mut MapPos, &Bullet)>) {
    for (entity, mut pos, bullet) in bullets.iter_mut() {
        let old_pos = *pos;
        pos.0 += bullet.direction;
        commands.entity(entity).insert(MoveAnimation {
            from: old_pos.to_vec3(PLAYER_Z),
            to: pos.to_vec3(PLAYER_Z),
            timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

            ease: EaseFunction::Linear,
            rotation: None,
        });
    }
}

#[derive(Resource, Default)]
struct FactionMap {
    dijkstra_map_per_faction: HashMap<i32, std::collections::HashMap<rogue_algebra::Pos, usize>>,
}

fn reachable(
    p: rogue_algebra::Pos,
    walk_blocked_map: &map::WalkBlockedMap,
    other_unwalkable: &HashSet<IVec2>,
) -> Vec<rogue_algebra::Pos> {
    rogue_algebra::DIRECTIONS
        .map(|o| p + o)
        .into_iter()
        .filter(|rogue_algebra::Pos { x, y }| !walk_blocked_map.contains(&IVec2::new(*x, *y)))
        .filter(|rogue_algebra::Pos { x, y }| !other_unwalkable.contains(&IVec2::new(*x, *y)))
        .collect()
}

fn build_faction_map(
    mut faction_map: ResMut<FactionMap>,
    creatures: Query<(Entity, &mut MapPos, &mut Creature)>,
    walk_blocked_map: ResMut<map::WalkBlockedMap>,
) {
    // For each enemy, target their nearest enemy
    // Build a dijkstra map _from_ each faction, towards all enemies of that faction.
    let mut positions_per_faction = HashMap::<i32, HashSet<IVec2>>::new();
    for (_entity, pos, creature) in creatures.iter() {
        positions_per_faction
            .entry(creature.faction)
            .or_default()
            .insert(pos.0);
    }
    let mut dijkstra_map_per_faction =
        HashMap::<i32, std::collections::HashMap<rogue_algebra::Pos, usize>>::new();
    for (faction, friendly_positions) in positions_per_faction.iter() {
        let enemy_positions = positions_per_faction
            .iter()
            .filter(|(f, _positions)| **f != *faction)
            .flat_map(|(_f, positions)| positions)
            .copied()
            .map(rogue_algebra::Pos::from)
            .collect::<Vec<_>>();
        let reachable_cb = |p| reachable(p, &walk_blocked_map, friendly_positions);
        let maxdist = 20;
        dijkstra_map_per_faction.insert(
            *faction,
            rogue_algebra::path::build_dijkstra_map(&enemy_positions, maxdist, reachable_cb),
        );
    }
    faction_map.dijkstra_map_per_faction = dijkstra_map_per_faction;
}

fn process_mob_turn(
    world: Single<Entity, With<GameWorld>>,
    assets: Res<WorldAssets>,
    pos_to_creature: Res<PosToCreature>,
    faction_map: Res<FactionMap>,
    mut mobs: Query<(Entity, &mut MapPos, &Creature, &mut Mob)>,
    mut commands: Commands,
    mut damage: ResMut<PendingDamage>,
) {
    let world_entity = world.into_inner();
    let rng = &mut rand::rng();
    // Determine mob intentions.
    let mut mob_moves = HashMap::new();
    let mut claimed_moves = HashSet::new();
    for (entity, pos, creature, _mob) in mobs.iter() {
        if creature.is_dead() {
            continue;
        }
        // follow dijkstra map
        let dijkstra_map = faction_map
            .dijkstra_map_per_faction
            .get(&creature.faction)
            .unwrap();
        let target_move = rogue_algebra::DIRECTIONS
            .map(|o| rogue_algebra::Pos::from(pos.0) + o)
            .into_iter()
            .filter(|p| dijkstra_map.contains_key(p))
            .filter(|p| !claimed_moves.contains(&IVec2::from(*p)))
            .min_by_key(|p| dijkstra_map.get(p).cloned().unwrap_or(usize::MAX));

        if let Some(target_move) = target_move {
            // move
            mob_moves.insert(entity, target_move);
            claimed_moves.insert(IVec2::from(target_move));
        }
    }

    // Apply moves.
    for (entity, dest) in mob_moves.into_iter() {
        let (entity, mut pos, _creature, mob) = mobs.get_mut(entity).unwrap();
        let old_pos = *pos;
        let new_pos = MapPos(IVec2::from(dest));
        if let Some(enemy) = pos_to_creature.0.get(&new_pos.0) {
            // attack
            damage.0.push(DamageInstance {
                entity: *enemy,
                hp: mob.strength,
            });
        } else if mob.ranged && rng.random_bool(0.5) {
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
                damage: 1,
            };
            let bullet_id = commands
                .spawn((bullet, bullet_sprite, new_pos, transform))
                .id();
            commands.entity(world_entity).add_child(bullet_id);
        } else {
            // move
            *pos = new_pos;
            commands.entity(entity).insert(MoveAnimation {
                from: old_pos.to_vec3(PLAYER_Z),
                to: new_pos.to_vec3(PLAYER_Z),
                timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                ease: EaseFunction::SineInOut,
                rotation: None,
            });
        }
    }
}

fn prune_dead(
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    q_creatures: Query<(Entity, &Creature, &MapPos, Option<&DropsCorpse>), Without<Player>>,
) {
    let world_entity = world.into_inner();
    for (entity, creature, map_pos, corpse) in q_creatures {
        if creature.is_dead() {
            commands.entity(entity).despawn();
            if let Some(DropsCorpse(corpse_sprite)) = corpse {
                let transform = Transform::from_translation(map_pos.to_vec3(CORPSE_Z));
                let mut corpse_sprite = (*corpse_sprite).clone();
                corpse_sprite.color = corpse_sprite.color.darker(0.75);
                let corpse_id = commands
                    .spawn((Corpse, corpse_sprite, *map_pos, transform))
                    .id();
                commands.entity(world_entity).add_child(corpse_id);
            }
        }
    }
}

#[derive(Component, Default)]
struct ObscuresTile;

fn obscure_tiles(
    obscures: Query<&MapPos, With<ObscuresTile>>,
    tiles: Query<(&MapPos, &mut Visibility), With<Tile>>,
) {
    let obscured_positions = obscures.iter().map(|p| p.0).collect::<HashSet<IVec2>>();
    for (pos, mut visibility) in tiles {
        *visibility = if obscured_positions.contains(&pos.0) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

#[derive(Default, Resource)]
struct NearbyMobs {
    mobs: Vec<(Creature, Mob, Sprite)>,
}

fn update_nearby_mobs(
    mut nearby_mobs: ResMut<NearbyMobs>,
    player: Query<&MapPos, With<Player>>,
    mobs: Query<(&Creature, &Mob, &Sprite)>,
    pos_to_creature: Res<PosToCreature>,
) {
    nearby_mobs.mobs.clear();
    let player_pos = player.single().unwrap();
    let maxdist = 10;
    let reachable = |p| rogue_algebra::DIRECTIONS.map(|d| p + d);
    for path in rogue_algebra::path::bfs_paths(&[player_pos.0.into()], maxdist, reachable) {
        if let Some(pos) = path.last()
            && let Some(mob) = pos_to_creature.0.get(&IVec2::from(*pos))
            && let Ok((creature, mob, sprite)) = mobs.get(*mob)
        {
            nearby_mobs
                .mobs
                .push((creature.clone(), mob.clone(), sprite.clone()));
        }
    }
}

fn sidebar(
    mut contexts: EguiContexts,
    nearby_mobs: Res<NearbyMobs>,
    world_assets: If<Res<WorldAssets>>,
    atlas_assets: If<Res<Assets<TextureAtlasLayout>>>,
) {
    let mut mob_images = vec![];
    for (_creature, _mob, sprite) in &nearby_mobs.mobs {
        mob_images.push(
            assets::get_egui_image_from_sprite(&mut contexts, &atlas_assets, sprite)
                .fit_to_exact_size(egui::vec2(TILE_WIDTH, TILE_HEIGHT)),
        )
    }

    let heart = world_assets
        .get_urizen_egui_image(&mut contexts, &atlas_assets, 7700)
        .fit_to_exact_size(egui::vec2(TILE_WIDTH, TILE_HEIGHT));
    let half_heart = world_assets
        .get_urizen_egui_image(&mut contexts, &atlas_assets, 7703)
        .fit_to_exact_size(egui::vec2(TILE_WIDTH, TILE_HEIGHT));
    let sword = world_assets
        .get_urizen_egui_image(&mut contexts, &atlas_assets, 1262)
        .fit_to_exact_size(egui::vec2(TILE_WIDTH, TILE_HEIGHT));
    let half_sword = world_assets
        .get_urizen_egui_image(&mut contexts, &atlas_assets, 1280)
        .fit_to_exact_size(egui::vec2(TILE_WIDTH, TILE_HEIGHT));

    let ctx = contexts.ctx_mut().unwrap();
    egui::SidePanel::right("sidebar")
        .min_width(TILE_WIDTH * 6.0)
        .show(ctx, |ui| {
            for (i, (creature, mob, _sprite)) in nearby_mobs.mobs.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(mob_images[i].clone());
                    for _ in 0..creature.hp / 2 {
                        ui.add(heart.clone());
                    }
                    if creature.hp % 2 == 1 {
                        ui.add(half_heart.clone());
                    }
                    for _ in 0..mob.strength / 2 {
                        ui.add(sword.clone());
                    }
                    if mob.strength % 2 == 1 {
                        ui.add(half_sword.clone());
                    }
                });
            }
        });
}

pub fn enter(
    mut commands: Commands,
    assets: Res<assets::WorldAssets>,
    q_camera: Single<Entity, With<Camera2d>>,
) {
    lighting::enable_lighting(&mut commands, *q_camera);
    mapgen::gen_map(commands, assets);
}

pub fn exit(
    mut commands: Commands,
    q_camera: Single<Entity, With<Camera2d>>,
    game_world: Query<Entity, With<GameWorld>>,
) {
    commands.entity(game_world.single().unwrap()).despawn();
    lighting::disable_lighting(&mut commands, *q_camera);
}
