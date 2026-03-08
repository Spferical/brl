use std::{f32::consts::PI, ops::RangeInclusive, time::Duration};

use bevy::{
    asset::RenderAssetUsages,
    ecs::{schedule::ScheduleLabel, system::SystemParam},
    image::ImageSampler,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Align, Color32, FontSelection, Margin, RichText, WidgetText, text::LayoutJob},
};
#[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
use bevy_firefly::prelude::FireflyPlugin;
use rand::{
    Rng as _,
    seq::{IndexedRandom, SliceRandom as _},
};

use crate::{
    PrimaryCamera,
    asset_tracking::LoadResource as _,
    game::{
        animation::{DamageAnimationMessage, FloatingTextMessage, MoveAnimation, TitleDropMessage},
        assets::{WorldAssets, get_egui_image_from_sprite},
        debug::{DebugSettings, redo_faction_map},
        delivery::DungeonDashState,
        input::{AbilityClicked, EatEvent, InputMode, PlayerIntent, StairsClicked, WaitMessage},
        map::{
            MapPos, PlayerMemoryMap, PlayerVisibilityMap, PosToCreature, PosToInteractable,
            TILE_HEIGHT, TILE_WIDTH, WalkBlockedMap,
        },
        mapgen::{MapInfo, MobKind},
        spawn::spawn_mob,
        upgrades::{Effect, UPGRADES},
    },
    screens::Screen,
};
mod animation;
mod assets;
mod camera;
pub(crate) mod chat;
pub(crate) mod debug;
mod delivery;
mod examine;
mod input;
pub mod lighting;
mod map;
mod mapgen;
mod mobile_apps;
mod phone;
mod signal;
mod spawn;
mod targeting;
mod upgrades;

const BULLET_DAMAGE: i32 = 2;

const HIGHLIGHT_Z: f32 = 20.0;
const DAMAGE_Z: f32 = 15.0;
const PLAYER_Z: f32 = 10.0;
const CORPSE_Z: f32 = 5.0;
const TILE_Z: f32 = 0.0;

pub(crate) const PLAYER_FACTION: i32 = 0;
pub(crate) const FRIENDLY_FACTION: i32 = 2;
pub(crate) const ALLIED_FACTION: i32 = 3;
const ENEMY_FACTION: i32 = -1;

const UI_GREEN: egui::Color32 = egui::Color32::from_rgb(65, 163, 109);
const UI_YELLOW: egui::Color32 = egui::Color32::from_rgb(200, 181, 93);
const UI_RED: egui::Color32 = egui::Color32::from_rgb(227, 111, 108);

pub(super) fn plugin(app: &mut App) {
    #[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
    {
        app.add_plugins(FireflyPlugin);
    }
    app.insert_resource(ClearColor(Color::BLACK));
    app.load_resource::<assets::WorldAssets>();
    app.init_resource::<map::WalkBlockedMap>();
    app.init_resource::<map::SightBlockedMap>();
    app.init_resource::<map::PlayerVisibilityMap>();
    app.init_resource::<map::PlayerMemoryMap>();
    app.init_resource::<mapgen::MapInfo>();
    app.init_resource::<camera::ScreenShake>();
    app.init_resource::<PendingDamage>();
    app.init_resource::<PlayerMoved>();
    app.init_resource::<FactionMap>();
    app.init_resource::<PosToCreature>();
    app.init_resource::<PosToInteractable>();
    app.init_resource::<NearbyMobs>();
    app.init_resource::<LastTitleDropLevel>();
    app.init_resource::<DebugSettings>();
    app.init_resource::<lighting::LightingSettings>();
    app.init_resource::<examine::ExaminePos>();
    app.init_resource::<examine::ExamineResults>();
    app.init_resource::<input::InputMode>();
    app.init_resource::<targeting::ValidTargets>();
    app.init_resource::<phone::PhoneState>();
    app.init_resource::<delivery::DungeonDashState>();
    app.init_resource::<mobile_apps::CockatriceState>();
    app.init_resource::<mobile_apps::CrawlrState>();
    app.init_resource::<delivery::ActiveDelivery>();
    app.init_resource::<chat::StreamingState>();
    app.init_resource::<chat::ChatHistory>();
    app.init_resource::<TurnCounter>();
    app.init_state::<phone::PhoneScreen>();
    app.init_state::<delivery::DungeonDashScreen>();
    app.add_message::<DamageAnimationMessage>();
    app.add_message::<FloatingTextMessage>();
    app.add_message::<animation::TitleDropMessage>();
    app.add_message::<input::AbilityClicked>();
    app.add_message::<input::StairsClicked>();
    app.add_message::<input::WaitMessage>();
    app.add_message::<input::EatEvent>();
    app.add_message::<upgrades::UpgradeMessage>();
    app.add_systems(
        Update,
        (
            (
                lighting::update_lighting,
                update_fov_mask,
                lighting::on_add_occluder,
                lighting::on_add_player,
                (input::handle_wait_message, input::handle_input)
                    .run_if(is_player_alive.and(phone::is_phone_closed)),
                targeting::update_valid_targets,
                targeting::update_valid_target_indicators
                    .run_if(resource_changed::<targeting::ValidTargets>),
                (
                    phone::set_notification,
                    phone::toggle_phone,
                    phone::update_phone,
                    mobile_apps::update_cockatrice,
                )
                    .chain(),
                chat::update_streaming_stats,
                chat::update_money_timer,
                chat::update_chat,
            ),
            (
                update_level_info_on_change,
                (
                    animation::process_move_animations,
                    animation::process_attack_animations,
                    animation::spawn_damage_animations,
                    animation::spawn_floating_messages,
                    animation::update_floating_text,
                    animation::update_title_drop,
                )
                    .chain(),
                camera::update_camera,
                examine::update_examine_info,
                examine::highlight_examine_tile,
                delivery::draw_delivery_indicators,
                delivery::update_current_mobs,
                upgrades::handle_upgrades,
                debug::teleport_player,
                update_crawlr_animation,
            ),
        )
            .run_if(in_state(Screen::Gameplay))
            .chain(),
    );
    app.init_schedule(Turn);
    app.add_systems(
        Turn,
        (
            update_frozen,
            map::update_walk_blocked_map,
            map::update_pos_to_interactable,
            handle_player_move,
            map::update_sight_blocked_map,
            handle_eat,
            (
                (
                    increment_turn_counter,
                    mobile_apps::update_crawlr,
                    chat::update_streaming_turn,
                    tick_meters,
                    handle_subscriptions,
                    signal::update_player_signal,
                    delivery::process_deliveries,
                    delivery::process_dungeon_dash_jobs,
                    process_despawns,
                )
                    .chain(),
                // kill mobs from any player damage
                (apply_damage, prune_dead).chain(),
                // environment
                (
                    map::update_pos_to_creature,
                    apply_friend_of_machines,
                    process_spawners,
                    process_spawn_zones,
                    spawn_klarna_kop,
                    spawn_brainrot_enemies,
                    transform_brainrot_enemies,
                    transform_brainrot_corpses,
                    map::update_pos_to_creature,
                )
                    .chain(),
                // bullets
                (
                    check_bullet_collision,
                    move_bullets,
                    check_bullet_collision,
                    move_bullets,
                    check_bullet_collision,
                )
                    .chain(),
                // mobs get a turn
                (
                    build_faction_map,
                    process_mob_turn,
                    map::update_pos_to_creature,
                    check_bullet_collision,
                )
                    .chain(),
                // damage
                apply_damage,
                prune_dead,
                map::update_pos_to_creature,
                // end-of-turn bookkeeping
                redo_faction_map,
                set_player_corpse.run_if(not(is_player_alive)),
            )
                .chain()
                .run_if(player_moved),
            (
                update_frozen,
                map::update_sight_blocked_map,
                map::update_player_visibility,
                map::apply_hard_fov_to_tiles,
                update_nearby_mobs,
                map::update_pos_to_interactable,
            )
                .chain(),
        )
            .chain(),
    );
    app.add_systems(
        Update,
        (apply_brainrot_to_world_text, apply_brainrot_visual_effects)
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
    app.add_systems(
        EguiPrimaryContextPass,
        (
            sidebar,
            left_sidebar,
            draw_status_indicator,
            chat::draw_streaming_indicator,
            phone::draw_phone,
            chat::draw_chat,
            draw_interactable_popup,
        )
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

fn bevy_to_egui_color(color: Color) -> Color32 {
    let [r, g, b, a] = color.to_srgba().to_u8_array();
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn anger_crew(
    attacked_entity: Entity,
    _player_entity: Entity,
    creatures: &mut Query<
        (
            Entity,
            &MapPos,
            &mut Creature,
            Option<&mut Name>,
            Option<&Mob>,
        ),
        (Without<Player>, Without<Frozen>),
    >,
    _commands: &mut Commands,
    _pos_to_creature: &PosToCreature,
    sight_blocked_map: &map::SightBlockedMap,
) {
    let mut to_anger = Vec::new();
    let victim_pos;

    // Check if the attacked entity is a crew member or already angered
    if let Ok((_entity, pos, creature, name, mob)) = creatures.get(attacked_entity) {
        if creature.faction == FRIENDLY_FACTION
            || name
                .as_ref()
                .map(|n| n.as_str() == "Angered Crew Amogus")
                .unwrap_or(false)
        {
            victim_pos = pos.0;
            // The attacked entity is crew or already angered crew.
            // Find all other crew members who can see this
            for (other_entity, other_pos, other_creature, _other_name, other_mob) in
                creatures.iter()
            {
                if other_creature.faction == FRIENDLY_FACTION
                    && other_mob.map(|m| m.attrs.sus).unwrap_or(false)
                {
                    let fov = rogue_algebra::fov::calculate_fov(other_pos.0.into(), 10, |p| {
                        sight_blocked_map.contains(&IVec2::from(p))
                    });
                    if fov.iter().any(|&p| IVec2::from(p) == victim_pos) {
                        to_anger.push(other_entity);
                    }
                }
            }
            // Also anger the victim if they were faction 2
            if creature.faction == FRIENDLY_FACTION && mob.map(|m| m.attrs.sus).unwrap_or(false) {
                to_anger.push(attacked_entity);
            }
        } else {
            return;
        }
    } else {
        return;
    }

    to_anger.sort_unstable();
    to_anger.dedup();

    for entity in to_anger {
        if let Ok((_, _, mut creature, name, _)) = creatures.get_mut(entity) {
            creature.faction = -1; // Join the enemy faction
            if let Some(mut name) = name {
                *name = Name::new("Angered Crew Amogus");
            }
        }
    }
}

pub(crate) fn draw_interactable_popup(
    mut commands: Commands,
    mut contexts: EguiContexts,
    player_query: Single<(Entity, &MapPos, &Player)>,
    interactable_query: Query<
        (
            Entity,
            &MapPos,
            Option<&Name>,
            &Interactable,
            Option<&delivery::Food>,
        ),
        Without<Frozen>,
    >,
    q_camera: Single<(&Camera, &GlobalTransform), With<PrimaryCamera>>,
) {
    let (player_entity, player_pos, player) = player_query.into_inner();
    let (camera, camera_transform) = *q_camera;

    let mut interactables_by_pos: HashMap<MapPos, Vec<_>> = HashMap::default();

    for (entity, pos, name, interactable, food) in interactable_query.iter() {
        let is_at_pos = pos.0 == player_pos.0;
        let is_adjacent = (pos.0 - player_pos.0).abs().max_element() <= 1;

        if is_at_pos || (is_adjacent && !interactable.require_on_top) {
            interactables_by_pos
                .entry(*pos)
                .or_default()
                .push((entity, name, interactable, food));
        }
    }

    for (pos, list) in interactables_by_pos {
        // Sort by entity for consistent order
        let mut sorted_list = list;
        sorted_list.sort_by_key(|(entity, _, _, _)| *entity);

        for (i, (entity, name, interactable, food)) in sorted_list.into_iter().enumerate() {
            // Get screen position
            let world_pos = pos.to_vec3(PLAYER_Z);
            let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, world_pos) else {
                continue;
            };

            let Ok(ctx) = contexts.ctx_mut() else {
                return;
            };

            let (title, description) = if let Some(food) = food {
                let food_item = delivery::FOODS[food.food_idx];
                let verb = if food_item.rizz > 0 { "Equip" } else { "Eat" };
                (
                    format!("{} {}?", verb, food_item.name),
                    Some(food_item.effects.to_string()),
                )
            } else {
                let name_str = name.map(|n| n.as_str()).unwrap_or("");
                (
                    format!("{} {}?", interactable.action, name_str),
                    interactable.description.clone(),
                )
            };

            draw_world_popup(
                ctx,
                viewport_pos,
                title,
                description,
                player.brainrot,
                entity,
                i as f32 * 100.0,
                &mut commands,
                player_entity,
            );
        }
    }
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

    let mut new_job = LayoutJob {
        halign: job.halign,
        justify: job.justify,
        first_row_min_height: job.first_row_min_height,
        wrap: job.wrap.clone(),
        ..Default::default()
    };

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
        (Without<MoveAnimation>, Without<Frozen>, Without<Bullet>),
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

fn apply_brainrot_visual_effects(
    mut q_transforms: Query<
        (
            &map::MapPos,
            &mut Transform,
            Option<&lighting::Occluder>,
            Has<Creature>,
            Has<Player>,
        ),
        (
            Without<animation::MoveAnimation>,
            Without<animation::AttackAnimation>,
            Without<Frozen>,
        ),
    >,
    player_q: Query<(&map::MapPos, &Player)>,
    time: Res<Time>,
) {
    let Some((player_pos, player)) = player_q.iter().next() else {
        return;
    };

    let brainrot = player.brainrot;

    // Bulge walls
    let bulge_p = ((brainrot as f32 - 80.0) / 20.0).clamp(0.0, 1.0);
    let player_world = player_pos.to_vec3(TILE_Z);

    // Bounce amplitudes
    let bounce_mobs_p = ((brainrot as f32 - 80.0) / 20.0).clamp(0.0, 1.0);
    let bounce_all_p = ((brainrot as f32 - 90.0) / 10.0).clamp(0.0, 1.0);

    for (map_pos, mut transform, occluder, is_creature, is_player) in q_transforms.iter_mut() {
        let base_z = transform.translation.z;
        let mut target_pos = map_pos.to_vec3(base_z);

        if occluder.is_some() && bulge_p > 0.0 {
            let diff = target_pos.truncate() - player_world.truncate();
            let dist = diff.length();
            if dist > 0.0 {
                let dir = diff / dist;
                let max_displacement = map::TILE_WIDTH * 0.4;
                let radius = map::TILE_WIDTH * 4.0;
                let strength = (1.0 - (dist / radius)).clamp(0.0, 1.0);

                let displacement = dir * max_displacement * bulge_p * strength;
                target_pos += displacement.extend(0.0);
            }
        }

        let bounce_amp = if is_player {
            0.0
        } else if is_creature {
            bounce_mobs_p
        } else {
            bounce_all_p
        };

        if bounce_amp > 0.0 {
            let x = map_pos.0.x as f32;
            let y = map_pos.0.y as f32;
            let time_s = time.elapsed_secs() * 5.0;

            let wave1 = (time_s + x * 0.5 + y * 0.5).sin();
            let wave2 = (time_s * 1.3 - x * 0.8 + y * 0.2).cos();
            let wave3 = (time_s * 0.7 + (x * x + y * y).sqrt() * 0.3).sin();

            let combined = (wave1 + wave2 + wave3) / 3.0;
            let bounce_val = (combined + 1.0) * 0.5;

            // Squash and stretch
            let stretch = 1.0 + (bounce_val * 0.5 * bounce_amp);
            let squash = 1.0 - (bounce_val * 0.3 * bounce_amp);

            transform.scale = Vec3::new(squash, stretch, 1.0);
        } else {
            transform.scale = Vec3::ONE;
        }

        transform.translation = target_pos;
    }
}
#[derive(Component)]
pub struct Frozen;

#[derive(Component)]
pub struct GameWorld;

#[derive(Component)]
pub struct FovMask;

#[derive(Component, Reflect)]
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
    pub max_depth: i32,
    pub abilities: Vec<Ability>,
    pub ability_cooldowns: HashMap<Ability, u32>,

    pub upgrades: Vec<usize>,
    pub pending_upgrades: usize,
    pub upgrade_options: Vec<usize>,
    pub subscriptions: Vec<Subscription>,
    pub food_cooldowns: HashMap<usize, u32>,
    pub is_raided: bool,
    pub high_metabolism: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum Subscription {
    DungeonDashPlatinum,
    UndergroundTVPro,
    FiveGLTE,
    DungeonFitness,
}

impl Subscription {
    pub fn name(&self) -> &'static str {
        match self {
            Subscription::DungeonDashPlatinum => "DungeonDash Platinum",
            Subscription::UndergroundTVPro => "UndergroundTV Pro",
            Subscription::FiveGLTE => "5G LTE",
            Subscription::DungeonFitness => "Dungeon Fitness",
        }
    }

    pub fn cost(&self) -> i32 {
        match self {
            Subscription::DungeonDashPlatinum => 20,
            Subscription::UndergroundTVPro => 50,
            Subscription::FiveGLTE => 5,
            Subscription::DungeonFitness => 80,
        }
    }
}

impl Player {
    pub fn has_subscription(&self, sub: Subscription) -> bool {
        self.subscriptions.contains(&sub)
    }

    pub fn apply_hunger_damage(&mut self, creature: &mut Creature, amount: i32) {
        self.hunger += amount;
        if self.hunger > 100 {
            let overflow = self.hunger - 100;
            self.hunger = 100;
            self.apply_strength_damage(creature, overflow);
        }
    }

    pub fn apply_strength_damage(&mut self, creature: &mut Creature, amount: i32) {
        self.strength -= amount;
        if self.strength < 0 {
            creature.hp += self.strength;
            self.strength = 0;
        }
    }

    pub fn apply_boredom(&mut self, creature: &mut Creature, amount: i32) {
        self.boredom += amount;
        if self.boredom >= 100 {
            creature.hp -= 1;
        }
        self.boredom = self.boredom.clamp(0, 100);
    }
    #[allow(unused)]
    fn add_or_remove_ability(&mut self, condition: bool, ability: Ability) {
        let ability_idx = self.abilities.iter().position(|a| *a == ability);
        if condition && ability_idx.is_none() {
            self.abilities.push(ability);
        } else if let Some(idx) = ability_idx {
            self.abilities.remove(idx);
        }
    }

    fn melee_damage(&self) -> i32 {
        (self.strength / 10).max(1)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum Ability {
    Sprint,
    ShoulderCheck,
    Mog,
    Cook,
    Gun,
    ReadBook,
    Yap,
    Surveys,
}

impl std::fmt::Display for Ability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Ability::Sprint => "Sprint",
            Ability::ShoulderCheck => "Shoulder Check",
            Ability::Mog => "Mog",
            Ability::Cook => "Cook",
            Ability::ReadBook => "Read Book",
            Ability::Yap => "Yap",
            Ability::Gun => "Shoot",
            Ability::Surveys => "Fill Surveys for Cash",
        })
    }
}

pub enum AbilityTarget {
    NearbyTile {
        maxdist: i32,
    },
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
    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Ability::Sprint => "Move multiple tiles in one turn. Costs hunger.",
            Ability::ShoulderCheck => "Damage and swap positions with an adjacent enemy.",
            Ability::Mog => "Deal aura damage to an enemy up to 2 tiles way. Scales with rizz.",
            Ability::Cook => "Cook a corpse you are standing on. Requires < 10 brainrot.",
            Ability::Gun => "Get a big iron on your hip.",
            Ability::ReadBook => {
                "Reduce brainrot. Might be a little boring. (Borrow period: 10 turns)"
            }
            Ability::Yap => {
                "Deal boredom damage to an enemy up to 3 tiles away. Scales with boredom. Increases your own boredom."
            }
            Ability::Surveys => "Gain money and boredom. Requires < 50 brainrot.",
        }
    }
    pub(crate) fn target(&self) -> AbilityTarget {
        match self {
            Ability::Sprint => AbilityTarget::ReachableTile { maxdist: 5 },
            Ability::ShoulderCheck => AbilityTarget::NearbyMob { maxdist: 1 },
            Ability::Mog => AbilityTarget::NearbyMob { maxdist: 2 },
            Ability::Gun => AbilityTarget::NearbyTile { maxdist: 1 },
            Ability::Cook | Ability::ReadBook => AbilityTarget::NoTarget,
            Ability::Yap => AbilityTarget::NearbyMob { maxdist: 3 },
            Ability::Surveys => AbilityTarget::NoTarget,
        }
    }

    pub(crate) fn damage_info(&self, player: &Player) -> Option<(DamageType, RangeInclusive<i32>)> {
        let Player { rizz, boredom, .. } = player;
        match self {
            Ability::Sprint => None,
            Ability::ShoulderCheck => Some((
                DamageType::Physical,
                player.melee_damage()..=player.melee_damage(),
            )),
            Ability::Mog => Some((
                DamageType::Aura,
                2 + (rizz * 6) / 100..=2 + (rizz * 12) / 100,
            )),
            Ability::Cook => None,
            Ability::Gun => Some((DamageType::Physical, BULLET_DAMAGE..=BULLET_DAMAGE)),
            Ability::ReadBook => None,
            Ability::Surveys => None,
            Ability::Yap => Some((DamageType::Boredom, 1 + boredom / 50..=1 + boredom / 25)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionType {
    Stairs,
    Eat,
    Workout,
    Irradiate,
    MedicalPod,
    Arcade,
    Upgrade(Option<usize>),
}

#[derive(Component)]
pub struct Interactable {
    pub action: String,
    pub description: Option<String>,
    pub kind: InteractionType,
    pub require_on_top: bool,
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            action: "Use".to_string(),
            description: None,
            kind: InteractionType::Stairs,
            require_on_top: false,
        }
    }
}

#[derive(Component, Clone)]
pub(crate) struct CookedMeal {
    pub hunger: i32,
    pub hp: i32,
    pub strength: i32,
    pub rizz: i32,
    pub brainrot: i32,
    pub boredom: i32,
}

pub(crate) fn handle_eat(
    mut events: MessageReader<EatEvent>,
    player_query: Single<(&mut Player, &mut Creature, Entity)>,
    food_query: Query<(
        Option<&delivery::Food>,
        Option<&CookedMeal>,
        Has<Ambrosia>,
        Option<&Name>,
    )>,
    mut commands: Commands,
    streaming_state: Res<chat::StreamingState>,
    mut chat: ResMut<chat::ChatHistory>,
    dd_selection: Option<ResMut<DungeonDashState>>,
    mut floating_text: MessageWriter<crate::game::animation::FloatingTextMessage>,
) {
    let mut dd_selection = dd_selection;
    let (mut player, mut creature, player_entity) = player_query.into_inner();
    for event in events.read() {
        if let Ok((food, cooked, is_ambrosia, name)) = food_query.get(event.0) {
            let mut name_str = name.map(|n| n.as_str()).unwrap_or("Something unknown");

            if let Some(ref mut dd) = dd_selection
                && Some(event.0) == dd.dropped_food_entity
            {
                let amount = dd.active_job_amount.unwrap_or(10);
                player.money -= amount * 2;
                floating_text.write(crate::game::animation::FloatingTextMessage {
                    entity: Some(player_entity),
                    world_pos: None,
                    text: "You bit it, you bought it!".to_string(),
                    color: Color::srgb(1.0, 0.0, 0.0),
                    ..default()
                });
                dd.active_job_turns = None;
                dd.active_job_amount = None;
                dd.job_target = None;
                dd.failed_job_turns = Some(10);
                dd.dropped_food_entity = None;
            }

            if is_ambrosia {
                use rand::Rng;
                let mut rng = rand::rng();
                let amount = rng.random_range(5..=15);
                match rng.random_range(0..3) {
                    0 => player.brainrot += amount,
                    1 => player.rizz += amount,
                    _ => player.strength += amount,
                }
                creature.hp = (creature.hp + 1).clamp(0, creature.max_hp);
                name_str = "Ambrosia";
            } else if let Some(food) = food {
                let food_item = delivery::FOODS[food.food_idx];
                player.hunger = (player.hunger + food_item.hunger).clamp(0, 100);
                player.strength += food_item.strength;
                player.rizz += food_item.rizz;
                creature.hp = (creature.hp + food_item.hp).clamp(0, creature.max_hp);
                name_str = food_item.name;
            } else if let Some(cooked) = cooked {
                player.hunger = (player.hunger - cooked.hunger).clamp(0, 100);
                player.strength += cooked.strength;
                player.rizz += cooked.rizz;
                player.brainrot += cooked.brainrot;
                player.apply_boredom(&mut creature, -cooked.boredom);
                creature.hp = (creature.hp + cooked.hp).clamp(0, creature.max_hp);
            }

            chat::handle_food_payout(&mut player, &streaming_state, &mut chat, name_str);
            commands.entity(event.0).despawn();
        }
    }
}

#[derive(Component)]
pub(crate) struct Ambrosia;

#[derive(Component)]
#[require(Interactable)]
pub struct Stairs {
    pub(crate) destination: MapPos,
}

pub(crate) fn draw_world_popup(
    ctx: &egui::Context,
    viewport_pos: Vec2,
    title: String,
    description: Option<String>,
    brainrot: i32,
    id_entity: Entity,
    offset_y: f32,
    commands: &mut Commands,
    player_entity: Entity,
) {
    egui::Area::new(egui::Id::new(id_entity))
        .fixed_pos(egui::pos2(
            viewport_pos.x - 100.0,
            viewport_pos.y - 120.0 - offset_y,
        ))
        .show(ctx, |ui| {
            egui::Frame::window(ui.style())
                .fill(egui::Color32::from_rgba_premultiplied(30, 30, 30, 240))
                .show(ui, |ui| {
                    ui.set_width(200.0);
                    ui.vertical_centered(|ui| {
                        ui.label(apply_brainrot_ui(
                            egui::RichText::new(title)
                                .size(18.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                            brainrot,
                            ui.style(),
                            egui::FontSelection::Default,
                            egui::Align::Center,
                        ));
                        if let Some(desc) = description {
                            ui.label(apply_brainrot_ui(
                                egui::RichText::new(desc)
                                    .size(14.0)
                                    .color(egui::Color32::LIGHT_GRAY),
                                brainrot,
                                ui.style(),
                                egui::FontSelection::Default,
                                egui::Align::Center,
                            ));
                        }
                        if ui
                            .button(apply_brainrot_ui(
                                "Interact (e)",
                                brainrot,
                                ui.style(),
                                egui::FontSelection::Default,
                                egui::Align::Center,
                            ))
                            .clicked()
                        {
                            commands
                                .entity(player_entity)
                                .insert(PlayerIntent::Interact(id_entity));
                            commands.run_schedule(Turn);
                        }
                    });
                });
        });
}

#[derive(Component)]
pub(crate) struct Corpse {
    pub nutrition: i32,
    pub name: String,
    pub kind: MobKind,
}

#[derive(Component)]
pub(crate) struct DespawnAfterTurns(pub u32);

#[derive(Component, Clone)]
pub(crate) struct DropsCorpse {
    pub sprite: assets::AsciiSprite,
    pub nutrition: i32,
    pub name: String,
    pub kind: MobKind,
}

#[derive(Clone, Bundle)]
pub(crate) struct MobBundle {
    pub name: Name,
    pub creature: Creature,
    pub mob: Mob,
    pub sprite: assets::AsciiSprite,
    pub corpse: DropsCorpse,
}

#[derive(Component)]
struct MobSpawner {
    spawns: Vec<MobBundle>,
    odds: f64,
}

#[derive(Component)]
struct Bullet {
    direction: IVec2,
    damage: i32,
    attacker: Entity,
}

/// Common fields between the player and mobs.
#[derive(Component, Clone, Debug, Reflect)]
pub(crate) struct Creature {
    pub hp: i32,
    pub max_hp: i32,
    pub faction: i32,
    pub killed_by_player: bool,
    pub machine: bool,
    pub friend_of_machines: bool,
}

fn is_enemy(faction: i32, other: i32) -> bool {
    if faction == FRIENDLY_FACTION || other == FRIENDLY_FACTION {
        false
    } else if faction == ALLIED_FACTION {
        other == ENEMY_FACTION
    } else if other == ALLIED_FACTION {
        faction == ENEMY_FACTION
    } else if faction == ENEMY_FACTION {
        other == PLAYER_FACTION
    } else {
        other != faction
    }
}

impl Creature {
    fn is_dead(&self) -> bool {
        self.hp <= 0
    }
}

fn is_player_alive(player: Single<&Creature, With<Player>>, settings: Res<DebugSettings>) -> bool {
    settings.nohurt || player.hp > 0
}

#[derive(Clone, Debug, Reflect, Default)]
pub(crate) struct Summon {
    kind: MobKind,
    delay: u64,
}

#[derive(Clone, Debug, Reflect, Default)]
pub(crate) struct MobAttrs {
    pub based: bool,
    pub basic: bool,
    pub mog_risk: bool,
    pub sus: bool,
    pub friendly: bool,
    pub knows_player_location: bool,
    pub raids_player: bool,
    pub summon: Option<Summon>,
    pub aura_resist: Resist,
    pub physical_resist: Resist,
    pub psychic_resist: Resist,
    pub boredom_resist: Resist,
}

#[derive(Default, Clone, Copy, Debug, Reflect)]
pub(crate) enum Resist {
    Weak,
    #[default]
    Normal,
    Strong,
}

// NPC-specific fields.
#[derive(Component, Clone, Debug, Reflect, Default)]
pub(crate) struct Mob {
    pub melee_damage: i32,
    pub ranged: bool,
    pub keepaway: bool,
    pub attrs: MobAttrs,
    pub target: Option<IVec2>,
    pub destination: Option<IVec2>,
}

#[derive(Component)]
pub(crate) struct BrainrotEnemyMarker;

impl Mob {
    fn get_melee_damage_type(&self) -> DamageType {
        if self.attrs.based {
            DamageType::Psychic
        } else if self.attrs.mog_risk {
            DamageType::Aura
        } else if self.attrs.basic {
            DamageType::Boredom
        } else {
            DamageType::Physical
        }
    }

    fn get_damage_resist(&self, ty: DamageType) -> Resist {
        match ty {
            DamageType::Physical => self.attrs.physical_resist,
            DamageType::Psychic => self.attrs.psychic_resist,
            DamageType::Aura => self.attrs.aura_resist,
            DamageType::Boredom => self.attrs.boredom_resist,
            DamageType::Hunger | DamageType::Strength => Resist::Normal,
        }
    }
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

fn tick_meters(
    turn_counter: Res<TurnCounter>,
    player: Single<(&mut Player, &mut Creature, &MapPos)>,
    map_info: Res<mapgen::MapInfo>,
    q_grass: Query<&MapPos, With<map::Grass>>,
) {
    let (mut player, mut creature, player_pos) = player.into_inner();

    // Touching Grass logic
    if let Some(level) = map_info.get_level(*player_pos)
        && level.ty == mapgen::LevelTitle::Minecraft
        && q_grass.iter().any(|gp| gp.0 == player_pos.0)
        && player.brainrot > 10
    {
        player.brainrot = (player.brainrot - 2).max(10);
    }

    // Passive brainrot decay
    if turn_counter.0.is_multiple_of(10) {
        player.brainrot = (player.brainrot - 1).max(0);
    }

    // Decrement cooldowns
    for cooldown in player.ability_cooldowns.values_mut() {
        if *cooldown > 0 {
            *cooldown -= 1;
        }
    }
    for cooldown in player.food_cooldowns.values_mut() {
        if *cooldown > 0 {
            *cooldown -= 1;
        }
    }

    let hunger_cooldown = if player.high_metabolism { 3 } else { 5 };
    if turn_counter.0.is_multiple_of(hunger_cooldown) {
        player.apply_hunger_damage(&mut creature, 1);
    }

    if player.high_metabolism && turn_counter.0.is_multiple_of(10) {
        creature.hp = (creature.hp + 1).clamp(0, creature.max_hp);
    }

    if turn_counter.0.is_multiple_of(5) {
        player.apply_hunger_damage(&mut creature, 1);

        player.apply_boredom(&mut creature, 1);

        if player.has_subscription(Subscription::DungeonFitness) && player.strength < 60 {
            player.strength += 1;
        }
    }
}

fn handle_subscriptions(turn_counter: Res<TurnCounter>, mut player: Single<&mut Player>) {
    if turn_counter.0 > 0 && turn_counter.0.is_multiple_of(100) {
        for sub in player.subscriptions.clone() {
            player.money -= sub.cost();
        }
    }
}

fn process_despawns(
    mut commands: Commands,
    mut q_despawns: Query<(Entity, &mut DespawnAfterTurns)>,
) {
    for (entity, mut despawn) in q_despawns.iter_mut() {
        if despawn.0 > 0 {
            despawn.0 -= 1;
        }

        if despawn.0 == 0 {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(SystemParam)]
struct PlayerMoveParams<'w, 's> {
    commands: Commands<'w, 's>,
    msg_stairs_clicked: MessageWriter<'w, StairsClicked>,
    msg_eat: MessageWriter<'w, EatEvent>,
    walk_blocked_map: Res<'w, map::WalkBlockedMap>,
    pos_to_creature: Res<'w, PosToCreature>,
    turn_counter: Res<'w, TurnCounter>,
    damage: ResMut<'w, PendingDamage>,
    moved: ResMut<'w, PlayerMoved>,
    screen_shake: ResMut<'w, camera::ScreenShake>,
    pos_to_interactable: Res<'w, PosToInteractable>,
    assets: Res<'w, assets::WorldAssets>,
    floating_text: MessageWriter<'w, FloatingTextMessage>,
    chat: ResMut<'w, crate::game::chat::ChatHistory>,
    streaming_state: Res<'w, crate::game::chat::StreamingState>,
    sight_blocked_map: Res<'w, map::SightBlockedMap>,
}

fn handle_player_move(
    params: PlayerMoveParams,
    player: Single<
        (
            Entity,
            &mut MapPos,
            Option<&PlayerIntent>,
            &mut Player,
            &mut Creature,
        ),
        With<Player>,
    >,
    mut creatures: Query<
        (
            Entity,
            &MapPos,
            &mut Creature,
            Option<&mut Name>,
            Option<&Mob>,
        ),
        (Without<Player>, Without<Frozen>),
    >,
    stairs: Query<&Stairs, (Without<Player>, Without<Creature>, Without<Frozen>)>,
    interactables: Query<&Interactable, Without<Frozen>>,
    q_corpses: Query<
        (Entity, &MapPos, &Corpse),
        (
            With<Corpse>,
            Without<Player>,
            Without<Creature>,
            Without<Frozen>,
        ),
    >,
    world: Single<Entity, With<GameWorld>>,
) {
    let PlayerMoveParams {
        mut commands,
        mut msg_stairs_clicked,
        mut msg_eat,
        walk_blocked_map,
        pos_to_creature,
        turn_counter,
        mut damage,
        mut moved,
        mut screen_shake,
        pos_to_interactable,
        assets,
        mut floating_text,
        mut chat,
        streaming_state,
        sight_blocked_map,
    } = params;
    let world_entity = world.into_inner();
    let (player_entity, mut pos, maybe_intent, mut player_stats, mut creature) =
        player.into_inner();

    let Some(intent) = maybe_intent else {
        return;
    };
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
        PlayerIntent::Move(target) => {
            let old_pos = *pos;
            let diff = target.0 - old_pos.0;
            if diff.abs().max_element() > 1 {
                moved.0 = false;
                return;
            }

            if walk_blocked_map.contains(&target.0) {
                if let Some(entity) = pos_to_interactable.0.get(target).and_then(|v| v.first()) {
                    // Turn move into interaction
                    commands
                        .entity(player_entity)
                        .insert(PlayerIntent::Interact(*entity));
                    // Re-run schedule to process the interaction immediately
                    commands.run_schedule(Turn);
                    return;
                }
                moved.0 = false;
                return;
            }

            let new_pos = target;

            if let Some(entity) = pos_to_creature.0.get(&new_pos.0) {
                damage.0.push(DamageInstance {
                    entity: *entity,
                    attacker: Some(player_entity),
                    amount: player_stats.melee_damage(),
                    ty: DamageType::Physical,
                });
                anger_crew(
                    *entity,
                    player_entity,
                    &mut creatures,
                    &mut commands,
                    &pos_to_creature,
                    &sight_blocked_map,
                );
                commands
                    .entity(player_entity)
                    .insert(animation::AttackAnimation {
                        direction: (new_pos.0 - old_pos.0).as_vec2(),
                        timer: Timer::new(Duration::from_millis(150), TimerMode::Once),
                        base_translation: old_pos.to_vec3(PLAYER_Z),
                    });
                screen_shake.trauma = (screen_shake.trauma + 0.4).min(1.0);
            } else {
                pos.0 = new_pos.0;
                commands.entity(player_entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                    ease: EaseFunction::SineInOut,
                    base_rotation: None,
                    rotation: None,
                    sway,
                });
            }
        }
        PlayerIntent::Wait => {}
        PlayerIntent::Interact(entity) => {
            if let Ok(interactable) = interactables.get(*entity) {
                match interactable.kind {
                    InteractionType::Stairs => {
                        msg_stairs_clicked.write(StairsClicked);
                    }
                    InteractionType::Eat => {
                        msg_eat.write(EatEvent(*entity));
                    }
                    InteractionType::Workout => {
                        player_stats.strength += 1;
                        player_stats.hunger += 5;
                    }
                    InteractionType::Arcade => {
                        player_stats.money -= 1;
                        player_stats.boredom -= 5;
                        player_stats.brainrot += 1;
                    }
                    InteractionType::Irradiate => {
                        player_stats.brainrot += 5;
                        player_stats.strength = (player_stats.strength - 1).max(1);
                        floating_text.write(FloatingTextMessage {
                            entity: Some(player_entity),
                            world_pos: None,
                            text: "IRRADIATED".to_string(),
                            color: Color::srgb(0.0, 1.0, 0.0),
                            ..default()
                        });
                    }
                    InteractionType::MedicalPod => {
                        creature.hp = (creature.hp + 5).min(creature.max_hp);
                        commands.entity(*entity).remove::<Interactable>();
                        commands
                            .entity(*entity)
                            .insert(assets.get_ascii_sprite('x', Color::srgb(0.2, 0.4, 0.4)));
                        commands
                            .entity(*entity)
                            .insert(Name::new("Depleted Medical Pod"));
                        floating_text.write(FloatingTextMessage {
                            entity: Some(player_entity),
                            world_pos: None,
                            text: "HEALED".to_string(),
                            color: Color::srgb(0.0, 0.8, 0.8),
                            ..default()
                        });
                    }
                    InteractionType::Upgrade(idx) => {
                        player_stats.pending_upgrades += 1;
                        if let Some(idx) = idx {
                            player_stats.upgrade_options.push(idx);
                        }
                        commands.entity(*entity).remove::<Interactable>();
                        commands
                            .entity(*entity)
                            .insert(assets.get_ascii_sprite('x', Color::srgb(0.2, 0.4, 0.4)));
                        commands.entity(*entity).insert(Name::new("Rubble"));
                        floating_text.write(FloatingTextMessage {
                            entity: Some(player_entity),
                            world_pos: None,
                            text: "UPGRADE".to_string(),
                            color: Color::srgb(0.2, 0.2, 0.8),
                            ..default()
                        });
                    }
                }
            }
        }
        PlayerIntent::UseStairs => {
            if let Some(Stairs { destination }) = stairs
                .iter_many(pos_to_interactable.0.get(&*pos).unwrap_or(&vec![]))
                .next()
            {
                let old_pos = *pos;
                *pos = *destination;

                commands.entity(player_entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                    ease: EaseFunction::SineInOut,
                    base_rotation: None,
                    rotation: None,
                    sway,
                });

                moved.0 = true;
            } else {
                return;
            }
        }
        PlayerIntent::UseAbility(ability, map_pos) => match ability {
            Ability::Sprint => {
                let old_pos = *pos;
                *pos = *map_pos;
                moved.0 = true;
                let dist = (pos.0).manhattan_distance(old_pos.0);
                player_stats.apply_hunger_damage(&mut creature, dist as i32);
                commands.entity(player_entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                    ease: EaseFunction::SineInOut,
                    base_rotation: None,
                    rotation: None,
                    sway,
                });
            }
            Ability::ShoulderCheck => {
                // swap positions and damage
                let old_pos = *pos;
                let new_pos = map_pos.0;
                if let Some(mob_entity) = pos_to_creature.0.get(&new_pos) {
                    let (ty, range) = Ability::ShoulderCheck.damage_info(&player_stats).unwrap();
                    let amount = rand::rng().random_range(range);
                    damage.0.push(DamageInstance {
                        entity: *mob_entity,
                        attacker: Some(player_entity),
                        amount,
                        ty,
                    });
                    anger_crew(
                        *mob_entity,
                        player_entity,
                        &mut creatures,
                        &mut commands,
                        &pos_to_creature,
                        &sight_blocked_map,
                    );
                    pos.0 = new_pos;
                    commands.entity(*mob_entity).insert(old_pos);
                    commands.entity(player_entity).insert(MoveAnimation {
                        from: old_pos.to_vec3(PLAYER_Z),
                        to: pos.to_vec3(PLAYER_Z),
                        timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                        ease: EaseFunction::SineInOut,
                        base_rotation: None,
                        rotation: None,
                        sway,
                    });
                    commands.entity(*mob_entity).insert(MoveAnimation {
                        from: pos.to_vec3(PLAYER_Z),
                        to: old_pos.to_vec3(PLAYER_Z),
                        timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                        ease: EaseFunction::SineInOut,
                        base_rotation: None,
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
                        base_rotation: None,
                        rotation: None,
                        sway,
                    });
                }
            }
            Ability::Mog => {
                let new_pos = map_pos;
                let old_pos = pos;
                if let Some(mob_entity) = pos_to_creature.0.get(&new_pos.0) {
                    let (ty, range) = Ability::Mog.damage_info(&player_stats).unwrap();
                    let amount = rand::rng().random_range(range);
                    damage.0.push(DamageInstance {
                        entity: *mob_entity,
                        attacker: Some(player_entity),
                        amount,
                        ty,
                    });
                    anger_crew(
                        *mob_entity,
                        player_entity,
                        &mut creatures,
                        &mut commands,
                        &pos_to_creature,
                        &sight_blocked_map,
                    );
                    commands
                        .entity(player_entity)
                        .insert(animation::AttackAnimation {
                            direction: (new_pos.0 - old_pos.0).as_vec2(),
                            timer: Timer::new(Duration::from_millis(150), TimerMode::Once),
                            base_translation: old_pos.to_vec3(PLAYER_Z),
                        });
                    crate::game::chat::queue_mog_message(&mut chat, &streaming_state);
                }
            }
            Ability::Gun => {
                commands.entity(world_entity).with_children(|parent| {
                    parent.spawn(get_bullet_bundle(
                        *map_pos,
                        map_pos.0 - pos.0,
                        player_entity,
                        &assets,
                    ));
                });
            }
            Ability::Yap => {
                let new_pos = map_pos;
                if let Some(mob_entity) = pos_to_creature.0.get(&new_pos.0) {
                    let (ty, range) = Ability::Yap.damage_info(&player_stats).unwrap();
                    let amount = rand::rng().random_range(range);
                    damage.0.push(DamageInstance {
                        entity: *mob_entity,
                        attacker: Some(player_entity),
                        amount,
                        ty,
                    });
                    player_stats.boredom += 3;
                    anger_crew(
                        *mob_entity,
                        player_entity,
                        &mut creatures,
                        &mut commands,
                        &pos_to_creature,
                        &sight_blocked_map,
                    );
                    crate::game::chat::queue_yap_message(&mut chat, &streaming_state);
                }
            }
            Ability::Cook => {
                if player_stats.brainrot < 10
                    && let Some((corpse_entity, corpse_pos, corpse)) =
                        q_corpses.iter().find(|(_, corpse_pos, corpse)| {
                            corpse_pos.0 == pos.0 && corpse.nutrition > 0
                        })
                {
                    let (meal_name, meal_stats) = corpse.kind.get_cooked_meal();

                    let transform = Transform::from_translation(corpse_pos.to_vec3(CORPSE_Z));
                    let sprite = assets.get_ascii_sprite('%', Color::srgb(0.5, 0.25, 0.0));
                    let meal_id = commands
                        .spawn((
                            Name::new(meal_name),
                            Corpse {
                                nutrition: 0,
                                name: format!("Cooked {}", corpse.name),
                                kind: corpse.kind,
                            },
                            DespawnAfterTurns(50),
                            meal_stats,
                            Interactable {
                                action: "Eat".to_string(),
                                description: Some(format!("A freshly cooked {}!", meal_name)),
                                kind: InteractionType::Eat,
                                require_on_top: false,
                            },
                            sprite,
                            *corpse_pos,
                            transform,
                            GlobalTransform::IDENTITY,
                            InheritedVisibility::VISIBLE,
                        ))
                        .id();
                    commands.entity(world_entity).add_child(meal_id);
                    floating_text.write(FloatingTextMessage {
                        entity: Some(meal_id),
                        world_pos: None,
                        text: format!("Cooked {}!", meal_name),
                        color: Color::srgb(1.0, 1.0, 0.0),
                        ..default()
                    });

                    commands.entity(corpse_entity).despawn();
                }
            }
            Ability::ReadBook => {
                if *player_stats
                    .ability_cooldowns
                    .get(&Ability::ReadBook)
                    .unwrap_or(&0)
                    == 0
                {
                    let boredom_increase = if player_stats.brainrot < 30 { 5 } else { 10 };
                    player_stats.brainrot = (player_stats.brainrot - 20).max(0);
                    player_stats.apply_boredom(&mut creature, boredom_increase);
                    player_stats.ability_cooldowns.insert(Ability::ReadBook, 10);
                } else {
                    moved.0 = false;
                    return;
                }
            }
            Ability::Surveys => {
                if player_stats.brainrot <= 50 {
                    player_stats.apply_boredom(&mut creature, 10);
                    player_stats.money += 1;
                } else {
                    moved.0 = false;
                    return;
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
    q_spawners: Query<(&MapPos, &MobSpawner), Without<Frozen>>,
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

fn get_valid_spawn_spots(
    ppos: IVec2,
    tiles: &Query<
        &MapPos,
        (
            With<map::Tile>,
            Without<map::BlocksMovement>,
            Without<Frozen>,
        ),
    >,
    pos_to_creature: &Res<map::PosToCreature>,
    map_info: &Res<mapgen::MapInfo>,
) -> Vec<MapPos> {
    let mut valid_spots = Vec::new();
    let search_range = 100;
    let cur_level = map_info.get_level(MapPos(ppos));
    for &MapPos(pos) in tiles.iter() {
        let diff = (pos - ppos).abs();
        let dist = diff.max_element();
        if dist > 2 && dist <= search_range && !pos_to_creature.0.contains_key(&pos) {
            if let Some(level) = cur_level {
                if level.rect.contains(rogue_algebra::Pos::from(pos)) {
                    valid_spots.push(MapPos(pos));
                }
            } else {
                valid_spots.push(MapPos(pos));
            }
        }
    }
    valid_spots
}

fn get_random_brainrot_attrs(rng: &mut impl rand::Rng) -> MobAttrs {
    let mut attrs = MobAttrs {
        knows_player_location: true,
        ..Default::default()
    };

    // Randomly assign some traits
    attrs.sus = rng.random_bool(0.1);
    attrs.basic = rng.random_bool(0.2);
    attrs.based = rng.random_bool(0.05);

    // Randomly assign resistances
    fn random_resist(rng: &mut impl rand::Rng) -> Resist {
        match rng.random_range(0..10) {
            0..=1 => Resist::Weak,
            2..=3 => Resist::Strong,
            _ => Resist::Normal,
        }
    }

    attrs.aura_resist = random_resist(rng);
    attrs.physical_resist = random_resist(rng);
    attrs.psychic_resist = random_resist(rng);
    attrs.boredom_resist = random_resist(rng);

    attrs
}

fn spawn_klarna_kop(
    turn_counter: Res<TurnCounter>,
    player: Single<(&Player, &MapPos)>,
    pos_to_creature: Res<map::PosToCreature>,
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    assets: Res<assets::WorldAssets>,
    map_info: Res<mapgen::MapInfo>,
    tiles: Query<
        &MapPos,
        (
            With<map::Tile>,
            Without<map::BlocksMovement>,
            Without<Frozen>,
        ),
    >,
) {
    let (player_stats, player_pos) = player.into_inner();
    let debt = -player_stats.money;
    if turn_counter.0 > 0 && debt >= 5 {
        let spawn_rate = if debt > 50 { 5 } else { 10 };
        if turn_counter.0.is_multiple_of(spawn_rate) {
            let mut rng = rand::rng();
            let valid_spots =
                get_valid_spawn_spots(player_pos.0, &tiles, &pos_to_creature, &map_info);

            if !valid_spots.is_empty()
                && let Some(pos) = valid_spots.choose(&mut rng)
            {
                let map_pos = *pos;
                let transform = Transform::from_translation(map_pos.to_vec3(PLAYER_Z));
                let level = (debt / 50) + 1;
                let bundle = mapgen::MobKind::KlarnaKop(level).get_bundle(&assets);

                let new_mob = commands.spawn((bundle, map_pos, transform)).id();
                commands.entity(world.into_inner()).add_child(new_mob);
            }
        }
    }
}

fn spawn_brainrot_enemies(
    turn_counter: Res<TurnCounter>,
    player: Single<(&Player, &MapPos)>,
    pos_to_creature: Res<map::PosToCreature>,
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    assets: Res<assets::WorldAssets>,
    map_info: Res<mapgen::MapInfo>,
    tiles: Query<
        &MapPos,
        (
            With<map::Tile>,
            Without<map::BlocksMovement>,
            Without<Frozen>,
        ),
    >,
) {
    let (player_stats, player_pos) = player.into_inner();
    let brainrot = player_stats.brainrot;
    if brainrot > 100 {
        let spawn_rate = if brainrot > 120 { 5 } else { 10 };
        if turn_counter.0 > 0 && turn_counter.0.is_multiple_of(spawn_rate) {
            let mut rng = rand::rng();
            let valid_spots =
                get_valid_spawn_spots(player_pos.0, &tiles, &pos_to_creature, &map_info);

            if !valid_spots.is_empty()
                && let Some(pos) = valid_spots.choose(&mut rng)
            {
                let map_pos = *pos;
                let transform = Transform::from_translation(map_pos.to_vec3(PLAYER_Z));
                let mut bundle = mapgen::MobKind::BrainrotEnemy.get_bundle(&assets);

                // Randomly assigned stats with reduced ranges
                use rand::Rng;
                let hp = rng.random_range(3..11);
                bundle.creature.hp = hp;
                bundle.creature.max_hp = hp;
                bundle.mob.melee_damage = rng.random_range(1..6);
                bundle.mob.attrs = get_random_brainrot_attrs(&mut rng);

                let entity_cmds = commands.spawn((
                    bundle,
                    map_pos,
                    transform,
                    BrainrotEnemyMarker,
                    assets.get_brainrot_sprite(),
                ));
                let new_mob = entity_cmds.id();
                commands.entity(world.into_inner()).add_child(new_mob);
            }
        }
    }
}

fn transform_brainrot_enemies(
    mut commands: Commands,
    player: Single<&Player>,
    assets: Res<assets::WorldAssets>,
    brainrot_enemies: Query<(Entity, &MapPos), (With<BrainrotEnemyMarker>, Without<Frozen>)>,
) {
    if player.brainrot < 100 && !brainrot_enemies.is_empty() {
        for (entity, _pos) in brainrot_enemies.iter() {
            let bundle = mapgen::MobKind::Capybara.get_bundle(&assets);
            // Replace everything but MapPos and Transform
            commands
                .entity(entity)
                .remove::<BrainrotEnemyMarker>()
                .insert(bundle);
        }
    }
}

fn transform_brainrot_corpses(
    mut commands: Commands,
    player: Single<&Player>,
    assets: Res<assets::WorldAssets>,
    brainrot_corpses: Query<Entity, (With<Ambrosia>, Without<Frozen>)>,
) {
    if player.brainrot < 100 && !brainrot_corpses.is_empty() {
        for entity in brainrot_corpses.iter() {
            let normie_bundle = mapgen::MobKind::Normie.get_bundle(&assets);
            let normie_corpse = normie_bundle.corpse;
            commands.entity(entity).remove::<Ambrosia>().insert((
                Corpse {
                    nutrition: normie_corpse.nutrition,
                    name: normie_corpse.name.clone(),
                    kind: normie_corpse.kind,
                },
                Name::new(normie_corpse.name),
                normie_corpse.sprite,
            ));
            // Normie corpses aren't directly edible, so remove Interactable
            commands.entity(entity).remove::<Interactable>();
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DamageType {
    Physical,
    Psychic,
    Aura,
    Boredom,
    #[allow(unused)]
    Hunger,
    #[allow(unused)]
    Strength,
}

impl DamageType {
    fn color(&self) -> Color {
        match self {
            DamageType::Physical => Color::srgb(1.0, 0.2, 0.2),
            DamageType::Psychic => Color::srgb(0.8, 0.2, 1.0),
            DamageType::Aura => Color::srgb(0.2, 0.8, 1.0),
            DamageType::Boredom => Color::srgb(0.6, 0.6, 0.6),
            DamageType::Hunger => Color::srgb(1.0, 0.5, 0.0),
            DamageType::Strength => Color::srgb(0.2, 1.0, 0.2),
        }
    }
}

impl std::fmt::Display for DamageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DamageType::Physical => "physical",
            DamageType::Psychic => "psychic",
            DamageType::Aura => "aura",
            DamageType::Boredom => "boredom",
            DamageType::Hunger => "hunger",
            DamageType::Strength => "strength",
        })
    }
}

pub struct DamageInstance {
    entity: Entity,
    attacker: Option<Entity>,
    amount: i32,
    ty: DamageType,
}

#[derive(Resource, Default)]
pub struct PendingDamage(Vec<DamageInstance>);

fn get_bullet_bundle(
    new_pos: MapPos,
    direction: IVec2,
    attacker: Entity,
    assets: &WorldAssets,
) -> impl Bundle {
    let rotation = match (direction.x, direction.y) {
        (0, 1) => 0.0,
        (-1, 1) => PI / 4.0,
        (-1, 0) => PI / 2.0,
        (-1, -1) => 3.0 * PI / 4.0,
        (0, -1) => PI,
        (1, -1) => 1.25 * PI,
        (1, 0) => 1.5 * PI,
        (1, 1) => 1.75 * PI,
        _ => panic!("Unexpected bullet direction: {direction:?}"),
    };
    let name = Name::new("Bullet");
    let bullet_sprite = assets.get_ascii_sprite('^', Color::WHITE);
    let transform = Transform::from_translation(new_pos.to_vec3(PLAYER_Z))
        .with_rotation(Quat::from_rotation_z(rotation));
    let bullet = Bullet {
        direction,
        damage: BULLET_DAMAGE,
        attacker,
    };
    (name, bullet, bullet_sprite, new_pos, transform, Frozen)
}

fn check_bullet_collision(
    mut commands: Commands,
    pos_to_mob: Res<PosToCreature>,
    sight_blocked_map: Res<map::SightBlockedMap>,
    bullets: Query<(Entity, &MapPos, &Bullet), Without<Frozen>>,
    mut damage: ResMut<PendingDamage>,
    player_q: Query<Entity, With<Player>>,
    mut screen_shake: ResMut<camera::ScreenShake>,
) {
    for (entity, pos, bullet) in bullets.iter() {
        if let Some(mob) = pos_to_mob.0.get(&pos.0) {
            damage.0.push(DamageInstance {
                entity: *mob,
                attacker: Some(bullet.attacker),
                amount: bullet.damage,
                ty: DamageType::Physical,
            });
            if player_q.get(*mob).is_ok() {
                screen_shake.trauma = (screen_shake.trauma + 0.6).min(1.0);
            }
        }
        if pos_to_mob.0.contains_key(&pos.0) || sight_blocked_map.0.contains(&pos.0) {
            commands.entity(entity).despawn();
        }
    }
}

fn apply_damage(
    mut damage: ResMut<PendingDamage>,
    mut animation: MessageWriter<DamageAnimationMessage>,
    mut creatures: Query<(
        &mut Creature,
        Option<&mut Player>,
        Option<&Mob>,
        &Transform,
        Option<&Name>,
    )>,
    player_q: Single<Entity, With<Player>>,
    dd_selection: Option<ResMut<DungeonDashState>>,
    mut floating_text: MessageWriter<crate::game::animation::FloatingTextMessage>,
    mut game_over_info: Option<ResMut<crate::screens::game_over::GameOverInfo>>,
    mut next_screen: ResMut<NextState<Screen>>,
    screen: Res<State<Screen>>,
) {
    let mut dd_selection = dd_selection;
    let player_entity = *player_q;
    for DamageInstance {
        entity,
        attacker,
        amount,
        ty,
    } in damage.0.drain(..)
    {
        if let Some(attacker) = attacker
            && attacker == player_entity
            && let Some(ref mut dd) = dd_selection
            && Some(entity) == dd.customer_entity
            && dd.active_job_turns.is_some()
        {
            dd.active_job_turns = None;
            dd.active_job_amount = None;
            dd.job_target = None;
            dd.failed_job_turns = Some(10);

            if let Ok((_, Some(mut player), _, _, _)) = creatures.get_mut(attacker) {
                player.money -= 10;
            }
            floating_text.write(crate::game::animation::FloatingTextMessage {
                entity: Some(attacker),
                world_pos: None,
                text: "Do not attack the customer! -$10".to_string(),
                color: Color::srgb(1.0, 0.0, 0.0),
                ..default()
            });
        }

        if let Ok((mut c, player, mob, transform, _name)) = creatures.get_mut(entity) {
            let world_pos = transform.translation;
            let is_player = player.is_some();
            let mut final_amount = amount;
            match player {
                Some(mut player) => match ty {
                    DamageType::Physical => {
                        c.hp -= amount;
                        if c.hp <= 0 && *screen.get() == Screen::Gameplay {
                            let info = game_over_info.as_mut().unwrap();
                            info.cause = crate::screens::game_over::DeathCause::LowHP;
                            info.brainrot = player.brainrot;
                            if let Some(attacker) = attacker
                                && let Ok((_, _, _, _, attacker_name)) = creatures.get(attacker)
                            {
                                info.killer_name = attacker_name.map(|n| n.as_str().to_string());
                            }
                            next_screen.set(Screen::GameOver);
                        }
                    }
                    DamageType::Psychic => {
                        player.brainrot += amount;
                    }
                    DamageType::Aura => {
                        player.rizz -= amount;
                        if player.rizz < 0 {
                            c.hp += player.rizz;
                            player.rizz = 0;
                        }
                        if c.hp <= 0 && *screen.get() == Screen::Gameplay {
                            let info = game_over_info.as_mut().unwrap();
                            info.cause = crate::screens::game_over::DeathCause::LowHP;
                            info.brainrot = player.brainrot;
                            if let Some(attacker) = attacker
                                && let Ok((_, _, _, _, attacker_name)) = creatures.get(attacker)
                            {
                                info.killer_name = attacker_name.map(|n| n.as_str().to_string());
                            }
                            next_screen.set(Screen::GameOver);
                        }
                    }
                    DamageType::Boredom => {
                        player.boredom += amount;
                        player.brainrot = (player.brainrot - amount).max(0);
                        if player.boredom > 100 {
                            c.hp -= player.boredom - 100;
                            player.boredom = 100;
                        }
                        if c.hp <= 0 && *screen.get() == Screen::Gameplay {
                            let info = game_over_info.as_mut().unwrap();
                            info.cause = crate::screens::game_over::DeathCause::Boredom;
                            info.brainrot = player.brainrot;
                            if let Some(attacker) = attacker
                                && let Ok((_, _, _, _, attacker_name)) = creatures.get(attacker)
                            {
                                info.killer_name = attacker_name.map(|n| n.as_str().to_string());
                            }
                            next_screen.set(Screen::GameOver);
                        }
                    }
                    DamageType::Hunger => {
                        player.apply_hunger_damage(&mut c, amount);
                        if c.hp <= 0 && *screen.get() == Screen::Gameplay {
                            let info = game_over_info.as_mut().unwrap();
                            info.cause = crate::screens::game_over::DeathCause::Other;
                            info.brainrot = player.brainrot;
                            next_screen.set(Screen::GameOver);
                        }
                    }
                    DamageType::Strength => {
                        player.apply_strength_damage(&mut c, amount);
                        if c.hp <= 0 && *screen.get() == Screen::Gameplay {
                            let info = game_over_info.as_mut().unwrap();
                            info.cause = crate::screens::game_over::DeathCause::Other;
                            info.brainrot = player.brainrot;
                            next_screen.set(Screen::GameOver);
                        }
                    }
                },
                None => {
                    let resist = mob
                        .map(|m| m.get_damage_resist(ty))
                        .unwrap_or(Resist::Normal);
                    final_amount = match resist {
                        Resist::Weak => amount * 2,
                        Resist::Normal => amount,
                        Resist::Strong => amount / 2,
                    };
                    c.hp -= final_amount;
                    if let Some(attacker) = attacker
                        && attacker == player_entity
                    {
                        c.killed_by_player = true;
                    }
                }
            }
            animation.write(DamageAnimationMessage {
                entity,
                amount: final_amount,
                ty,
                world_pos,
                is_player,
            });
        }
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

fn move_bullets(
    mut commands: Commands,
    mut bullets: Query<(Entity, &mut MapPos, &Bullet), Without<Frozen>>,
) {
    for (entity, mut pos, bullet) in bullets.iter_mut() {
        let old_pos = *pos;
        pos.0 += bullet.direction;

        let rotation = match (bullet.direction.x, bullet.direction.y) {
            (0, 1) => 0.0,
            (-1, 1) => PI / 4.0,
            (-1, 0) => PI / 2.0,
            (-1, -1) => 3.0 * PI / 4.0,
            (0, -1) => PI,
            (1, -1) => 1.25 * PI,
            (1, 0) => 1.5 * PI,
            (1, 1) => 1.75 * PI,
            _ => 0.0,
        };

        commands.entity(entity).insert(MoveAnimation {
            from: old_pos.to_vec3(PLAYER_Z),
            to: pos.to_vec3(PLAYER_Z),
            timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

            ease: EaseFunction::Linear,
            base_rotation: Some(rotation),
            rotation: None,
            sway: None,
        });
    }
}

#[derive(Resource, Default)]
pub(crate) struct FactionMap {
    dijkstra_map_per_faction: HashMap<i32, HashMap<MapPos, usize>>,
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
    creatures: Query<(Entity, &mut MapPos, &mut Creature), Without<Frozen>>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
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
    let mut dijkstra_map_per_faction = HashMap::<i32, HashMap<MapPos, usize>>::new();
    for (faction, friendly_positions) in positions_per_faction.iter() {
        let _span = info_span!("building faction map", faction = &faction).entered();
        if *faction == 0 {
            // Player doesn't need a dijkstra map to find targets.
            continue;
        }
        let enemy_positions = positions_per_faction
            .iter()
            .filter(|(f, _positions)| is_enemy(*faction, **f))
            .flat_map(|(_f, positions)| positions)
            .copied()
            .map(MapPos)
            .collect::<Vec<_>>();
        let reachable_cb = |p| reachable(p, &walk_blocked_map, friendly_positions);
        let maxdist = 100;
        dijkstra_map_per_faction.insert(
            *faction,
            rogue_algebra::path::build_dijkstra_map(&enemy_positions, maxdist, reachable_cb)
                .collect(),
        );
    }
    faction_map.dijkstra_map_per_faction = dijkstra_map_per_faction;
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Move(MapPos),
    Melee(Entity, MapPos),
    RangedAttack(MapPos),
    AttackAndTeleport(Entity, MapPos),
}

fn apply_friend_of_machines(
    player: Single<&Creature, With<Player>>,
    creatures: Query<&mut Creature, Without<Player>>,
) {
    let player = player.into_inner();
    if player.friend_of_machines {
        for mut creature in creatures {
            if creature.machine {
                creature.faction = ALLIED_FACTION;
            }
        }
    }
}

fn process_mob_turn(
    world: Single<Entity, With<GameWorld>>,
    assets: Res<WorldAssets>,
    pos_to_creature: Res<PosToCreature>,
    faction_map: Res<FactionMap>,
    mut mobs: Query<(Entity, &mut MapPos, &Creature, &mut Mob), (Without<Player>, Without<Frozen>)>,
    mut commands: Commands,
    mut damage: ResMut<PendingDamage>,
    player_q: Single<(Entity, &MapPos, &Creature, &mut Player), With<Player>>,
    mut screen_shake: ResMut<camera::ScreenShake>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
    sight_blocked_map: Res<map::SightBlockedMap>,
    map_info: Res<mapgen::MapInfo>,
    all_creatures: Query<&Creature>,
    mut floating_text: MessageWriter<FloatingTextMessage>,
    turn_counter: Res<TurnCounter>,
) {
    let world_entity = world.into_inner();
    let rng = &mut rand::rng();
    let (player_entity, player_pos, player_creature, mut player) = player_q.into_inner();
    player.is_raided = false;

    // Determine mob intentions.
    let mut mob_moves = HashMap::new();
    let mut claimed_locations = HashSet::new();
    for (entity, pos, creature, mut mob) in mobs.iter_mut() {
        if creature.is_dead() {
            continue;
        }

        if mob.attrs.knows_player_location && is_enemy(creature.faction, player_creature.faction) {
            mob.target = Some(player_pos.0);
        } else if let Some(target) = get_fov_target(
            *pos,
            creature,
            &sight_blocked_map,
            &pos_to_creature,
            &all_creatures,
        ) {
            mob.target = Some(target);
        } else if let Some(t) = mob.target
            && pos.0 == t
        {
            mob.target = None;
        }

        let sees_player = mob.target == Some(player_pos.0);

        if mob.attrs.raids_player && sees_player {
            player.is_raided = true;
        }

        if let Some(Summon { kind, delay }) = mob.attrs.summon
            && turn_counter.0 > 0
            && turn_counter.0.is_multiple_of(delay)
            && sees_player
        {
            let spawn_pos = pos.adjacent().into_iter().find(|p| {
                !walk_blocked_map.0.contains(&p.0)
                    && !pos_to_creature.0.contains_key(&p.0)
                    && !claimed_locations.contains(&p.0)
            });
            if let Some(spawn_pos) = spawn_pos {
                claimed_locations.insert(spawn_pos.0);
                spawn::spawn_mob(&mut commands, world_entity, spawn_pos, kind, &assets);
                let name = kind.get_bundle(&assets).name;
                floating_text.write(FloatingTextMessage {
                    entity: Some(entity),
                    world_pos: None,
                    text: format!("Summoned {name}!"),
                    color: Color::srgb(0.7, 0.4, 0.9),
                    ..default()
                });
            }
        }

        if mob.target.is_none() {
            if (creature.faction == FRIENDLY_FACTION || creature.faction == ALLIED_FACTION)
                && let Some(action) = get_crew_move(
                    *pos,
                    &mut mob,
                    rng,
                    &map_info,
                    &walk_blocked_map,
                    &pos_to_creature,
                    &claimed_locations,
                )
                && let Action::Move(target) = action
            {
                claimed_locations.insert(target.0);
                mob_moves.insert(entity, action);
            }
            continue;
        }

        // follow dijkstra map
        let Some(dijkstra_map) = faction_map.dijkstra_map_per_faction.get(&creature.faction) else {
            continue;
        };

        let target = pos
            .adjacent()
            .into_iter()
            .filter(|p| dijkstra_map.contains_key(p))
            .filter(|p| !claimed_locations.contains(&p.0))
            .min_by_key(|p| dijkstra_map.get(p).cloned().unwrap_or(usize::MAX));

        if let Some(target) = target {
            let action = if mob.ranged && rng.random_bool(0.5) {
                Action::RangedAttack(target)
            } else if mob.keepaway
                && (2..=3).contains(&pos.0.chebyshev_distance(target.0))
                && creature.hp >= creature.max_hp
            {
                // keepaway mobs don't close in the final 2 steps unless they are
                // damaged
                continue;
            } else if let Some(occupier) = pos_to_creature.0.get(&target.0) {
                if mob.attrs.sus {
                    get_teleport_action(
                        target,
                        *occupier,
                        &walk_blocked_map,
                        &pos_to_creature,
                        &claimed_locations,
                    )
                } else {
                    Action::Melee(*occupier, target)
                }
            } else {
                Action::Move(target)
            };

            mob_moves.insert(entity, action);
            if let Action::Move(t) | Action::AttackAndTeleport(_, t) = action {
                claimed_locations.insert(t.0);
            } else if dijkstra_map.get(&target) != Some(&1) {
                // Claim any move that is not a destination.
                // This works because destinations are always enemies.
                claimed_locations.insert(target.0);
            }
        }
    }

    // Apply moves.
    for (entity, action) in mob_moves.into_iter() {
        let (entity, mut pos, _creature, mob) = mobs.get_mut(entity).unwrap();
        let old_pos = *pos;
        match action {
            Action::Move(new_pos) => {
                *pos = new_pos;
                commands.entity(entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: new_pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(100), TimerMode::Once),

                    ease: EaseFunction::SineInOut,
                    base_rotation: None,
                    rotation: None,
                    sway: None,
                });
            }
            Action::Melee(enemy, new_pos) => {
                damage.0.push(DamageInstance {
                    entity: enemy,
                    attacker: Some(entity),
                    amount: mob.melee_damage,
                    ty: mob.get_melee_damage_type(),
                });
                commands.entity(entity).insert(animation::AttackAnimation {
                    direction: (new_pos.0 - old_pos.0).as_vec2(),
                    timer: Timer::new(Duration::from_millis(150), TimerMode::Once),
                    base_translation: old_pos.to_vec3(PLAYER_Z),
                });
                if enemy == player_entity {
                    screen_shake.trauma = (screen_shake.trauma + 0.7).min(1.0);
                }
            }
            Action::RangedAttack(new_pos) => {
                let direction = new_pos.0 - old_pos.0;
                commands.entity(world_entity).with_children(|parent| {
                    parent.spawn(get_bullet_bundle(new_pos, direction, entity, &assets));
                });
            }
            Action::AttackAndTeleport(enemy, teleport_pos) => {
                damage.0.push(DamageInstance {
                    entity: enemy,
                    attacker: Some(entity),
                    amount: mob.melee_damage,
                    ty: mob.get_melee_damage_type(),
                });
                *pos = teleport_pos;
                commands.entity(entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: teleport_pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(1), TimerMode::Once),

                    ease: EaseFunction::Linear,
                    base_rotation: None,
                    rotation: None,
                    sway: None,
                });
                if enemy == player_entity {
                    screen_shake.trauma = (screen_shake.trauma + 0.7).min(1.0);
                }
                floating_text.write(FloatingTextMessage {
                    entity: Some(entity),
                    world_pos: None,
                    text: "Teleported!".to_string(),
                    color: Color::srgb(1.0, 0.0, 1.0),
                    ..default()
                });
            }
        }
    }
}

fn prune_dead(
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    q_creatures: Query<
        (Entity, &Creature, &MapPos, Option<&DropsCorpse>, &Name),
        (Without<Player>, Without<Frozen>),
    >,
    mut player: Single<&mut Player>,
    mut streaming_state: ResMut<chat::StreamingState>,
    mut chat: ResMut<chat::ChatHistory>,
) {
    let world_entity = world.into_inner();
    let player = player.as_mut();
    for (entity, creature, map_pos, corpse, name) in q_creatures {
        if creature.is_dead() {
            commands.entity(entity).despawn();

            if creature.killed_by_player {
                chat::handle_payout(player, &streaming_state, &mut chat, name);
                if let Some(corpse) = corpse
                    && corpse.kind == mapgen::MobKind::Streamer
                {
                    use rand::Rng;
                    let mut rng = rand::rng();
                    let bonus = rng.random_range(50..=100);
                    streaming_state.subscribers += bonus;
                }
            }

            if let Some(DropsCorpse {
                sprite,
                nutrition,
                name,
                kind,
            }) = corpse
            {
                let transform = Transform::from_translation(map_pos.to_vec3(CORPSE_Z));
                let mut entity_cmds = commands.spawn((
                    Name::new("Corpse"),
                    Corpse {
                        nutrition: *nutrition,
                        name: name.clone(),
                        kind: *kind,
                    },
                    DespawnAfterTurns(50),
                    sprite.clone(),
                    *map_pos,
                    transform,
                ));

                if *kind == mapgen::MobKind::BrainrotEnemy {
                    entity_cmds.insert((
                        Ambrosia,
                        Name::new("Ambrosia"),
                        Interactable {
                            action: "Eat".to_string(),
                            description: Some("Directly edible brainrot essence.".to_string()),
                            kind: InteractionType::Eat,
                            require_on_top: false,
                        },
                    ));
                }

                let corpse_id = entity_cmds.id();
                commands.entity(world_entity).add_child(corpse_id);
            }
        }
    }
}

#[derive(Default, Resource)]
pub(crate) struct NearbyMobs {
    mobs: Vec<(
        MapPos,
        Creature,
        Option<Mob>,
        Option<String>,
        Option<Color>,
        Option<Sprite>,
        Name,
    )>,
}

fn get_fov_target(
    pos: MapPos,
    creature: &Creature,
    sight_blocked_map: &map::SightBlockedMap,
    pos_to_creature: &PosToCreature,
    all_creatures: &Query<&Creature>,
) -> Option<IVec2> {
    let fov = rogue_algebra::fov::calculate_fov(pos.0.into(), 10, |p| {
        sight_blocked_map.contains(&IVec2::from(p))
    });

    for visible_pos in fov {
        let visible_ivec = IVec2::from(visible_pos);
        if let Some(other_entity) = pos_to_creature.0.get(&visible_ivec)
            && let Ok(other_creature) = all_creatures.get(*other_entity)
            && is_enemy(creature.faction, other_creature.faction)
        {
            return Some(visible_ivec);
        }
    }
    None
}

fn get_teleport_action(
    pos: MapPos,
    occupier: Entity,
    walk_blocked_map: &map::WalkBlockedMap,
    pos_to_creature: &PosToCreature,
    claimed_locations: &HashSet<IVec2>,
) -> Action {
    let teleport_target = rogue_algebra::path::bfs_paths(&[pos.0], 25, |p| {
        let map_pos = MapPos(p);
        map_pos
            .adjacent()
            .into_iter()
            .filter(|p| !walk_blocked_map.contains(&p.0))
            .map(|p| p.0)
    })
    .filter(|path| {
        let pos = path.last().unwrap();
        !pos_to_creature.0.contains_key(pos) && !claimed_locations.contains(pos)
    })
    .max_by_key(|path| path.len())
    .map(|path| MapPos(*path.last().unwrap()));

    if let Some(teleport_target) = teleport_target {
        Action::AttackAndTeleport(occupier, teleport_target)
    } else {
        Action::Melee(occupier, pos)
    }
}

fn select_random_move(
    pos: MapPos,
    rng: &mut impl rand::Rng,
    walk_blocked_map: &map::WalkBlockedMap,
    pos_to_creature: &PosToCreature,
    claimed_locations: &HashSet<IVec2>,
) -> Option<Action> {
    let mut adjacent = pos.adjacent();
    adjacent.shuffle(rng);
    adjacent
        .into_iter()
        .find(|p| {
            !walk_blocked_map.contains(&p.0)
                && !pos_to_creature.0.contains_key(&p.0)
                && !claimed_locations.contains(&p.0)
        })
        .map(Action::Move)
}

fn get_crew_move(
    pos: MapPos,
    mob: &mut Mob,
    rng: &mut impl rand::Rng,
    map_info: &mapgen::MapInfo,
    walk_blocked_map: &map::WalkBlockedMap,
    pos_to_creature: &PosToCreature,
    claimed_locations: &HashSet<IVec2>,
) -> Option<Action> {
    if (mob.destination.is_none() || mob.destination == Some(pos.0))
        && let Some(level) = map_info.get_level(pos)
        && !level.destinations.is_empty()
    {
        mob.destination = Some(IVec2::from(*level.destinations.choose(rng).unwrap()));
    }

    if let Some(destination) = mob.destination {
        let path = rogue_algebra::path::bfs_paths(&[pos.0], 50, |p| {
            let map_pos = MapPos(p);
            map_pos.adjacent().into_iter().map(|p| p.0).filter(|&p| {
                !walk_blocked_map.contains(&p)
                    && (p == destination || !pos_to_creature.0.contains_key(&p))
                    && !claimed_locations.contains(&p)
            })
        })
        .find(|path| path.last() == Some(&destination));

        if let Some(path) = path
            && path.len() > 1
        {
            return Some(Action::Move(MapPos(path[1])));
        }
    }

    select_random_move(
        pos,
        rng,
        walk_blocked_map,
        pos_to_creature,
        claimed_locations,
    )
}

fn update_nearby_mobs(
    mut nearby_mobs: ResMut<NearbyMobs>,
    player: Query<&MapPos, With<Player>>,
    mobs: Query<
        (
            &MapPos,
            &Creature,
            Option<&Mob>,
            Option<&Text2d>,
            Option<&TextColor>,
            Option<&Sprite>,
            &Name,
        ),
        (Without<Player>, Without<Frozen>),
    >,
    pos_to_creature: Res<PosToCreature>,
    player_vis_map: Res<map::PlayerVisibilityMap>,
) {
    nearby_mobs.mobs.clear();
    let player_pos = *player.single().unwrap();
    let maxdist = 10;
    let reachable = |p: MapPos| p.adjacent();
    for path in rogue_algebra::path::bfs_paths(&[player_pos], maxdist, reachable) {
        if let Some(pos) = path.last()
            && player_vis_map.contains(&pos.0)
            && let Some(mob_entity) = pos_to_creature.0.get(&pos.0)
            && let Ok((pos, creature, mob, text, color, sprite, name)) = mobs.get(*mob_entity)
        {
            nearby_mobs.mobs.push((
                *pos,
                creature.clone(),
                mob.cloned(),
                text.map(|t| t.0.clone()),
                color.map(|c| c.0),
                sprite.cloned(),
                name.clone(),
            ));
        }
    }
}

fn draw_meter(ui: &mut egui::Ui, ratio: f32, text: String, color: egui::Color32, brainrot: i32) {
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
    let text_job = apply_brainrot_ui(
        text,
        brainrot,
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

fn sidebar(
    mut contexts: EguiContexts,
    player: Single<(&Player, &MapPos)>,
    nearby_mobs: Res<NearbyMobs>,
    examine_results: Res<examine::ExamineResults>,
    world_assets: If<Res<WorldAssets>>,
    atlas_assets: If<Res<Assets<TextureAtlasLayout>>>,
    mut input_mode: ResMut<InputMode>,
    mut examine_pos: ResMut<examine::ExaminePos>,
    mut msg_ability_clicked: MessageWriter<AbilityClicked>,
    mut msg_wait: MessageWriter<WaitMessage>,
    mut phone_state: ResMut<phone::PhoneState>,
) {
    let (player, player_pos) = player.into_inner();
    let sword = world_assets
        .get_urizen_egui_image(&mut contexts, &atlas_assets, 1262)
        .fit_to_exact_size(egui::vec2(TILE_WIDTH, TILE_HEIGHT));

    let mut mob_images = Vec::new();
    for (_, _, _, _, _, sprite, _) in nearby_mobs.mobs.iter() {
        if let Some(sprite) = sprite {
            mob_images.push(Some(get_egui_image_from_sprite(
                &mut contexts,
                &atlas_assets,
                sprite,
            )));
        } else {
            mob_images.push(None);
        }
    }

    let ctx = contexts.ctx_mut().unwrap();
    egui::SidePanel::right("sidebar")
        .min_width(TILE_WIDTH * 8.0)
        .show(ctx, |ui| {
            let active_examine_pos = examine_results.info.as_ref().map(|i| i.pos);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.group(|ui| {
                    ui.set_min_height(400.0);

                    for (i, (pos, creature, mob, text, color, _sprite, name)) in
                        nearby_mobs.mobs.iter().enumerate()
                    {
                        let highlight = Some(*pos) == active_examine_pos;
                        let mut frame = egui::Frame::new().inner_margin(Margin::same(4));
                        if highlight {
                            frame = frame.fill(ui.style().visuals.code_bg_color);
                        }
                        frame.show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    if let Some(text) = text
                                        && let Some(color) = color
                                    {
                                        let [r, g, b, a] = color.to_srgba().to_u8_array();
                                        let c32 = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
                                        ui.add(
                                            egui::Label::new(apply_brainrot_ui(
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
                                            ))
                                            .selectable(false),
                                        );
                                    } else if let Some(sprite_img) = &mob_images[i] {
                                        ui.add(sprite_img.clone().fit_to_exact_size(egui::vec2(
                                            TILE_WIDTH,
                                            TILE_HEIGHT,
                                        )));
                                    }
                                    if !name.as_str().is_empty() {
                                        ui.add(
                                            egui::Label::new(apply_brainrot_ui(
                                                RichText::new(name.as_str()).strong(),
                                                player.brainrot,
                                                ui.style(),
                                                FontSelection::Default,
                                                Align::LEFT,
                                            ))
                                            .selectable(false),
                                        );
                                    }
                                });
                                let ratio = creature.hp as f32 / creature.max_hp as f32;
                                ui.horizontal(|ui| {
                                    draw_meter(
                                        ui,
                                        ratio,
                                        format!("{}/{}", creature.hp, creature.max_hp),
                                        egui::Color32::from_rgb(65, 148, 109),
                                        player.brainrot,
                                    );
                                });
                                ui.horizontal_wrapped(|ui| {
                                    if let Some(mob) = mob {
                                        ui.add(sword.clone());
                                        ui.label(mob.melee_damage.to_string());
                                        for (attr, name, color, tooltip) in [
                                            (mob.ranged, "Gun", UI_RED, "oh shit he got a gun"),
                                            (
                                                mob.attrs.based,
                                                "Based",
                                                egui::Color32::PURPLE,
                                                "Deals psychic damage",
                                            ),
                                            (
                                                mob.attrs.basic,
                                                "Basic",
                                                egui::Color32::DARK_GRAY,
                                                "Deals boredom damage",
                                            ),
                                            (
                                                mob.attrs.mog_risk,
                                                "Mog Risk",
                                                egui::Color32::DARK_GREEN,
                                                "Deals aura damage",
                                            ),
                                            (
                                                mob.attrs.sus,
                                                "Sus",
                                                UI_RED,
                                                "Can teleport when attacking",
                                            ),
                                            (
                                                creature.faction == ALLIED_FACTION,
                                                "Allied",
                                                egui::Color32::LIGHT_BLUE,
                                                "Will fight for you",
                                            ),
                                            (
                                                creature.faction == FRIENDLY_FACTION,
                                                "Friendly",
                                                egui::Color32::LIGHT_BLUE,
                                                "Neutral and peaceful",
                                            ),
                                            (
                                                creature.machine,
                                                "Machine",
                                                egui::Color32::LIGHT_GRAY,
                                                "It's a robot",
                                            ),
                                        ] {
                                            if attr {
                                                ui.add(
                                                    egui::Label::new(apply_brainrot_ui(
                                                        RichText::new(name).background_color(color),
                                                        player.brainrot,
                                                        ui.style(),
                                                        FontSelection::Default,
                                                        Align::LEFT,
                                                    ))
                                                    .selectable(false),
                                                )
                                                .on_hover_text(tooltip);
                                                if highlight {
                                                    ui.add(
                                                        egui::Label::new(apply_brainrot_ui(
                                                            tooltip,
                                                            player.brainrot,
                                                            ui.style(),
                                                            FontSelection::Default,
                                                            Align::LEFT,
                                                        ))
                                                        .selectable(false),
                                                    );
                                                }
                                            }
                                        }
                                        fn resist_name(
                                            ty: DamageType,
                                            resist: Resist,
                                        ) -> Option<(&'static str, egui::Color32, &'static str)>
                                        {
                                            match (ty, resist) {
                                                (_, Resist::Normal) => None,
                                                (DamageType::Physical, Resist::Weak) => Some((
                                                    "Weak",
                                                    egui::Color32::DARK_RED,
                                                    "Takes double physical damage",
                                                )),
                                                (DamageType::Physical, Resist::Strong) => Some((
                                                    "Unit",
                                                    egui::Color32::DARK_BLUE,
                                                    "Takes half physical damage",
                                                )),
                                                (DamageType::Psychic, Resist::Weak) => Some((
                                                    "Cooked",
                                                    egui::Color32::DARK_RED,
                                                    "Takes double psychic damage",
                                                )),
                                                (DamageType::Psychic, Resist::Strong) => Some((
                                                    "Locked in",
                                                    egui::Color32::DARK_BLUE,
                                                    "Takes half psychic damage",
                                                )),
                                                (DamageType::Aura, Resist::Weak) => Some((
                                                    "Cringe",
                                                    egui::Color32::DARK_RED,
                                                    "Takes double aura damage",
                                                )),
                                                (DamageType::Aura, Resist::Strong) => Some((
                                                    "Snatched",
                                                    egui::Color32::DARK_BLUE,
                                                    "Takes half aura damage",
                                                )),
                                                (DamageType::Boredom, Resist::Weak) => Some((
                                                    "NPC",
                                                    egui::Color32::DARK_RED,
                                                    "Takes double boredom damage",
                                                )),
                                                (DamageType::Boredom, Resist::Strong) => Some((
                                                    "Focused",
                                                    egui::Color32::DARK_BLUE,
                                                    "Takes half boredom damage",
                                                )),
                                                _ => None,
                                            }
                                        }
                                        for damage_type in [
                                            DamageType::Physical,
                                            DamageType::Psychic,
                                            DamageType::Aura,
                                            DamageType::Boredom,
                                        ] {
                                            if let Some((name, color, tooltip)) = resist_name(
                                                damage_type,
                                                mob.get_damage_resist(damage_type),
                                            ) {
                                                ui.add(
                                                    egui::Label::new(apply_brainrot_ui(
                                                        RichText::new(name).background_color(color),
                                                        player.brainrot,
                                                        ui.style(),
                                                        FontSelection::Default,
                                                        Align::LEFT,
                                                    ))
                                                    .selectable(false),
                                                )
                                                .on_hover_text(tooltip);
                                                if highlight {
                                                    ui.add(
                                                        egui::Label::new(apply_brainrot_ui(
                                                            tooltip,
                                                            player.brainrot,
                                                            ui.style(),
                                                            FontSelection::Default,
                                                            Align::LEFT,
                                                        ))
                                                        .selectable(false),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                });
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
                            if ui
                                .button(apply_brainrot_ui(
                                    "examine: x",
                                    player.brainrot,
                                    ui.style(),
                                    FontSelection::Default,
                                    Align::LEFT,
                                ))
                                .clicked()
                            {
                                *input_mode = InputMode::Examine(player_pos.0);
                                examine_pos.pos = Some(*player_pos);
                            }
                            if ui
                                .button(apply_brainrot_ui(
                                    "phone: space",
                                    player.brainrot,
                                    ui.style(),
                                    FontSelection::Default,
                                    Align::LEFT,
                                ))
                                .clicked()
                                && !phone_state.forced_open
                            {
                                phone_state.is_open = !phone_state.is_open;
                            }
                            if ui
                                .button(apply_brainrot_ui(
                                    "wait: .",
                                    player.brainrot,
                                    ui.style(),
                                    FontSelection::Default,
                                    Align::LEFT,
                                ))
                                .clicked()
                            {
                                msg_wait.write(WaitMessage);
                            }
                            ui.label(
                                RichText::new(format!(
                                    "melee damage: {}-{} {}",
                                    player.melee_damage(),
                                    player.melee_damage(),
                                    DamageType::Physical,
                                ))
                                .color(bevy_to_egui_color(DamageType::Physical.color())),
                            );

                            for (i, ability) in player.abilities.iter().enumerate() {
                                let ability_key = (i + 1) % 10;
                                let label = if matches!(ability, Ability::Sprint) {
                                    format!("{ability_key}/Shift: {ability}")
                                } else {
                                    format!("{ability_key}: {ability}")
                                };
                                let cooldown = player.ability_cooldowns.get(ability).unwrap_or(&0);
                                let label = if *cooldown > 0 {
                                    format!("{label} ({cooldown})")
                                } else {
                                    label
                                };
                                ui.horizontal(|ui| {
                                    let button = egui::Button::new(apply_brainrot_ui(
                                        label,
                                        player.brainrot,
                                        ui.style(),
                                        FontSelection::Default,
                                        Align::LEFT,
                                    ));
                                    if ui.add_enabled(*cooldown == 0, button).clicked() {
                                        msg_ability_clicked.write(AbilityClicked(*ability));
                                    }
                                    if let Some((ty, range)) = ability.damage_info(player) {
                                        ui.label(
                                            RichText::new(format!(
                                                "{}-{} {} damage",
                                                range.start(),
                                                range.end(),
                                                ty,
                                            ))
                                            .color(bevy_to_egui_color(ty.color())),
                                        );
                                    }
                                });
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
                            if ui
                                .button(apply_brainrot_ui(
                                    "exit: x",
                                    player.brainrot,
                                    ui.style(),
                                    FontSelection::Default,
                                    Align::LEFT,
                                ))
                                .clicked()
                            {
                                *input_mode = InputMode::Normal;
                                examine_pos.pos = None;
                            }
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
                            if ui
                                .button(apply_brainrot_ui(
                                    "exit: x",
                                    player.brainrot,
                                    ui.style(),
                                    FontSelection::Default,
                                    Align::LEFT,
                                ))
                                .clicked()
                            {
                                *input_mode = InputMode::Normal;
                                examine_pos.pos = None;
                            }
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
        });
}

fn stat_label(ui: &mut egui::Ui, name: &str, brainrot: i32, is_bad: bool, time: f32) {
    let base_size = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .map(|f| f.size)
        .unwrap_or(14.0);

    if !is_bad {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(5.0);
            ui.label(apply_brainrot_ui(
                RichText::new(name).size(base_size),
                brainrot,
                ui.style(),
                FontSelection::Default,
                Align::LEFT,
            ));
        });
        return;
    }

    ui.horizontal(|ui| {
        animation::jumping_text(ui, name, brainrot, time, base_size, None);
    });
}

fn left_sidebar(
    mut contexts: EguiContexts,
    player: Single<(&Creature, &Player)>,
    time: Res<Time>,
    streaming_state: Res<crate::game::chat::StreamingState>,
    turn_counter: Res<TurnCounter>,
) {
    let (creature, player_stats) = player.into_inner();
    let ctx = contexts.ctx_mut().unwrap();
    egui::SidePanel::left("left_sidebar")
        .min_width(250.0)
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
                let color = if !invert_colors {
                    if ratio > 0.5 {
                        UI_GREEN
                    } else if ratio > 0.25 {
                        UI_YELLOW
                    } else {
                        UI_RED
                    }
                } else {
                    // High is bad
                    if ratio < 0.5 {
                        UI_GREEN
                    } else if ratio < 0.75 {
                        UI_YELLOW
                    } else {
                        UI_RED
                    }
                };

                stat_label(ui, name, player_stats.brainrot, is_bad, time.elapsed_secs());
                let meter_text = format!("{}/{}", value, max);
                draw_meter(ui, ratio, meter_text, color, player_stats.brainrot);
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
                    match signal_val {
                        1 | 2 => UI_RED,
                        3 => UI_YELLOW,
                        4 | 5 => UI_GREEN,
                        _ => UI_RED,
                    }
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
                format!("Subscribers: {}", streaming_state.subscribers),
                player_stats.brainrot,
                ui.style(),
                FontSelection::Default,
                Align::LEFT,
            ));

            if streaming_state.is_streaming {
                ui.label(apply_brainrot_ui(
                    format!("Viewers: {}", streaming_state.viewers_displayed as i32),
                    player_stats.brainrot,
                    ui.style(),
                    FontSelection::Default,
                    Align::LEFT,
                ));
            }

            if !player_stats.subscriptions.is_empty() {
                ui.add_space(20.0);
                ui.label(apply_brainrot_ui(
                    RichText::new("Subscriptions").size(18.0).strong(),
                    player_stats.brainrot,
                    ui.style(),
                    FontSelection::Default,
                    Align::LEFT,
                ));
                let next_payment = 100 - (turn_counter.0 % 100);
                let mut next_payment_text =
                    RichText::new(format!("Next billing in {} turns", next_payment))
                        .italics()
                        .size(12.0);
                if next_payment <= 5 {
                    next_payment_text = next_payment_text.color(egui::Color32::RED);
                }

                ui.label(apply_brainrot_ui(
                    next_payment_text,
                    player_stats.brainrot,
                    ui.style(),
                    FontSelection::Default,
                    Align::LEFT,
                ));
                ui.add_space(5.0);
                for sub in &player_stats.subscriptions {
                    ui.label(apply_brainrot_ui(
                        RichText::new(format!("{}: ${}/100 turns", sub.name(), sub.cost()))
                            .size(12.0),
                        player_stats.brainrot,
                        ui.style(),
                        FontSelection::Default,
                        Align::LEFT,
                    ));
                }
            }
            // non-subscription non-active upgrades
            let passive_upgrades = player_stats
                .upgrades
                .iter()
                .map(|u| &UPGRADES[*u])
                .filter(|upgrade| {
                    upgrade
                        .effects
                        .iter()
                        .all(|eff| !matches!(eff, Effect::GainAbility(_) | Effect::Subscription(_)))
                })
                .collect::<Vec<_>>();
            if !passive_upgrades.is_empty() {
                ui.add_space(20.0);
                ui.label("PASSIVE UPGRADES:");
                ui.add_space(5.0);
                for upgrade in passive_upgrades.into_iter() {
                    ui.label(upgrade.name);
                }
            }
        });
}

#[derive(Resource, Default)]
pub struct LastTitleDropLevel(pub Option<rogue_algebra::Rect>);

fn update_level_info_on_change(
    mut commands: Commands,
    mut msg_td: MessageWriter<TitleDropMessage>,
    map_info: Res<MapInfo>,
    mut last_level: ResMut<LastTitleDropLevel>,
    player: Single<(&MapPos, &mut Player), (With<Player>, Changed<MapPos>)>,
    all_map_pos: Query<(Entity, &MapPos, Option<&Mob>), Without<Player>>,
    all_spawn_zones: Query<(Entity, &MinSpawnZone)>,
    mut dd_selection: ResMut<DungeonDashState>,
) {
    let (pos, mut player_stats) = player.into_inner();
    if let Some(cur_map) = map_info.get_level(*pos)
        && Some(cur_map.rect) != last_level.0
    {
        last_level.0 = Some(cur_map.rect);
        msg_td.write(TitleDropMessage(format!("{}", cur_map.ty)));

        dd_selection.deliveries_this_level = 0;
        let mob_count = all_map_pos
            .iter()
            .filter(|(_, map_pos, mob)| {
                mob.is_some() && cur_map.rect.contains(rogue_algebra::Pos::from(map_pos.0))
            })
            .count() as u32;
        dd_selection.initial_mobs = mob_count;

        if let Some(target) = dd_selection.job_target
            && !cur_map.rect.contains(rogue_algebra::Pos::from(target.0))
        {
            dd_selection.job_target = None;
        }

        // Handle freezing/unfreezing
        for (entity, map_pos, _) in all_map_pos.iter() {
            if cur_map.rect.contains(rogue_algebra::Pos::from(map_pos.0)) {
                commands.entity(entity).remove::<Frozen>();
            } else {
                commands.entity(entity).insert(Frozen);
            }
        }
        for (entity, zone) in all_spawn_zones.iter() {
            if cur_map.rect.contains(zone.rect.center()) {
                commands.entity(entity).remove::<Frozen>();
            } else {
                commands.entity(entity).insert(Frozen);
            }
        }

        // Update max depth
        let new_depth = (pos.0.y / 200).max(0);
        if new_depth > player_stats.max_depth {
            player_stats.max_depth = new_depth;
            player_stats.pending_upgrades += 1;
        }
    }
}

#[derive(Component)]
#[require(InheritedVisibility, GlobalTransform, Transform)]
struct MinSpawnZone {
    rect: rogue_algebra::Rect,
    min_units: usize,
    distribution: &'static [(MobKind, usize)],
}

fn process_spawn_zones(
    mut commands: Commands,
    pos_to_mob: Res<PosToCreature>,
    q_zones: Query<(Entity, &MinSpawnZone), Without<Frozen>>,
    walk_blocked_map: Res<WalkBlockedMap>,
    player_vis_map: Res<PlayerVisibilityMap>,
    assets: Res<WorldAssets>,
) {
    let rng = &mut rand::rng();

    for (entity, zone) in q_zones.iter() {
        let num_mobs = zone
            .rect
            .into_iter()
            .filter(|p| pos_to_mob.0.contains_key(&IVec2::from(*p)))
            .count();
        let num_missing = zone.min_units.saturating_sub(num_mobs);
        if num_missing > 0 {
            let available_spots = zone
                .rect
                .into_iter()
                .filter(|p| !pos_to_mob.0.contains_key(&IVec2::from(*p)))
                .filter(|p| !walk_blocked_map.0.contains(&IVec2::from(*p)))
                .filter(|p| !player_vis_map.0.contains(&IVec2::from(*p)))
                .collect::<Vec<rogue_algebra::Pos>>();
            for p in available_spots.choose_multiple(rng, num_missing) {
                let mob_kind = zone
                    .distribution
                    .choose_weighted(rng, |(_k, w)| *w)
                    .unwrap()
                    .0;
                spawn_mob(
                    &mut commands,
                    entity,
                    MapPos(IVec2::from(*p)),
                    mob_kind,
                    &assets,
                );
            }
        }
    }
}

fn update_frozen(
    mut commands: Commands,
    player_pos: Single<&MapPos, With<Player>>,
    map_info: Res<MapInfo>,
    q_map_pos: Query<(Entity, &MapPos, Option<&Frozen>), Without<Player>>,
) {
    let cur_level = map_info.get_level(**player_pos);
    let mut new_frozen_batch = vec![];
    for (entity, pos, frozen) in q_map_pos {
        let should_be_frozen = cur_level
            .map(|l| !l.rect.contains(pos.0.into()))
            .unwrap_or(false);
        match (frozen.is_some(), should_be_frozen) {
            (false, true) => {
                new_frozen_batch.push((entity, Frozen));
            }
            (true, false) => {
                commands.entity(entity).remove::<Frozen>();
            }
            _ => {}
        }
    }
    commands.insert_batch(new_frozen_batch);
}

#[derive(SystemParam)]
pub struct GameResetParams<'w> {
    pub phone: ResMut<'w, phone::PhoneState>,
    pub map_info: ResMut<'w, mapgen::MapInfo>,
    pub streaming_state: ResMut<'w, chat::StreamingState>,
    pub chat_history: ResMut<'w, chat::ChatHistory>,
    pub active_delivery: ResMut<'w, delivery::ActiveDelivery>,
    pub dungeon_dash_selection: ResMut<'w, delivery::DungeonDashState>,
    pub cockatrice_state: ResMut<'w, mobile_apps::CockatriceState>,
    pub crawlr_state: ResMut<'w, mobile_apps::CrawlrState>,
    pub turn_counter: ResMut<'w, TurnCounter>,
    pub next_phone_screen: ResMut<'w, NextState<phone::PhoneScreen>>,
    pub next_dungeon_dash_screen: ResMut<'w, NextState<delivery::DungeonDashScreen>>,
    pub examine_pos: ResMut<'w, examine::ExaminePos>,
    pub examine_results: ResMut<'w, examine::ExamineResults>,
    pub screen_shake: ResMut<'w, camera::ScreenShake>,
    pub pending_damage: ResMut<'w, PendingDamage>,
    pub valid_targets: ResMut<'w, targeting::ValidTargets>,
    pub nearby_mobs: ResMut<'w, NearbyMobs>,
    pub last_title_drop_level: ResMut<'w, LastTitleDropLevel>,
    pub player_memory_map: ResMut<'w, PlayerMemoryMap>,
    #[allow(dead_code)]
    pub lighting_settings: Res<'w, lighting::LightingSettings>,
    pub game_over_info: ResMut<'w, crate::screens::game_over::GameOverInfo>,
}

pub fn enter(
    mut commands: Commands,
    assets: Res<assets::WorldAssets>,
    _q_camera: Single<Entity, With<PrimaryCamera>>,
    mut params: GameResetParams,
) {
    let world = (
        GameWorld,
        Name::new("GameWorldRoot"),
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        InheritedVisibility::VISIBLE,
    );
    let world = commands.spawn(world).id();

    // Reset game state BEFORE generation
    *params.game_over_info = crate::screens::game_over::GameOverInfo::default();
    *params.phone = phone::PhoneState::default();
    *params.map_info = mapgen::MapInfo::default();
    *params.streaming_state = chat::StreamingState::default();
    *params.chat_history = chat::ChatHistory::default();
    *params.active_delivery = delivery::ActiveDelivery::default();
    *params.dungeon_dash_selection = delivery::DungeonDashState::default();
    *params.cockatrice_state = mobile_apps::CockatriceState::default();
    *params.crawlr_state = mobile_apps::CrawlrState::default();
    *params.turn_counter = TurnCounter::default();
    *params.examine_pos = examine::ExaminePos::default();
    *params.examine_results = examine::ExamineResults::default();
    *params.screen_shake = camera::ScreenShake::default();
    *params.pending_damage = PendingDamage::default();
    *params.valid_targets = targeting::ValidTargets::default();
    *params.nearby_mobs = NearbyMobs::default();
    *params.last_title_drop_level = LastTitleDropLevel::default();
    *params.player_memory_map = PlayerMemoryMap::default();

    params.next_phone_screen.set(phone::PhoneScreen::Home);
    params
        .next_dungeon_dash_screen
        .set(delivery::DungeonDashScreen::RoleSelection);

    examine::init_examine_highlight(world, &mut commands, &assets);
    #[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
    {
        if params.lighting_settings.fancy_lighting {
            lighting::enable_lighting(&mut commands, *_q_camera);
        }
    }
    mapgen::gen_map(world, &mut commands, assets, &mut params.map_info);

    commands.spawn((
        FovMask,
        Name::new("FovMask"),
        Sprite::default(),
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        Visibility::Hidden,
    ));

    commands.run_schedule(Turn);
}

pub fn exit(
    mut commands: Commands,
    _q_camera: Single<Entity, With<PrimaryCamera>>,
    game_world: Query<Entity, With<GameWorld>>,
    fov_mask: Query<Entity, With<FovMask>>,
) {
    for world in game_world.iter() {
        commands.entity(world).despawn();
    }
    for mask in fov_mask.iter() {
        commands.entity(mask).despawn();
    }
    #[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
    {
        info!("got here!");
        lighting::disable_lighting(&mut commands, *_q_camera);
    }
}

fn draw_status_indicator(
    mut contexts: EguiContexts,
    player: Single<(&Player, &Creature, &MapPos)>,
    map_info: Res<mapgen::MapInfo>,
    q_grass: Query<&MapPos, With<map::Grass>>,
    crawlr_state: Res<mobile_apps::CrawlrState>,
) {
    let (player, _creature, player_pos) = player.into_inner();

    let mut is_touching_grass = false;
    if let Some(level) = map_info.get_level(*player_pos)
        && level.ty == mapgen::LevelTitle::Minecraft
        && q_grass.iter().any(|gp| gp.0 == player_pos.0)
    {
        is_touching_grass = true;
    }

    if player.hunger < 100 && !is_touching_grass && crawlr_state.match_effect.is_none() {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let (text, color) = if let Some(effect) = &crawlr_state.match_effect {
        let displayed = &effect.text[..effect.teletype_index];
        (displayed, egui::Color32::from_rgb(255, 50, 50))
    } else if player.hunger >= 100 {
        let text = if player.brainrot > 80 {
            if player.strength == 0 {
                "HUNGERMAXXING FR"
            } else {
                "HUNGERMAXXING"
            }
        } else if player.strength == 0 {
            "STARVING TO DEATH"
        } else {
            "STARVING"
        };
        (text, egui::Color32::RED)
    } else {
        ("TOUCHING GRASS", egui::Color32::GREEN)
    };

    egui::Area::new(egui::Id::new("hunger_warning"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
        .show(ctx, |ui| {
            ui.add(
                egui::Label::new(apply_brainrot_ui(
                    egui::RichText::new(text).color(color).size(56.0).strong(),
                    player.brainrot,
                    ui.style(),
                    egui::FontSelection::Default,
                    egui::Align::Center,
                ))
                .wrap_mode(egui::TextWrapMode::Extend),
            );
        });
}

fn update_fov_mask(
    _commands: Commands,
    lighting_settings: Res<lighting::LightingSettings>,
    player_vis_map: Res<PlayerVisibilityMap>,
    player_memory_map: Res<PlayerMemoryMap>,
    mut q_mask: Query<(Entity, &mut Visibility, &mut Transform, &mut Sprite), With<FovMask>>,
    mut images: ResMut<Assets<Image>>,
    player: Single<&MapPos, With<Player>>,
    map_info: Res<mapgen::MapInfo>,
) {
    let Some((_entity, mut visibility, mut transform, mut sprite)) = q_mask.iter_mut().next()
    else {
        return;
    };

    if lighting_settings.fancy_lighting {
        *visibility = Visibility::Hidden;
        return;
    }

    let player_pos = **player;
    let Some(level) = map_info.get_level(player_pos) else {
        return;
    };

    *visibility = Visibility::Inherited;

    let rect = level.rect;
    let width = rect.width() as u32;
    let height = rect.height() as u32;

    // Check if we need to recreate the image
    let needs_new_image = if let Some(img) = images.get(sprite.image.id()) {
        img.texture_descriptor.size.width != width || img.texture_descriptor.size.height != height
    } else {
        true
    };

    if needs_new_image {
        let data = vec![0u8; (width * height * 4) as usize];

        let mut image = Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();

        let image_handle = images.add(image);

        // Position the mask
        let world_width = rect.width() as f32 * map::TILE_WIDTH;
        let world_height = rect.height() as f32 * map::TILE_HEIGHT;
        let center_x = (rect.x1 as f32 + rect.x2 as f32) * map::TILE_WIDTH / 2.0;
        let center_y = (rect.y1 as f32 + rect.y2 as f32) * map::TILE_HEIGHT / 2.0;

        transform.translation = Vec3::new(center_x, center_y, TILE_Z - 0.02);

        sprite.image = image_handle;
        sprite.custom_size = Some(Vec2::new(world_width, world_height));
    }

    // Update the image data
    if let Some(img) = images.get_mut(sprite.image.id()) {
        let Some(ref mut data) = img.data else { return };
        for y in 0..height {
            for x in 0..width {
                let map_x = rect.x1 + x as i32;
                let map_y = rect.y1 + y as i32;
                let pos = IVec2::new(map_x, map_y);

                let alpha = if player_vis_map.contains(&pos) {
                    0u8
                } else if player_memory_map.contains(&pos) {
                    180u8
                } else {
                    255u8
                };

                // Invert Y for texture coordinates (0,0 is top-left in image data)
                let img_y = height - 1 - y;
                let idx = ((img_y * width + x) * 4) as usize;
                if data[idx + 3] != alpha {
                    data[idx + 3] = alpha;
                    data[idx] = 0;
                    data[idx + 1] = 0;
                    data[idx + 2] = 0;
                }
            }
        }
    }
}

fn update_crawlr_animation(
    mut state: ResMut<mobile_apps::CrawlrState>,
    mut player_query: Single<&mut Player>,
    mut mob_query: Query<(&crate::game::DropsCorpse, &mut Mob)>,
    time: Res<Time>,
) {
    if let Some(effect) = &mut state.match_effect {
        effect.timer -= time.delta_secs();
        effect.teletype_timer -= time.delta_secs();
        if effect.teletype_timer <= 0.0 && effect.teletype_index < effect.text.len() {
            effect.teletype_index += 1;
            effect.teletype_timer = 0.05;
        }
        if effect.timer <= 0.0 {
            state.match_effect = None;
        }
    }

    // Update swipe animation timer
    if state.swipe_animation_timer > 0.0 {
        state.swipe_animation_timer -= time.delta_secs();
        if state.swipe_animation_timer <= 0.0 {
            state.swipe_animation_timer = 0.0;
            // Finish swipe
            if let Some(entity) = state.last_swiped_entity.take() {
                state.swiped_entities.insert(entity);
                player_query.brainrot += 2;

                let is_match = state.matches.contains(&entity);
                if state.last_swiped_is_like {
                    if is_match {
                        state.pending_faction_changes.insert(entity, ALLIED_FACTION);
                        if let Ok((_, mut mob)) = mob_query.get_mut(entity) {
                            mob.target = None;
                        }
                    } else {
                        // PROACTIVE SWIPE
                        if let Ok((corpse, _)) = mob_query.get(entity) {
                            let chance = (corpse.kind.get_attractiveness() as f64 / 100.0) * 0.03;
                            state.pending_swipes.push(mobile_apps::PendingRightSwipe {
                                entity,
                                turns_remaining: 20,
                                chance,
                            });
                        }
                    }
                } else {
                    if is_match {
                        state.pending_psychic_damage.insert(entity);
                        state.match_effect = Some(mobile_apps::MatchEffect {
                            text: "ouch".to_string(),
                            timer: 2.0,
                            teletype_index: 0,
                            teletype_timer: 0.05,
                        });
                    }
                }
            }
        }
    }
}
