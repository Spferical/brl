use std::{f32::consts::PI, time::Duration};

use bevy::{
    ecs::schedule::ScheduleLabel,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Align, FontSelection, Margin, RichText, WidgetText, text::LayoutJob},
};
use bevy_firefly::prelude::FireflyPlugin;
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
        input::{AbilityClicked, InputMode, PlayerIntent, StairsClicked},
        map::{MapPos, PosToCreature, PosToInteractable, TILE_HEIGHT, TILE_WIDTH},
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
    app.init_resource::<camera::ScreenShake>();
    app.init_resource::<PlayerAbilities>();
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
    app.init_resource::<delivery::ActiveDelivery>();
    app.init_resource::<chat::StreamingState>();
    app.init_resource::<chat::ChatHistory>();
    app.init_resource::<TurnCounter>();
    app.init_state::<phone::PhoneScreen>();
    app.init_state::<mobile_apps::DungeonDashScreen>();
    app.add_message::<DamageAnimationMessage>();
    app.add_message::<input::AbilityClicked>();
    app.add_message::<input::StairsClicked>();
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
            chat::update_streaming_stats,
            chat::update_money_timer,
            chat::update_chat,
            animation::process_move_animations,
            animation::process_attack_animations,
            animation::update_damage_animations,
            camera::update_camera,
            examine::update_examine_info,
            examine::highlight_examine_tile,
            delivery::draw_delivery_indicators,
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
            (
                (
                    increment_turn_counter,
                    chat::update_streaming_turn,
                    tick_meters,
                    signal::update_player_signal,
                    delivery::process_deliveries,
                )
                    .chain(),
                // kill mobs from any player damage
                (apply_damage, prune_dead).chain(),
                // environment
                map::update_pos_to_creature,
                process_spawners,
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
                spawn_damage_animations,
                prune_dead,
                map::update_pos_to_creature,
                // end-of-turn bookkeeping
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
        (apply_brainrot_to_world_text, apply_brainrot_visual_effects)
            .run_if(in_state(Screen::Gameplay)),
    );
    app.add_systems(
        EguiPrimaryContextPass,
        (
            sidebar,
            left_sidebar,
            chat::draw_streaming_indicator,
            phone::draw_phone,
            chat::draw_chat,
            delivery::draw_eat_popup,
            draw_interactable_popup,
        )
            .run_if(in_state(Screen::Gameplay)),
    );
}

pub(crate) fn draw_interactable_popup(
    mut contexts: EguiContexts,
    player_query: Single<(&MapPos, &Player)>,
    interactable_query: Query<(&MapPos, &Name, &Interactable, Option<&Stairs>)>,
    mut msg_stairs_clicked: MessageWriter<StairsClicked>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q_camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let (player_pos, player) = player_query.into_inner();
    let (camera, camera_transform) = *q_camera;

    for (pos, name, interactable, stairs) in interactable_query.iter() {
        if pos.0 == player_pos.0 {
            // Get screen position
            let world_pos = player_pos.to_vec3(PLAYER_Z);
            let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, world_pos) else {
                continue;
            };

            let Ok(ctx) = contexts.ctx_mut() else {
                return;
            };

            draw_world_popup(
                ctx,
                viewport_pos,
                format!("{} {}? (e)", interactable.action, name),
                interactable.description.clone(),
                player.brainrot,
            );

            if keyboard_input.just_pressed(KeyCode::KeyE) {
                if stairs.is_some() {
                    msg_stairs_clicked.write(StairsClicked);
                }
            }
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
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Ability {
    Sprint,
    ShoulderCheck,
    Mog,
}

impl std::fmt::Display for Ability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Ability::Sprint => "Sprint",
            Ability::ShoulderCheck => "Shoulder Check",
            Ability::Mog => "Mog",
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
            Ability::Mog => AbilityTarget::NearbyMob { maxdist: 5 },
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
    abilities.add_or_remove(player.rizz >= 10, Ability::Mog);
}

#[derive(Component)]
pub struct Interactable {
    pub action: String,
    pub description: Option<String>,
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            action: "Use".to_string(),
            description: None,
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
struct MobAttrs {
    based: bool,
    basic: bool,
    mog_risk: bool,
    sus: bool,
    aura_resist: Resist,
    physical_resist: Resist,
    psychic_resist: Resist,
    boredom_resist: Resist,
}

#[derive(Default, Clone, Copy, Debug, Reflect)]
enum Resist {
    Weak,
    #[default]
    Normal,
    Strong,
}

// NPC-specific fields.
#[derive(Component, Clone, Debug, Reflect)]
struct Mob {
    melee_damage: i32,
    ranged: bool,
    attrs: MobAttrs,
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
    player: Single<(Entity, &mut MapPos, &PlayerIntent, &mut Player)>,
    mut mobs: Query<&mut MapPos, (With<Creature>, Without<Player>)>,
    stairs: Query<&Stairs, (Without<Player>, Without<Creature>)>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
    pos_to_creature: Res<PosToCreature>,
    turn_counter: Res<TurnCounter>,
    mut damage: ResMut<PendingDamage>,
    mut moved: ResMut<PlayerMoved>,
    mut screen_shake: ResMut<camera::ScreenShake>,
    pos_to_interactable: Res<PosToInteractable>,
) {
    let (player_entity, mut pos, intent, mut player_stats) = player.into_inner();
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
            let new_pos = rogue_algebra::path::bfs_paths(&[old_pos], 50, |p| {
                p.adjacent()
                    .into_iter()
                    .filter(|p| !walk_blocked_map.contains(&p.0))
            })
            .find(|path| path.last().unwrap() == target)
            .and_then(|path| path.into_iter().nth(1));
            let Some(new_pos) = new_pos else {
                moved.0 = false;
                return;
            };

            if let Some(entity) = pos_to_creature.0.get(&new_pos.0) {
                damage.0.push(DamageInstance {
                    entity: *entity,
                    amount: 2,
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
                player_stats.hunger += dist as i32;
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum DamageType {
    Physical,
    Psychic,
    Aura,
    Boredom,
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
    mut creature: Query<(&mut Creature, Option<&mut Player>, Option<&Mob>)>,
) {
    for DamageInstance { entity, amount, ty } in damage.0.drain(..) {
        if let Ok((mut creature, player, mob)) = creature.get_mut(entity) {
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
                },
                None => {
                    let resist = mob
                        .map(|m| m.get_damage_resist(ty))
                        .unwrap_or(Resist::Normal);
                    creature.hp -= match resist {
                        Resist::Weak => amount * 2,
                        Resist::Normal => amount,
                        Resist::Strong => amount / 2,
                    };
                }
            }
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
    player_q: Query<Entity, With<Player>>,
    mut screen_shake: ResMut<camera::ScreenShake>,
    walk_blocked_map: Res<map::WalkBlockedMap>,
) {
    enum Action {
        Move(MapPos),
        Melee(Entity, MapPos),
        RangedAttack(MapPos),
        AttackAndTeleport(Entity, MapPos),
    }

    let world_entity = world.into_inner();
    let rng = &mut rand::rng();
    // Determine mob intentions.
    let mut mob_moves = HashMap::new();
    let mut claimed_locations = HashSet::new();
    for (entity, pos, creature, mob) in mobs.iter() {
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
                if player_q.get(enemy).is_ok() {
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
                if player_q.get(enemy).is_ok() {
                    screen_shake.trauma = (screen_shake.trauma + 0.7).min(1.0);
                }
            }
        }
    }
}

fn prune_dead(
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    mut damage_animation: MessageWriter<DamageAnimationMessage>,
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
    player: Single<(&Player, &MapPos)>,
    pos_to_interactable: Res<PosToInteractable>,
    interactables: Query<Option<&Stairs>>,
    nearby_mobs: Res<NearbyMobs>,
    examine_results: Res<examine::ExamineResults>,
    world_assets: If<Res<WorldAssets>>,
    atlas_assets: If<Res<Assets<TextureAtlasLayout>>>,
    player_abilities: Res<PlayerAbilities>,
    input_mode: Res<InputMode>,
    mut msg_ability_clicked: MessageWriter<AbilityClicked>,
    mut msg_stairs_clicked: MessageWriter<StairsClicked>,
) {
    let (player, player_pos) = player.into_inner();
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
                                for _ in 0..mob.melee_damage / 2 {
                                    ui.add(sword.clone());
                                }
                                if mob.melee_damage % 2 == 1 {
                                    ui.add(half_sword.clone());
                                }
                                for (attr, name, color) in [
                                    (mob.attrs.based, "Based", egui::Color32::PURPLE),
                                    (mob.attrs.basic, "Basic", egui::Color32::DARK_GRAY),
                                    (mob.attrs.mog_risk, "Mog Risk", egui::Color32::DARK_GREEN),
                                    (mob.attrs.sus, "Sus", egui::Color32::RED),
                                ] {
                                    if attr {
                                        ui.label(apply_brainrot_ui(
                                            RichText::new(name).background_color(color),
                                            player.brainrot,
                                            ui.style(),
                                            FontSelection::Default,
                                            Align::LEFT,
                                        ));
                                    }
                                }
                                fn resist_name(
                                    ty: DamageType,
                                    resist: Resist,
                                ) -> Option<(&'static str, egui::Color32)>
                                {
                                    match (ty, resist) {
                                        (_, Resist::Normal) => None,
                                        (DamageType::Physical, Resist::Weak) => {
                                            Some(("Weak", egui::Color32::DARK_RED))
                                        }
                                        (DamageType::Physical, Resist::Strong) => {
                                            Some(("Unit", egui::Color32::DARK_BLUE))
                                        }
                                        (DamageType::Psychic, Resist::Weak) => {
                                            Some(("Cooked", egui::Color32::DARK_RED))
                                        }
                                        (DamageType::Psychic, Resist::Strong) => {
                                            Some(("Locked in", egui::Color32::DARK_BLUE))
                                        }
                                        (DamageType::Aura, Resist::Weak) => {
                                            Some(("Cringe", egui::Color32::DARK_RED))
                                        }
                                        (DamageType::Aura, Resist::Strong) => {
                                            Some(("Snatched", egui::Color32::DARK_BLUE))
                                        }
                                        (DamageType::Boredom, Resist::Weak) => {
                                            Some(("NPC", egui::Color32::DARK_RED))
                                        }
                                        (DamageType::Boredom, Resist::Strong) => {
                                            Some(("Focused", egui::Color32::DARK_BLUE))
                                        }
                                    }
                                }
                                for damage_type in [
                                    DamageType::Physical,
                                    DamageType::Psychic,
                                    DamageType::Aura,
                                    DamageType::Boredom,
                                ] {
                                    if let Some((name, color)) =
                                        resist_name(damage_type, mob.get_damage_resist(damage_type))
                                    {
                                        ui.label(apply_brainrot_ui(
                                            RichText::new(name).background_color(color),
                                            player.brainrot,
                                            ui.style(),
                                            FontSelection::Default,
                                            Align::LEFT,
                                        ));
                                    }
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
