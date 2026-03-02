use std::{f32::consts::PI, time::Duration};

use bevy::{
    color::palettes::tailwind::GRAY_500,
    ecs::schedule::ScheduleLabel,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Margin, RichText},
};
use bevy_lit::prelude::Lighting2dPlugin;
use rand::{
    Rng as _,
    seq::{IndexedRandom, SliceRandom as _},
};

use crate::{
    asset_tracking::LoadResource as _,
    game::{
        animation::{DamageAnimationMessage, MoveAnimation, spawn_damage_animations},
        assets::WorldAssets,
        debug::{DebugSettings, redo_faction_map},
        input::{InputMode, PlayerIntent},
        map::{MapPos, TILE_HEIGHT, TILE_WIDTH},
        mapgen::{Stairs, Tile},
    },
    screens::Screen,
};

mod animation;
mod assets;
mod camera;
pub(crate) mod debug;
mod examine;
mod input;
pub mod lighting;
mod map;
mod mapgen;
mod phone;

const HIGHLIGHT_Z: f32 = 20.0;
const DAMAGE_Z: f32 = 15.0;
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
    app.init_resource::<DebugSettings>();
    app.init_resource::<examine::ExaminePos>();
    app.init_resource::<examine::ExamineResults>();
    app.init_resource::<input::InputMode>();
    app.init_resource::<phone::PhoneState>();
    app.init_state::<phone::PhoneScreen>();
    app.add_message::<DamageAnimationMessage>();
    app.add_systems(
        Update,
        (
            lighting::on_add_occluder,
            lighting::on_add_player,
            input::handle_input.run_if(is_player_alive.and(phone::is_phone_closed)),
            phone::toggle_phone,
            phone::update_phone,
            animation::process_move_animations,
            animation::update_damage_animations,
            camera::update_camera,
            examine::update_examine_info,
            examine::highlight_examine_tile,
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
                (apply_damage, prune_dead).chain(),
                // environment
                update_pos_to_creature,
                process_spawners,
                update_pos_to_creature,
                // bullets
                (check_bullet_collision, move_bullets, check_bullet_collision).chain(),
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
                redo_faction_map,
                set_player_corpse.run_if(not(is_player_alive)),
            )
                .chain()
                .run_if(player_moved),
        )
            .chain(),
    );
    app.add_systems(
        EguiPrimaryContextPass,
        (sidebar, left_sidebar, phone::draw_phone).run_if(in_state(Screen::Gameplay)),
    );
}

#[derive(Component)]
pub struct GameWorld;

#[derive(Component, Reflect)]
#[require(ObscuresTile)]
pub struct Player {
    pub brainrot: i32,
    pub hunger: i32,
    pub money: i32,
    pub rizz: i32,
    pub strength: i32,
    pub boredom: i32,
    pub signal: i32,
}

#[derive(Component)]
#[require(ObscuresTile)]
struct Corpse;

#[derive(Component, Clone)]
struct DropsCorpse(assets::AsciiSprite);

#[derive(Clone, Bundle)]
struct MobBundle {
    name: Name,
    creature: Creature,
    mob: Mob,
    sprite: assets::AsciiSprite,
    corpse: DropsCorpse,
}

#[derive(Component)]
struct MobSpawner {
    spawns: Vec<MobBundle>,
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
    max_hp: i32,
    faction: i32,
}

impl Creature {
    fn is_dead(&self) -> bool {
        self.hp <= 0
    }
}

fn is_player_alive(player: Single<&Creature, With<Player>>, settings: Res<DebugSettings>) -> bool {
    settings.nohurt || player.hp > 0
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
    player: Single<(Entity, &mut MapPos, &PlayerIntent), With<Player>>,
    stairs: Query<(&MapPos, &Stairs), Without<Player>>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
    pos_to_creature: Res<PosToCreature>,
    mut damage: ResMut<PendingDamage>,
    mut moved: ResMut<PlayerMoved>,
) {
    let (player_entity, mut pos, intent) = player.into_inner();
    commands.entity(player_entity).remove::<PlayerIntent>();

    match intent {
        PlayerIntent::Move(move_intent) => {
            let old_pos = *pos;
            let new_pos = pos.0 + move_intent;
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
                commands.entity(player_entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                    ease: EaseFunction::SineInOut,
                    rotation: None,
                });
            }
        }
        PlayerIntent::Wait => {}
        PlayerIntent::UseStairs => {
            moved.0 = false;
            for (stairs_pos, Stairs { destination }) in stairs {
                if *stairs_pos == *pos {
                    let old_pos = *pos;
                    *pos = *destination;
                    commands.entity(player_entity).insert(MoveAnimation {
                        from: old_pos.to_vec3(PLAYER_Z),
                        to: pos.to_vec3(PLAYER_Z),
                        timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                        ease: EaseFunction::SineInOut,
                        rotation: None,
                    });

                    moved.0 = true;
                    break;
                }
            }
            if !moved.0 {
                return;
            }
        }
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
            let new_mob = commands.spawn((spawn.clone(), *pos, transform)).id();
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

fn set_player_corpse(
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    assets: Res<WorldAssets>,
) {
    commands
        .entity(*player)
        .insert(assets.get_ascii_sprite('%', bevy::color::palettes::css::DARK_RED.into()));
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
pub(crate) struct FactionMap {
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

        let mut directions = rogue_algebra::DIRECTIONS;
        directions.shuffle(rng);
        let target_move = directions
            .map(|o| rogue_algebra::Pos::from(pos.0) + o)
            .into_iter()
            .filter(|p| dijkstra_map.contains_key(p))
            .filter(|p| !claimed_moves.contains(&IVec2::from(*p)))
            .min_by_key(|p| dijkstra_map.get(p).cloned().unwrap_or(usize::MAX));

        if let Some(target_move) = target_move {
            // move
            mob_moves.insert(entity, target_move);
            // Claim any move that is not a destination.
            // This works because destinations are always enemies.
            if dijkstra_map.get(&target_move) != Some(&1) {
                claimed_moves.insert(IVec2::from(target_move));
            }
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
            let (_, rotation) = match (direction.x, direction.y) {
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
            let bullet_sprite = assets.get_ascii_sprite('^', Color::WHITE);
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
    mut damage_animation: MessageWriter<DamageAnimationMessage>,
    q_creatures: Query<(Entity, &Creature, &MapPos, Option<&DropsCorpse>), Without<Player>>,
) {
    let world_entity = world.into_inner();
    for (entity, creature, map_pos, corpse) in q_creatures {
        if creature.is_dead() {
            commands.entity(entity).despawn();
            if let Some(DropsCorpse(corpse_sprite)) = corpse {
                let transform = Transform::from_translation(map_pos.to_vec3(CORPSE_Z));
                let corpse_id = commands
                    .spawn((Corpse, corpse_sprite.clone(), *map_pos, transform))
                    .id();
                commands.entity(world_entity).add_child(corpse_id);
                damage_animation.write(DamageAnimationMessage { entity: corpse_id });
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
    mobs: Vec<(MapPos, Creature, Option<Mob>, String, Color)>,
}

fn update_nearby_mobs(
    mut nearby_mobs: ResMut<NearbyMobs>,
    player: Query<&MapPos, With<Player>>,
    mobs: Query<(&MapPos, &Creature, Option<&Mob>, &Text2d, &TextColor), Without<Player>>,
    pos_to_creature: Res<PosToCreature>,
) {
    nearby_mobs.mobs.clear();
    let player_pos = player.single().unwrap();
    let maxdist = 10;
    let reachable = |p| rogue_algebra::DIRECTIONS.map(|d| p + d);
    for path in rogue_algebra::path::bfs_paths(&[player_pos.0.into()], maxdist, reachable) {
        if let Some(pos) = path.last()
            && let Some(mob) = pos_to_creature.0.get(&IVec2::from(*pos))
            && let Ok((pos, creature, mob, text, color)) = mobs.get(*mob)
        {
            nearby_mobs.mobs.push((
                *pos,
                creature.clone(),
                mob.cloned(),
                text.0.clone(),
                color.0,
            ));
        }
    }
}

fn sidebar(
    mut contexts: EguiContexts,
    nearby_mobs: Res<NearbyMobs>,
    examine_results: Res<examine::ExamineResults>,
    world_assets: If<Res<WorldAssets>>,
    atlas_assets: If<Res<Assets<TextureAtlasLayout>>>,
    input_mode: Res<InputMode>,
) {
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
        .min_width(TILE_WIDTH * 8.0)
        .show(ctx, |ui| {
            let examine_pos = examine_results.info.as_ref().map(|i| i.pos);

            ui.group(|ui| {
                ui.set_min_height(400.0);

                for (pos, creature, mob, text, color) in nearby_mobs.mobs.iter() {
                    let highlight = Some(*pos) == examine_pos;
                    let mut frame = egui::Frame::new().inner_margin(Margin::same(4));
                    if highlight {
                        frame = frame.fill(ui.style().visuals.code_bg_color);
                    }
                    frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let [r, g, b, a] = color.to_srgba().to_u8_array();
                            let c32 = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
                            ui.label(
                                RichText::new(text)
                                    .size(TILE_HEIGHT)
                                    .color(c32)
                                    .background_color(if highlight {
                                        ui.style().visuals.code_bg_color
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    }),
                            );
                            for _ in 0..creature.hp / 2 {
                                ui.add(heart.clone());
                            }
                            if creature.hp % 2 == 1 {
                                ui.add(half_heart.clone());
                            }
                            if let Some(mob) = mob {
                                for _ in 0..mob.strength / 2 {
                                    ui.add(sword.clone());
                                }
                                if mob.strength % 2 == 1 {
                                    ui.add(half_sword.clone());
                                }
                            }
                        });
                    });
                }
                if let Some(ref info) = examine_results.info {
                    ui.label(&info.info);
                }
            });

            ui.add_space(20.0);

            ui.group(|ui| {
                match *input_mode {
                    InputMode::Normal => {
                        ui.label("move: arrow keys");
                        ui.label("move: hjklyubn");
                        ui.label("examine: x");
                    }
                    InputMode::Examine(_) => {
                        ui.label(RichText::new("EXAMINING"));
                        ui.label("move: arrow keys");
                        ui.label("move: hjklyubn");
                        ui.label("exit: x");
                    }
                };
            });
        });
}

fn left_sidebar(mut contexts: EguiContexts, player: Single<(&Creature, &Player)>) {
    let (creature, player_stats) = player.into_inner();
    let ctx = contexts.ctx_mut().unwrap();
    egui::SidePanel::left("left_sidebar")
        .min_width(200.0)
        .show(ctx, |ui| {
            ui.add_space(20.0);

            ui.label(RichText::new("PLAYER").size(24.0).strong());
            ui.add_space(10.0);

            let stats = [
                ("Health", creature.hp, creature.max_hp, false),
                ("Brainrot", player_stats.brainrot, 100, true),
                ("Hunger", player_stats.hunger, 100, false),
                ("Rizz", player_stats.rizz, 100, false),
                ("Strength", player_stats.strength, 100, false),
                ("Boredom", player_stats.boredom, 100, true),
            ];

            for (name, value, max, invert_colors) in stats {
                ui.label(name);
                let ratio = (value as f32 / max as f32).clamp(0.0, 1.0);
                let bar_size = egui::vec2(180.0, 20.0);

                let (rect, _response) = ui.allocate_exact_size(bar_size, egui::Sense::hover());

                // Background
                ui.painter().rect_filled(
                    rect,
                    3.0,
                    egui::Color32::from_rgba_premultiplied(50, 50, 50, 180),
                );

                // Add fill (for "filled" part of stat)
                if ratio > 0.0 {
                    let fill_rect = egui::Rect::from_min_max(
                        rect.min,
                        egui::pos2(rect.min.x + rect.width() * ratio, rect.max.y),
                    );

                    let color = if !invert_colors {
                        if ratio > 0.5 {
                            egui::Color32::from_rgb(0, 150, 0)
                        } else if ratio > 0.25 {
                            egui::Color32::from_rgb(180, 150, 0)
                        } else {
                            egui::Color32::from_rgb(150, 0, 0)
                        }
                    } else {
                        // High is bad
                        if ratio < 0.5 {
                            egui::Color32::from_rgb(0, 150, 0)
                        } else if ratio < 0.75 {
                            egui::Color32::from_rgb(180, 150, 0)
                        } else {
                            egui::Color32::from_rgb(150, 0, 0)
                        }
                    };
                    ui.painter().rect_filled(fill_rect, 3.0, color);
                }

                // White border around the bar
                ui.painter().rect_stroke(
                    rect,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                    egui::StrokeKind::Middle,
                );

                // Overlay text in white
                let text = format!("{}/{}", value, max);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                );

                ui.add_space(10.0);
            }

            ui.label("Signal");
            let signal_max = 5;
            let signal_val = player_stats.signal.clamp(0, signal_max);
            let bar_width = 10.0;
            let bar_spacing = 4.0;
            let max_bar_height = 20.0;
            let total_width = (bar_width + bar_spacing) * signal_max as f32;
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(total_width, max_bar_height),
                egui::Sense::hover(),
            );
            for i in 0..signal_max {
                let height = max_bar_height * ((i + 1) as f32 / signal_max as f32);
                let x_offset = i as f32 * (bar_width + bar_spacing);
                let bar_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.min.x + x_offset, rect.max.y - height),
                    egui::pos2(rect.min.x + x_offset + bar_width, rect.max.y),
                );
                let color = if i < signal_val {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgba_premultiplied(100, 100, 100, 100)
                };
                ui.painter().rect_filled(bar_rect, 1.0, color);
            }
            ui.add_space(10.0);

            ui.label(format!("$$$: {}", player_stats.money));
        });
}

pub fn enter(
    mut commands: Commands,
    assets: Res<assets::WorldAssets>,
    q_camera: Single<Entity, With<Camera2d>>,
) {
    let world = (
        GameWorld,
        Name::new("GameWorldRoot"),
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        InheritedVisibility::VISIBLE,
    );
    let world = commands.spawn(world).id();
    examine::init_examine_highlight(world, &mut commands, &assets);
    lighting::enable_lighting(&mut commands, *q_camera);
    mapgen::gen_map(world, commands, assets);
}

pub fn exit(
    mut commands: Commands,
    q_camera: Single<Entity, With<Camera2d>>,
    game_world: Query<Entity, With<GameWorld>>,
) {
    commands.entity(game_world.single().unwrap()).despawn();
    lighting::disable_lighting(&mut commands, *q_camera);
}
