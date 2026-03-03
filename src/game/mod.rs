use std::{f32::consts::PI, time::Duration};

use bevy::{
    color::palettes::tailwind::GRAY_500,
    ecs::schedule::ScheduleLabel,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Align, FontSelection, Margin, RichText, WidgetText, text::LayoutJob},
};
use bevy_lit::prelude::Lighting2dPlugin;
use indexmap::IndexSet;
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
        input::{AbilityClicked, InputMode, PlayerIntent},
        map::{MapPos, TILE_HEIGHT, TILE_WIDTH},
        mapgen::{Stairs, Tile},
    },
    screens::Screen,
};

mod animation;
mod assets;
mod camera;
mod chat;
pub(crate) mod debug;
mod examine;
mod input;
pub mod lighting;
mod map;
mod mapgen;
mod phone;
mod targeting;

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
    app.init_resource::<PlayerAbilities>();
    app.init_resource::<PendingDamage>();
    app.init_resource::<PlayerMoved>();
    app.init_resource::<FactionMap>();
    app.init_resource::<PosToCreature>();
    app.init_resource::<NearbyMobs>();
    app.init_resource::<DebugSettings>();
    app.init_resource::<examine::ExaminePos>();
    app.init_resource::<examine::ExamineResults>();
    app.init_resource::<input::InputMode>();
    app.init_resource::<targeting::ValidTargets>();
    app.init_resource::<phone::PhoneState>();
    app.init_resource::<chat::ChatHistory>();
    app.init_resource::<TurnCounter>();
    app.init_state::<phone::PhoneScreen>();
    app.add_message::<DamageAnimationMessage>();
    app.add_message::<input::AbilityClicked>();
    app.add_systems(
        Update,
        (
            lighting::on_add_occluder,
            lighting::on_add_player,
            input::handle_input.run_if(is_player_alive.and(phone::is_phone_closed)),
            targeting::update_valid_targets,
            targeting::update_valid_target_indicators
                .run_if(resource_changed::<targeting::ValidTargets>),
            phone::toggle_phone,
            phone::update_phone,
            phone::update_streaming_stats,
            chat::update_money_timer,
            chat::update_chat,
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
                (
                    increment_turn_counter,
                    chat::update_streaming_turn,
                    tick_meters,
                )
                    .chain(),
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
                update_player_abilities,
            )
                .chain()
                .run_if(player_moved),
        )
            .chain(),
    );
    app.add_systems(
        Update,
        apply_brainrot_to_world_text.run_if(in_state(Screen::Gameplay)),
    );
    app.add_systems(
        EguiPrimaryContextPass,
        (sidebar, left_sidebar, phone::draw_phone, chat::draw_chat)
            .run_if(in_state(Screen::Gameplay)),
    );
}

pub fn apply_brainrot_ui(
    text: impl Into<WidgetText>,
    brainrot: i32,
    style: &egui::Style,
    font_selection: FontSelection,
    align: Align,
) -> WidgetText {
    let text = text.into();
    let p = ((brainrot as f32 - 60.0) / 30.0).clamp(0.0, 1.0);

    let job = text.into_layout_job(style, font_selection, align);
    if p <= 0.0 {
        return WidgetText::LayoutJob(job);
    }

    let mut new_job = LayoutJob::default();
    new_job.halign = job.halign;
    new_job.justify = job.justify;
    new_job.first_row_min_height = job.first_row_min_height;
    new_job.wrap = job.wrap.clone();

    for section in &job.sections {
        let section_text = &job.text[section.byte_range.clone()];

        let mut current_text = String::new();
        let mut current_format = section.format.clone();

        for (i, c) in section_text.chars().enumerate() {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            use std::hash::{Hash, Hasher};
            i.hash(&mut hasher);
            c.hash(&mut hasher);
            let hash = hasher.finish();
            let random_val = (hash % 1000) as f32 / 1000.0;

            let mut target_format = section.format.clone();
            if random_val < p {
                target_format.font_id.family = egui::FontFamily::Name("comic_relief".into());
            }

            // If the format changed and we have accumulated text, append it
            if target_format.font_id.family != current_format.font_id.family
                && !current_text.is_empty()
            {
                new_job.append(&current_text, 0.0, current_format.clone());
                current_text.clear();
            }

            current_format = target_format;
            current_text.push(c);
        }

        // Append the remaining text
        if !current_text.is_empty() {
            new_job.append(&current_text, 0.0, current_format);
        }
    }

    WidgetText::LayoutJob(new_job.into())
}

fn apply_brainrot_to_world_text(
    mut q_text: Query<
        (
            Entity,
            &mut TextColor,
            &mut Transform,
            &Text2d,
            &assets::BaseColor,
        ),
        Without<MoveAnimation>,
    >,
    player: Query<&Player>,
) {
    let Some(player) = player.iter().next() else {
        return;
    };
    let p = ((player.brainrot as f32 - 60.0) / 30.0).clamp(0.0, 1.0);

    for (entity, mut text_color, mut transform, text, base_color) in q_text.iter_mut() {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        entity.hash(&mut hasher);
        text.0.hash(&mut hasher);
        let hash = hasher.finish();

        let random_val = (hash % 1000) as f32 / 1000.0;

        if random_val < p {
            // Glitch effect: Color and Rotation shift
            let glitch_colors = [
                Color::srgb(0.0, 1.0, 0.0), // Neon Green
                Color::srgb(1.0, 0.0, 1.0), // Magenta
                Color::srgb(0.0, 1.0, 1.0), // Cyan
                Color::srgb(1.0, 1.0, 0.0), // Yellow
            ];
            let color_idx = (hash % glitch_colors.len() as u64) as usize;

            // Subtle color: Mix base color with glitch color based on p
            // Use a small intensity so it stays close to base color at low p
            let intensity = p * 0.5;
            text_color.0 = base_color.0.mix(&glitch_colors[color_idx], intensity);

            let rotation_jitter = ((hash % 10) as f32 - 5.0).to_radians() * p;
            transform.rotation = Quat::from_rotation_z(rotation_jitter);
        } else {
            // Restore
            text_color.0 = base_color.0;
            transform.rotation = Quat::IDENTITY;
        }
    }
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
    pub money_gain_timer: f32,
    pub last_gain_amount: i32,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Ability {
    Sprint,
    ShoulderCheck,
}

impl std::fmt::Display for Ability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Ability::Sprint => "Sprint",
            Ability::ShoulderCheck => "Shoulder Check",
        })
    }
}

pub enum AbilityTarget {
    ReachableTile {
        maxdist: i32,
    },
    NearbyMob {
        maxdist: i32,
    },
    #[allow(unused)]
    NoTarget,
}
impl Ability {
    fn target(&self) -> AbilityTarget {
        match self {
            Ability::Sprint => AbilityTarget::ReachableTile { maxdist: 5 },
            Ability::ShoulderCheck => AbilityTarget::NearbyMob { maxdist: 1 },
        }
    }
}

#[derive(Resource, Default)]
pub struct PlayerAbilities {
    // IndexSet to preserve insertion order.
    abilities: IndexSet<Ability>,
}

impl PlayerAbilities {
    fn add_or_remove(&mut self, condition: bool, ability: Ability) {
        if condition {
            self.abilities.insert(ability);
        } else {
            self.abilities.shift_remove(&ability);
        }
    }
}

fn update_player_abilities(player: Single<&Player>, mut abilities: ResMut<PlayerAbilities>) {
    abilities.add_or_remove(player.strength >= 10, Ability::Sprint);
    abilities.add_or_remove(player.strength >= 20, Ability::ShoulderCheck);
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
pub(crate) struct Turn;

#[derive(Resource, Default)]
struct PlayerMoved(bool);

fn player_moved(moved: Res<PlayerMoved>) -> bool {
    moved.0
}

#[derive(Resource, Default)]
pub(crate) struct TurnCounter(pub u64);

fn increment_turn_counter(mut counter: ResMut<TurnCounter>) {
    counter.0 += 1;
}

fn tick_meters(turn_counter: Res<TurnCounter>, player: Single<(&mut Player, &mut Creature)>) {
    let (mut player, mut creature) = player.into_inner();
    if turn_counter.0.is_multiple_of(10) {
        if player.hunger >= 100 {
            creature.hp -= 1;
        }
        player.hunger += 1;
        player.hunger = player.hunger.clamp(0, 100);

        if player.boredom >= 100 {
            creature.hp -= 1;
        }
        player.boredom += 1;
        player.boredom = player.boredom.clamp(0, 100);
    }
}

fn handle_player_move(
    mut commands: Commands,
    player: Single<(Entity, &mut MapPos, &PlayerIntent, &Player)>,
    mut mobs: Query<&mut MapPos, (With<Creature>, Without<Player>)>,
    stairs: Query<(&MapPos, &Stairs), (Without<Player>, Without<Creature>)>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
    pos_to_creature: Res<PosToCreature>,
    turn_counter: Res<TurnCounter>,
    mut damage: ResMut<PendingDamage>,
    mut moved: ResMut<PlayerMoved>,
) {
    let (player_entity, mut pos, intent, player_stats) = player.into_inner();
    commands.entity(player_entity).remove::<PlayerIntent>();

    let p = ((player_stats.brainrot as f32 - 60.0) / 30.0).clamp(0.0, 1.0);
    let sway_direction = if turn_counter.0.is_multiple_of(2) {
        1.0
    } else {
        -1.0
    };
    let sway = if p > 0.0 {
        Some(p * 0.2 * sway_direction)
    } else {
        None
    };

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
                    sway,
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
                        sway,
                    });

                    moved.0 = true;
                    break;
                }
            }
            if !moved.0 {
                return;
            }
        }
        PlayerIntent::UseAbility(ability, map_pos) => match ability {
            Ability::Sprint => {
                let old_pos = *pos;
                *pos = *map_pos;
                moved.0 = true;
                commands.entity(player_entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                    ease: EaseFunction::SineInOut,
                    rotation: None,
                    sway,
                });
            }
            Ability::ShoulderCheck => {
                // swap positions and damage
                let old_pos = *pos;
                let new_pos = map_pos.0;
                if let Some(mob_entity) = pos_to_creature.0.get(&new_pos) {
                    damage.0.push(DamageInstance {
                        entity: *mob_entity,
                        hp: 2,
                    });
                    pos.0 = new_pos;
                    if let Ok(mut mob_pos) = mobs.get_mut(*mob_entity) {
                        *mob_pos = old_pos;
                    }
                    commands.entity(player_entity).insert(MoveAnimation {
                        from: old_pos.to_vec3(PLAYER_Z),
                        to: pos.to_vec3(PLAYER_Z),
                        timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                        ease: EaseFunction::SineInOut,
                        rotation: None,
                        sway,
                    });
                    commands.entity(*mob_entity).insert(MoveAnimation {
                        from: pos.to_vec3(PLAYER_Z),
                        to: old_pos.to_vec3(PLAYER_Z),
                        timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                        ease: EaseFunction::SineInOut,
                        rotation: None,
                        sway,
                    });
                } else {
                    pos.0 = new_pos;
                    commands.entity(player_entity).insert(MoveAnimation {
                        from: old_pos.to_vec3(PLAYER_Z),
                        to: pos.to_vec3(PLAYER_Z),
                        timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                        ease: EaseFunction::SineInOut,
                        rotation: None,
                        sway,
                    });
                }
            }
        },
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
            sway: None,
        });
    }
}

#[derive(Resource, Default)]
pub(crate) struct FactionMap {
    dijkstra_map_per_faction: HashMap<i32, std::collections::HashMap<MapPos, usize>>,
}

fn reachable(
    p: MapPos,
    walk_blocked_map: &map::WalkBlockedMap,
    other_unwalkable: &HashSet<IVec2>,
) -> Vec<MapPos> {
    p.adjacent()
        .into_iter()
        .filter(|p| !walk_blocked_map.contains(&p.0))
        .filter(|p| !other_unwalkable.contains(&p.0))
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
        HashMap::<i32, std::collections::HashMap<MapPos, usize>>::new();
    for (faction, friendly_positions) in positions_per_faction.iter() {
        let enemy_positions = positions_per_faction
            .iter()
            .filter(|(f, _positions)| **f != *faction)
            .flat_map(|(_f, positions)| positions)
            .copied()
            .map(MapPos)
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

        let mut adjacent = pos.adjacent();
        adjacent.shuffle(rng);
        let target_move = adjacent
            .into_iter()
            .filter(|p| dijkstra_map.contains_key(p))
            .filter(|p| !claimed_moves.contains(&p.0))
            .min_by_key(|p| dijkstra_map.get(p).cloned().unwrap_or(usize::MAX));

        if let Some(target_move) = target_move {
            // move
            mob_moves.insert(entity, target_move);
            // Claim any move that is not a destination.
            // This works because destinations are always enemies.
            if dijkstra_map.get(&target_move) != Some(&1) {
                claimed_moves.insert(target_move.0);
            }
        }
    }

    // Apply moves.
    for (entity, dest) in mob_moves.into_iter() {
        let (entity, mut pos, _creature, mob) = mobs.get_mut(entity).unwrap();
        let old_pos = *pos;
        let new_pos = dest;
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
                sway: None,
            });
        }
    }
}

fn prune_dead(
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    mut damage_animation: MessageWriter<DamageAnimationMessage>,
    q_creatures: Query<(Entity, &Creature, &MapPos, Option<&DropsCorpse>), Without<Player>>,
    mut player: Single<&mut Player>,
    phone_state: Res<phone::PhoneState>,
    mut chat: ResMut<chat::ChatHistory>,
) {
    let world_entity = world.into_inner();
    let player = player.as_mut();
    for (entity, creature, map_pos, corpse) in q_creatures {
        if creature.is_dead() {
            commands.entity(entity).despawn();

            chat::handle_payout(player, &phone_state, &mut chat);

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
    let player_pos = *player.single().unwrap();
    let maxdist = 10;
    let reachable = |p: MapPos| p.adjacent();
    for path in rogue_algebra::path::bfs_paths(&[player_pos], maxdist, reachable) {
        if let Some(pos) = path.last()
            && let Some(mob) = pos_to_creature.0.get(&pos.0)
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
    player: Single<&Player>,
    nearby_mobs: Res<NearbyMobs>,
    examine_results: Res<examine::ExamineResults>,
    world_assets: If<Res<WorldAssets>>,
    atlas_assets: If<Res<Assets<TextureAtlasLayout>>>,
    player_abilities: Res<PlayerAbilities>,
    input_mode: Res<InputMode>,
    mut msg_ability_clicked: MessageWriter<AbilityClicked>,
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
                            ui.label(apply_brainrot_ui(
                                RichText::new(text)
                                    .size(TILE_HEIGHT)
                                    .color(c32)
                                    .background_color(if highlight {
                                        ui.style().visuals.code_bg_color
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    }),
                                player.brainrot,
                                ui.style(),
                                FontSelection::Default,
                                Align::LEFT,
                            ));
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
                    ui.label(apply_brainrot_ui(
                        &info.info,
                        player.brainrot,
                        ui.style(),
                        FontSelection::Default,
                        Align::LEFT,
                    ));
                }
            });

            ui.add_space(20.0);

            ui.group(|ui| {
                match *input_mode {
                    InputMode::Normal => {
                        ui.label(apply_brainrot_ui(
                            "move: arrow keys",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "move: hjklyubn",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "examine: x",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));

                        for (i, ability) in player_abilities.abilities.iter().enumerate() {
                            let ability_key = (i + 1) % 10;
                            let label = if matches!(ability, Ability::Sprint) {
                                format!("{ability_key}/Shift: {ability}")
                            } else {
                                format!("{ability_key}: {ability}")
                            };
                            if ui
                                .button(apply_brainrot_ui(
                                    label,
                                    player.brainrot,
                                    ui.style(),
                                    FontSelection::Default,
                                    Align::LEFT,
                                ))
                                .clicked()
                            {
                                msg_ability_clicked.write(AbilityClicked(*ability));
                            };
                        }
                    }
                    InputMode::Examine(_) => {
                        ui.label(apply_brainrot_ui(
                            RichText::new("EXAMINING"),
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "move: arrow keys",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "move: hjklyubn",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "exit: x",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                    }
                    InputMode::Targeting(ability, _pos) => {
                        ui.label(apply_brainrot_ui(
                            RichText::new(format!("TARGETING {ability}")),
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "choose target: enter",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "move: arrow keys",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "move: hjklyubn",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        ui.label(apply_brainrot_ui(
                            "exit: x",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));
                        if ability == Ability::Sprint {
                            ui.label(apply_brainrot_ui(
                                "exit: shift",
                                player.brainrot,
                                ui.style(),
                                FontSelection::Default,
                                Align::LEFT,
                            ));
                        }
                    }
                };
            });
        });
}

fn stat_label(ui: &mut egui::Ui, name: &str, brainrot: i32, is_bad: bool, time: f32) {
    if !is_bad {
        ui.label(apply_brainrot_ui(
            name,
            brainrot,
            ui.style(),
            FontSelection::Default,
            Align::LEFT,
        ));
        return;
    }

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (i, c) in name.chars().enumerate() {
            let phase = i as f32 * 0.5;
            let t = time * 10.0 - phase;
            let jump = (t.sin() * 5.0).max(0.0);

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(5.0 - jump);
                ui.label(apply_brainrot_ui(
                    c.to_string(),
                    brainrot,
                    ui.style(),
                    FontSelection::Default,
                    Align::LEFT,
                ));
                ui.add_space(jump);
            });
        }
    });
}

fn left_sidebar(
    mut contexts: EguiContexts,
    player: Single<(&Creature, &Player)>,
    time: Res<Time>,
    phone_state: Res<crate::game::phone::PhoneState>,
) {
    let (creature, player_stats) = player.into_inner();
    let ctx = contexts.ctx_mut().unwrap();
    egui::SidePanel::left("left_sidebar")
        .min_width(200.0)
        .show(ctx, |ui| {
            ui.add_space(20.0);

            ui.label(apply_brainrot_ui(
                RichText::new("PLAYER").size(24.0).strong(),
                player_stats.brainrot,
                ui.style(),
                FontSelection::Default,
                Align::LEFT,
            ));
            ui.add_space(10.0);

            let stats = [
                ("Health", creature.hp, creature.max_hp, false),
                ("Brainrot", player_stats.brainrot, 100, true),
                ("Hunger", player_stats.hunger, 100, true),
                ("Rizz", player_stats.rizz, 100, false),
                ("Strength", player_stats.strength, 100, false),
                ("Boredom", player_stats.boredom, 100, true),
            ];

            for (name, value, max, invert_colors) in stats {
                let ratio = (value as f32 / max as f32).clamp(0.0, 1.0);
                let is_bad = if !invert_colors {
                    ratio <= 0.25
                } else {
                    ratio >= 0.75
                };

                stat_label(ui, name, player_stats.brainrot, is_bad, time.elapsed_secs());
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
                let text_job = apply_brainrot_ui(
                    text,
                    player_stats.brainrot,
                    ui.style(),
                    egui::FontSelection::FontId(egui::FontId::proportional(14.0)),
                    egui::Align::Center,
                )
                .into_layout_job(
                    ui.style(),
                    egui::FontSelection::FontId(egui::FontId::proportional(14.0)),
                    egui::Align::Center,
                );

                // We need to modify text color in the layout job because apply_brainrot_ui uses default
                let mut text_job = (*text_job).clone();
                for section in &mut text_job.sections {
                    section.format.color = egui::Color32::WHITE;
                }

                let galley = ui.painter().layout_job(text_job);
                ui.painter().galley(
                    rect.center() - galley.size() / 2.0,
                    galley,
                    egui::Color32::WHITE,
                );

                ui.add_space(10.0);
            }

            ui.label(apply_brainrot_ui(
                "Signal",
                player_stats.brainrot,
                ui.style(),
                FontSelection::Default,
                Align::LEFT,
            ));
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

            ui.horizontal(|ui| {
                ui.label(apply_brainrot_ui(
                    format!("$$$: {}", player_stats.money),
                    player_stats.brainrot,
                    ui.style(),
                    FontSelection::Default,
                    Align::LEFT,
                ));

                if player_stats.money_gain_timer > 0.0 {
                    let alpha = (player_stats.money_gain_timer / 2.0).min(1.0);
                    let color =
                        egui::Color32::from_rgba_unmultiplied(0, 255, 0, (255.0 * alpha) as u8);
                    ui.label(apply_brainrot_ui(
                        RichText::new(format!(" +${}", player_stats.last_gain_amount)).color(color),
                        player_stats.brainrot,
                        ui.style(),
                        FontSelection::Default,
                        Align::LEFT,
                    ));
                }
            });

            ui.separator();
            ui.label(apply_brainrot_ui(
                format!("Subscribers: {}", phone_state.subscribers),
                player_stats.brainrot,
                ui.style(),
                FontSelection::Default,
                Align::LEFT,
            ));

            if phone_state.is_streaming {
                ui.label(apply_brainrot_ui(
                    format!("Viewers: {}", phone_state.viewers_displayed as i32),
                    player_stats.brainrot,
                    ui.style(),
                    FontSelection::Default,
                    Align::LEFT,
                ));
            }
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
