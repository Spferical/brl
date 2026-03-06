use std::{f32::consts::PI, time::Duration};

use bevy::{
    ecs::{schedule::ScheduleLabel, system::SystemParam},
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Align, FontSelection, Margin, RichText, WidgetText, text::LayoutJob},
};
use bevy_firefly::prelude::FireflyPlugin;
use rand::{
    Rng as _,
    seq::{IndexedRandom, SliceRandom as _},
};

use crate::{
    asset_tracking::LoadResource as _,
    game::{
        animation::{DamageAnimationMessage, FloatingTextMessage, MoveAnimation},
        assets::WorldAssets,
        debug::{DebugSettings, redo_faction_map},
        input::{AbilityClicked, EatEvent, InputMode, PlayerIntent, StairsClicked},
        map::{MapPos, PosToCreature, PosToInteractable, TILE_HEIGHT, TILE_WIDTH},
        mapgen::MobKind,
        phone::PhoneState,
    },
    screens::Screen,
};

mod animation;
mod assets;
mod camera;
mod chat;
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
mod targeting;
mod upgrades;

const HIGHLIGHT_Z: f32 = 20.0;
const DAMAGE_Z: f32 = 15.0;
const PLAYER_Z: f32 = 10.0;
const CORPSE_Z: f32 = 5.0;
const TILE_Z: f32 = 0.0;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(FireflyPlugin);
    app.insert_resource(ClearColor(Color::BLACK));
    app.load_resource::<assets::WorldAssets>();
    app.init_resource::<map::WalkBlockedMap>();
    app.init_resource::<map::SightBlockedMap>();
    app.init_resource::<map::PlayerVisibilityMap>();
    app.init_resource::<map::PlayerMemoryMap>();
    app.init_resource::<camera::ScreenShake>();
    app.init_resource::<PendingDamage>();
    app.init_resource::<PlayerMoved>();
    app.init_resource::<FactionMap>();
    app.init_resource::<PosToCreature>();
    app.init_resource::<PosToInteractable>();
    app.init_resource::<NearbyMobs>();
    app.init_resource::<DebugSettings>();
    app.init_resource::<examine::ExaminePos>();
    app.init_resource::<examine::ExamineResults>();
    app.init_resource::<input::InputMode>();
    app.init_resource::<targeting::ValidTargets>();
    app.init_resource::<phone::PhoneState>();
    app.init_resource::<mobile_apps::DungeonDashSelection>();
    app.init_resource::<mobile_apps::CockatriceState>();
    app.init_resource::<delivery::ActiveDelivery>();
    app.init_resource::<chat::StreamingState>();
    app.init_resource::<chat::ChatHistory>();
    app.init_resource::<TurnCounter>();
    app.init_state::<phone::PhoneScreen>();
    app.init_state::<mobile_apps::DungeonDashScreen>();
    app.add_message::<DamageAnimationMessage>();
    app.add_message::<FloatingTextMessage>();
    app.add_message::<input::AbilityClicked>();
    app.add_message::<input::StairsClicked>();
    app.add_message::<input::EatEvent>();
    app.add_message::<upgrades::UpgradeMessage>();
    app.add_systems(
        Update,
        (
            lighting::on_add_occluder,
            lighting::on_add_player,
            input::handle_input.run_if(is_player_alive.and(phone::is_phone_closed)),
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
            (
                animation::process_move_animations,
                animation::process_attack_animations,
                animation::spawn_damage_animations,
                animation::spawn_floating_messages,
                animation::update_floating_text,
            )
                .chain(),
            camera::update_camera,
            examine::update_examine_info,
            examine::highlight_examine_tile,
            delivery::draw_delivery_indicators,
            upgrades::handle_upgrades,
        )
            .run_if(in_state(Screen::Gameplay))
            .chain(),
    );
    app.init_schedule(Turn);
    app.add_systems(
        Turn,
        (
            map::update_walk_blocked_map,
            map::update_pos_to_interactable,
            handle_player_move,
            handle_eat,
            (
                (
                    increment_turn_counter,
                    chat::update_streaming_turn,
                    tick_meters,
                    handle_subscriptions,
                    signal::update_player_signal,
                    delivery::process_deliveries,
                )
                    .chain(),
                // kill mobs from any player damage
                (apply_damage, prune_dead).chain(),
                // environment
                map::update_pos_to_creature,
                process_spawners,
                spawn_klarna_kop,
                map::update_pos_to_creature,
                // bullets
                (check_bullet_collision, move_bullets, check_bullet_collision).chain(),
                // mobs get a turn
                build_faction_map,
                process_mob_turn,
                map::update_pos_to_creature,
                check_bullet_collision,
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
            draw_hunger_warning,
            chat::draw_streaming_indicator,
            phone::draw_phone,
            chat::draw_chat,
            draw_interactable_popup,
        )
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

pub(crate) fn draw_interactable_popup(
    mut contexts: EguiContexts,
    player_query: Single<(Entity, &MapPos, &Player)>,
    interactable_query: Query<(
        Entity,
        &MapPos,
        Option<&Name>,
        &Interactable,
        Option<&delivery::Food>,
    )>,
    q_camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let (_player_entity, player_pos, player) = player_query.into_inner();
    let (camera, camera_transform) = *q_camera;

    for (_entity, pos, name, interactable, food) in interactable_query.iter() {
        if pos.0 == player_pos.0 {
            // Get screen position
            let world_pos = player_pos.to_vec3(PLAYER_Z);
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
                    format!("{} {}? (e)", verb, food_item.name),
                    Some(food_item.effects.to_string()),
                )
            } else {
                (
                    format!(
                        "{} {}? (e)",
                        interactable.action,
                        name.map(|n| n.as_str()).unwrap_or("")
                    ),
                    interactable.description.clone(),
                )
            };

            draw_world_popup(ctx, viewport_pos, title, description, player.brainrot);
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
pub struct GameWorld;

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum Subscription {
    DungeonDashPlatinum,
    UndergroundTVPro,
    FiveGLTE,
    DungeonFitness,
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
        (self.strength / 5).max(1)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum Ability {
    Sprint,
    ShoulderCheck,
    Mog,
    Cook,
    ReadBook,
}

impl std::fmt::Display for Ability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Ability::Sprint => "Sprint",
            Ability::ShoulderCheck => "Shoulder Check",
            Ability::Mog => "Mog",
            Ability::Cook => "Cook",
            Ability::ReadBook => "Read Book",
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
    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Ability::Sprint => "Move multiple tiles in one turn. Costs hunger.",
            Ability::ShoulderCheck => "Damage and swap positions with an adjacent enemy.",
            Ability::Mog => "Deal aura damage to an adjacent enemy.",
            Ability::Cook => "Cook a corpse you are standing on. Requires < 10 brainrot.",
            Ability::ReadBook => {
                "Reduce brainrot. Might be a little boring. (Borrow period: 10 turns)"
            }
        }
    }
    pub(crate) fn target(&self) -> AbilityTarget {
        match self {
            Ability::Sprint => AbilityTarget::ReachableTile { maxdist: 5 },
            Ability::ShoulderCheck => AbilityTarget::NearbyMob { maxdist: 1 },
            Ability::Mog => AbilityTarget::NearbyMob { maxdist: 5 },
            Ability::Cook | Ability::ReadBook => AbilityTarget::NoTarget,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionType {
    Stairs,
    Eat,
    Workout,
}

#[derive(Component)]
pub struct Interactable {
    pub action: String,
    pub description: Option<String>,
    pub kind: InteractionType,
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            action: "Use".to_string(),
            description: None,
            kind: InteractionType::Stairs,
        }
    }
}

#[derive(Component, Clone)]
pub(crate) struct CookedMeal {
    pub hunger: i32,
    pub hp: i32,
    pub strength: i32,
    pub boredom: i32,
}

pub(crate) fn handle_eat(
    mut events: MessageReader<EatEvent>,
    player_query: Single<(&mut Player, &mut Creature)>,
    food_query: Query<(Option<&delivery::Food>, Option<&CookedMeal>)>,
    mut commands: Commands,
) {
    let (mut player, mut creature) = player_query.into_inner();
    for event in events.read() {
        if let Ok((food, cooked)) = food_query.get(event.0) {
            if let Some(food) = food {
                let food_item = delivery::FOODS[food.food_idx];
                player.hunger = (player.hunger + food_item.hunger).clamp(0, 100);
                player.strength += food_item.strength;
                player.rizz += food_item.rizz;
                creature.hp = (creature.hp + food_item.hp).clamp(0, creature.max_hp);
            } else if let Some(cooked) = cooked {
                player.hunger = (player.hunger - cooked.hunger).clamp(0, 100);
                player.strength += cooked.strength;
                player.boredom = (player.boredom - cooked.boredom).clamp(0, 100);
                creature.hp = (creature.hp + cooked.hp).clamp(0, creature.max_hp);
            }
            commands.entity(event.0).despawn();
        }
    }
}

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
) {
    egui::Area::new(egui::Id::new(&title))
        .fixed_pos(egui::pos2(viewport_pos.x - 100.0, viewport_pos.y - 120.0))
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
}

/// Common fields between the player and mobs.
#[derive(Component, Clone, Debug, Reflect)]
pub(crate) struct Creature {
    pub hp: i32,
    pub max_hp: i32,
    pub faction: i32,
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
pub(crate) struct MobAttrs {
    pub based: bool,
    pub basic: bool,
    pub mog_risk: bool,
    pub sus: bool,
    pub knows_player_location: bool,
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
#[derive(Component, Clone, Debug, Reflect)]
pub(crate) struct Mob {
    pub melee_damage: i32,
    pub ranged: bool,
    pub attrs: MobAttrs,
    pub target: Option<IVec2>,
}

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

fn tick_meters(turn_counter: Res<TurnCounter>, player: Single<(&mut Player, &mut Creature)>) {
    let (mut player, mut creature) = player.into_inner();

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

    if turn_counter.0.is_multiple_of(5) {
        player.apply_hunger_damage(&mut creature, 1);

        if player.boredom >= 100 {
            creature.hp -= 1;
        }
        player.boredom += 1;
        player.boredom = player.boredom.clamp(0, 100);

        if player.has_subscription(Subscription::DungeonFitness) && player.strength < 60 {
            player.strength += 1;
        }
    }
}

fn handle_subscriptions(turn_counter: Res<TurnCounter>, mut player: Single<&mut Player>) {
    if turn_counter.0 > 0 && turn_counter.0.is_multiple_of(100) {
        for sub in player.subscriptions.clone() {
            let cost = match sub {
                Subscription::DungeonDashPlatinum => 20,
                Subscription::UndergroundTVPro => 50,
                Subscription::FiveGLTE => 5,
                Subscription::DungeonFitness => 80,
            };
            player.money -= cost;
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
    mut mobs: Query<&mut MapPos, (With<Creature>, Without<Player>, Without<Corpse>)>,
    stairs: Query<&Stairs, (Without<Player>, Without<Creature>)>,
    interactables: Query<&Interactable>,
    q_corpses: Query<
        (Entity, &MapPos, &Corpse),
        (With<Corpse>, Without<Player>, Without<Creature>),
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
                moved.0 = false;
                return;
            }

            let new_pos = target;

            if let Some(entity) = pos_to_creature.0.get(&new_pos.0) {
                damage.0.push(DamageInstance {
                    entity: *entity,
                    amount: player_stats.melee_damage(),
                    ty: DamageType::Physical,
                });
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
                    rotation: None,
                    sway,
                });

                let new_depth = (destination.0.y / 200).max(0);
                if new_depth > player_stats.max_depth {
                    player_stats.max_depth = new_depth;
                    player_stats.pending_upgrades += 1;
                }

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
                        amount: 2,
                        ty: DamageType::Physical,
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
            Ability::Mog => {
                let new_pos = map_pos;
                let old_pos = pos;
                if let Some(mob_entity) = pos_to_creature.0.get(&new_pos.0) {
                    damage.0.push(DamageInstance {
                        entity: *mob_entity,
                        amount: 2,
                        ty: DamageType::Aura,
                    });
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
                            meal_stats,
                            Interactable {
                                action: "Eat".to_string(),
                                description: Some(format!("A freshly cooked {}!", meal_name)),
                                kind: InteractionType::Eat,
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
                    player_stats.boredom = (player_stats.boredom + boredom_increase).min(100);
                    player_stats.ability_cooldowns.insert(Ability::ReadBook, 10);
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

fn spawn_klarna_kop(
    turn_counter: Res<TurnCounter>,
    player: Single<(&Player, &MapPos)>,
    pos_to_creature: Res<map::PosToCreature>,
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    assets: Res<assets::WorldAssets>,
    tiles: Query<&MapPos, (With<map::Tile>, Without<map::BlocksMovement>)>,
) {
    let debt = -player.0.money;
    if turn_counter.0 > 0 && debt > 5 {
        let spawn_rate = if debt > 50 { 5 } else { 10 };
        if turn_counter.0.is_multiple_of(spawn_rate) {
            let ppos = player.1.0;
            let mut rng = rand::rng();
            let mut valid_spots = Vec::new();
            // Limit search to a 100x100 area around the player for performance
            let search_range = 100;
            for &MapPos(pos) in tiles.iter() {
                let diff = (pos - ppos).abs();
                let dist = diff.max_element();
                if dist > 2 && dist <= search_range && !pos_to_creature.0.contains_key(&pos) {
                    valid_spots.push(MapPos(pos));
                }
            }

            if !valid_spots.is_empty()
                && let Some(pos) = valid_spots.choose(&mut rng)
            {
                let map_pos = *pos;
                let transform = Transform::from_translation(map_pos.to_vec3(PLAYER_Z));
                let mut bundle = mapgen::MobKind::KlarnaKop.get_bundle(&assets);

                if debt > 100 {
                    bundle.creature.hp *= 2;
                    bundle.creature.max_hp *= 2;
                }

                let new_mob = commands.spawn((bundle, map_pos, transform)).id();
                commands.entity(world.into_inner()).add_child(new_mob);
            }
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

pub struct DamageInstance {
    entity: Entity,
    amount: i32,
    ty: DamageType,
}

#[derive(Resource, Default)]
pub struct PendingDamage(Vec<DamageInstance>);

fn check_bullet_collision(
    mut commands: Commands,
    pos_to_mob: Res<PosToCreature>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
    bullets: Query<(Entity, &MapPos, &Bullet)>,
    mut damage: ResMut<PendingDamage>,
    player_q: Query<Entity, With<Player>>,
    mut screen_shake: ResMut<camera::ScreenShake>,
) {
    for (entity, pos, bullet) in bullets.iter() {
        if let Some(mob) = pos_to_mob.0.get(&pos.0) {
            damage.0.push(DamageInstance {
                entity: *mob,
                amount: bullet.damage,
                ty: DamageType::Physical,
            });
            if player_q.get(*mob).is_ok() {
                screen_shake.trauma = (screen_shake.trauma + 0.6).min(1.0);
            } else {
                screen_shake.trauma = (screen_shake.trauma + 0.2).min(1.0);
            }
        }
        if pos_to_mob.0.contains_key(&pos.0) || walk_blocked_map.0.contains(&pos.0) {
            commands.entity(entity).despawn();
        }
    }
}

fn apply_damage(
    mut damage: ResMut<PendingDamage>,
    mut animation: MessageWriter<DamageAnimationMessage>,
    mut creature: Query<(&mut Creature, Option<&mut Player>, Option<&Mob>, &Transform)>,
) {
    for DamageInstance { entity, amount, ty } in damage.0.drain(..) {
        if let Ok((mut creature, player, mob, transform)) = creature.get_mut(entity) {
            let world_pos = transform.translation;
            let is_player = player.is_some();
            let mut final_amount = amount;
            match player {
                Some(mut player) => match ty {
                    DamageType::Physical => creature.hp -= amount,
                    DamageType::Psychic => {
                        player.brainrot += amount;
                        if player.brainrot > 100 {
                            creature.hp -= player.brainrot - 100;
                            player.brainrot = 100;
                        }
                    }
                    DamageType::Aura => {
                        player.rizz -= amount;
                        if player.rizz < 0 {
                            creature.hp += player.rizz;
                            player.rizz = 0;
                        }
                    }
                    DamageType::Boredom => {
                        player.boredom += amount;
                        player.brainrot -= amount;
                        player.brainrot = player.brainrot.clamp(0, 100);
                        if player.boredom > 100 {
                            creature.hp -= player.boredom - 100;
                            player.boredom = 100;
                        }
                    }
                    DamageType::Hunger => {
                        player.apply_hunger_damage(&mut creature, amount);
                    }
                    DamageType::Strength => {
                        player.apply_strength_damage(&mut creature, amount);
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
                    creature.hp -= final_amount;
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
    let mut dijkstra_map_per_faction =
        HashMap::<i32, std::collections::HashMap<MapPos, usize>>::new();
    for (faction, friendly_positions) in positions_per_faction.iter() {
        if *faction == 0 {
            // Player doesn't need a dijkstra map to find targets.
            continue;
        }
        let enemy_positions = positions_per_faction
            .iter()
            .filter(|(f, _positions)| **f != *faction)
            .flat_map(|(_f, positions)| positions)
            .copied()
            .map(MapPos)
            .collect::<Vec<_>>();
        let reachable_cb = |p| reachable(p, &walk_blocked_map, friendly_positions);
        let maxdist = 100;
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
    mut mobs: Query<(Entity, &mut MapPos, &Creature, &mut Mob), Without<Player>>,
    mut commands: Commands,
    mut damage: ResMut<PendingDamage>,
    player_q: Single<(Entity, &MapPos, &Creature), With<Player>>,
    mut screen_shake: ResMut<camera::ScreenShake>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
    sight_blocked_map: Res<map::SightBlockedMap>,
    all_creatures: Query<&Creature>,
) {
    enum Action {
        Move(MapPos),
        Melee(Entity, MapPos),
        RangedAttack(MapPos),
        AttackAndTeleport(Entity, MapPos),
    }

    let world_entity = world.into_inner();
    let rng = &mut rand::rng();
    let (player_entity, player_pos, _) = *player_q;
    // Determine mob intentions.
    let mut mob_moves = HashMap::new();
    let mut claimed_locations = HashSet::new();
    for (entity, pos, creature, mut mob) in mobs.iter_mut() {
        if creature.is_dead() {
            continue;
        }

        if mob.attrs.knows_player_location {
            mob.target = Some(player_pos.0);
        } else {
            let fov = rogue_algebra::fov::calculate_fov(pos.0.into(), 10, |p| {
                sight_blocked_map.contains(&IVec2::from(p))
            });

            let mut sees_enemy = false;
            let mut target_pos = None;
            for visible_pos in fov {
                let visible_ivec = IVec2::from(visible_pos);
                if let Some(other_entity) = pos_to_creature.0.get(&visible_ivec)
                    && let Ok(other_creature) = all_creatures.get(*other_entity)
                    && other_creature.faction != creature.faction
                {
                    sees_enemy = true;
                    target_pos = Some(visible_ivec);
                    break;
                }
            }

            if sees_enemy {
                mob.target = target_pos;
            } else if let Some(t) = mob.target
                && pos.0 == t
            {
                mob.target = None;
            }
        }

        if mob.target.is_none() {
            continue;
        }

        // follow dijkstra map
        let Some(dijkstra_map) = faction_map.dijkstra_map_per_faction.get(&creature.faction) else {
            continue;
        };

        let mut adjacent = pos.adjacent();
        adjacent.shuffle(rng);
        let target = adjacent
            .into_iter()
            .filter(|p| dijkstra_map.contains_key(p))
            .filter(|p| !claimed_locations.contains(&p.0))
            .min_by_key(|p| dijkstra_map.get(p).cloned().unwrap_or(usize::MAX));

        if let Some(target) = target {
            if mob.ranged && rng.random_bool(0.5) {
                mob_moves.insert(entity, Action::RangedAttack(target));
            } else if let Some(occupier) = pos_to_creature.0.get(&target.0) {
                if mob.attrs.sus {
                    let teleport_target = rogue_algebra::path::bfs_paths(&[*pos], 25, |p| {
                        p.adjacent()
                            .into_iter()
                            .filter(|p| !walk_blocked_map.contains(&p.0))
                    })
                    .filter(|path| {
                        let pos = path.last().unwrap();
                        !pos_to_creature.0.contains_key(&pos.0)
                            && !claimed_locations.contains(&pos.0)
                    })
                    .max_by_key(|path| path.len())
                    .map(|path| *path.last().unwrap());
                    if let Some(teleport_target) = teleport_target {
                        mob_moves.insert(
                            entity,
                            Action::AttackAndTeleport(*occupier, teleport_target),
                        );
                        claimed_locations.insert(teleport_target.0);
                    } else {
                        mob_moves.insert(entity, Action::Melee(*occupier, target));
                    }
                } else {
                    mob_moves.insert(entity, Action::Melee(*occupier, target));
                }
            } else {
                mob_moves.insert(entity, Action::Move(target));
            }
            // Claim any move that is not a destination.
            // This works because destinations are always enemies.
            if dijkstra_map.get(&target) != Some(&1) {
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
                    rotation: None,
                    sway: None,
                });
            }
            Action::Melee(enemy, new_pos) => {
                damage.0.push(DamageInstance {
                    entity: enemy,
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
            }
            Action::AttackAndTeleport(enemy, teleport_pos) => {
                damage.0.push(DamageInstance {
                    entity: enemy,
                    amount: mob.melee_damage,
                    ty: mob.get_melee_damage_type(),
                });
                *pos = teleport_pos;
                commands.entity(entity).insert(MoveAnimation {
                    from: old_pos.to_vec3(PLAYER_Z),
                    to: teleport_pos.to_vec3(PLAYER_Z),
                    timer: Timer::new(Duration::from_millis(1), TimerMode::Once),

                    ease: EaseFunction::Linear,
                    rotation: None,
                    sway: None,
                });
                if enemy == player_entity {
                    screen_shake.trauma = (screen_shake.trauma + 0.7).min(1.0);
                }
            }
        }
    }
}

fn prune_dead(
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    q_creatures: Query<(Entity, &Creature, &MapPos, Option<&DropsCorpse>), Without<Player>>,
    mut player: Single<&mut Player>,
    streaming_state: Res<chat::StreamingState>,
    mut chat: ResMut<chat::ChatHistory>,
) {
    let world_entity = world.into_inner();
    let player = player.as_mut();
    for (entity, creature, map_pos, corpse) in q_creatures {
        if creature.is_dead() {
            commands.entity(entity).despawn();

            chat::handle_payout(player, &streaming_state, &mut chat);

            if let Some(DropsCorpse {
                sprite,
                nutrition,
                name,
                kind,
            }) = corpse
            {
                let transform = Transform::from_translation(map_pos.to_vec3(CORPSE_Z));
                let corpse_id = commands
                    .spawn((
                        Corpse {
                            nutrition: *nutrition,
                            name: name.clone(),
                            kind: *kind,
                        },
                        sprite.clone(),
                        *map_pos,
                        transform,
                    ))
                    .id();
                commands.entity(world_entity).add_child(corpse_id);
            }
        }
    }
}

#[derive(Default, Resource)]
struct NearbyMobs {
    mobs: Vec<(MapPos, Creature, Option<Mob>, String, Color, Name)>,
}

fn update_nearby_mobs(
    mut nearby_mobs: ResMut<NearbyMobs>,
    player: Query<&MapPos, With<Player>>,
    mobs: Query<(&MapPos, &Creature, Option<&Mob>, &Text2d, &TextColor, &Name), Without<Player>>,
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
            && let Some(mob) = pos_to_creature.0.get(&pos.0)
            && let Ok((pos, creature, mob, text, color, name)) = mobs.get(*mob)
        {
            nearby_mobs.mobs.push((
                *pos,
                creature.clone(),
                mob.cloned(),
                text.0.clone(),
                color.0,
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
    input_mode: Res<InputMode>,
    mut msg_ability_clicked: MessageWriter<AbilityClicked>,
) {
    let (player, _) = player.into_inner();
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

                for (pos, creature, mob, text, color, name) in nearby_mobs.mobs.iter() {
                    let highlight = Some(*pos) == examine_pos;
                    let mut frame = egui::Frame::new().inner_margin(Margin::same(4));
                    if highlight {
                        frame = frame.fill(ui.style().visuals.code_bg_color);
                    }
                    frame.show(ui, |ui| {
                        ui.vertical(|ui| {
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
                            ui.horizontal(|ui| {
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
                                for _ in 0..creature.hp / 2 {
                                    ui.add(heart.clone());
                                }
                                if creature.hp % 2 == 1 {
                                    ui.add(half_heart.clone());
                                }
                                if let Some(mob) = mob {
                                    for _ in 0..mob.melee_damage / 2 {
                                        ui.add(sword.clone());
                                    }
                                    if mob.melee_damage % 2 == 1 {
                                        ui.add(half_sword.clone());
                                    }
                                    for (attr, name, color, tooltip) in [
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
                                            egui::Color32::RED,
                                            "Can teleport when attacking",
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
                        ui.label(apply_brainrot_ui(
                            "examine: x",
                            player.brainrot,
                            ui.style(),
                            FontSelection::Default,
                            Align::LEFT,
                        ));

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
                        1 | 2 => egui::Color32::from_rgb(255, 50, 50), // Red
                        3 => egui::Color32::from_rgb(255, 255, 50),    // Yellow
                        4 | 5 => egui::Color32::from_rgb(50, 255, 50), // Green
                        _ => egui::Color32::from_rgb(255, 50, 50),
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
        });
}

pub fn enter(
    mut commands: Commands,
    assets: Res<assets::WorldAssets>,
    q_camera: Single<Entity, With<Camera2d>>,
    mut phone: ResMut<PhoneState>,
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
    mapgen::gen_map(world, &mut commands, assets);
    *phone = PhoneState::default();
    commands.run_schedule(Turn);
}

pub fn exit(
    mut commands: Commands,
    q_camera: Single<Entity, With<Camera2d>>,
    game_world: Query<Entity, With<GameWorld>>,
) {
    commands.entity(game_world.single().unwrap()).despawn();
    lighting::disable_lighting(&mut commands, *q_camera);
}

fn draw_hunger_warning(mut contexts: EguiContexts, player: Single<(&Player, &Creature)>) {
    let (player, _creature) = player.into_inner();
    if player.hunger < 100 {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
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

    egui::Area::new(egui::Id::new("hunger_warning"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
        .show(ctx, |ui| {
            ui.label(apply_brainrot_ui(
                egui::RichText::new(text)
                    .color(egui::Color32::RED)
                    .size(56.0)
                    .strong(),
                player.brainrot,
                ui.style(),
                egui::FontSelection::Default,
                egui::Align::Center,
            ));
        });
}
