use crate::game::DamageType;
use crate::game::{
    CORPSE_Z, Corpse, DamageInstance, DespawnAfterTurns, GameWorld, HIGHLIGHT_Z, Interactable,
    InteractionType, PendingDamage, Player, PosToCreature, animation::FloatingTextMessage,
    assets::WorldAssets, map,
};
use bevy::prelude::*;
use rand::Rng;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DungeonDashScreen {
    #[default]
    RoleSelection,
    Menu,
    Checkout,
    JobOffer,
}

#[derive(Resource, Default)]
pub struct DungeonDashState {
    pub selected_food: Option<usize>,
    pub tip_percentage: u32,
    pub checkout_start_time: f64,
    pub job_target: Option<map::MapPos>,
    pub job_distance: i32,
    pub active_job_turns: Option<u32>,
    pub active_job_amount: Option<i32>,
    pub job_turns_at_completion: Option<u32>,
    pub failed_job_turns: Option<u32>,
    pub cancelled_job_turns: Option<u32>,
    pub spawn_customer_at: Option<map::MapPos>,
    pub customer_entity: Option<Entity>,
    pub dropped_food_entity: Option<Entity>,
    pub deliveries_this_level: u32,
    pub initial_mobs: u32,
    pub current_mobs: u32,
}

impl DungeonDashState {
    pub fn reset_job(&mut self) {
        self.active_job_turns = None;
        self.active_job_amount = None;
        self.job_turns_at_completion = None;
        self.job_target = None;
        self.dropped_food_entity = None;
        self.customer_entity = None;
    }

    pub fn start_job(&mut self, turn_limit: u32, amount: i32) {
        self.active_job_turns = Some(turn_limit);
        self.active_job_amount = Some(amount);
        self.spawn_customer_at = self.job_target;
        self.cancelled_job_turns = None;
        self.failed_job_turns = None;
        self.job_turns_at_completion = None;
    }

    pub fn fail_job(&mut self, commands: &mut Commands) {
        self.reset_job();
        if let Some(food_entity) = self.dropped_food_entity.take() {
            if let Ok(mut entity) = commands.get_entity(food_entity) {
                entity.despawn();
            }
        }
        self.failed_job_turns = Some(10);
    }

    pub fn decrement_timers(&mut self) {
        if let Some(mut turns) = self.active_job_turns
            && self.dropped_food_entity.is_none()
        {
            if turns > 0 {
                turns -= 1;
                self.active_job_turns = Some(turns);
            }
        }

        if let Some(mut f_turns) = self.failed_job_turns {
            if f_turns > 0 {
                f_turns -= 1;
                self.failed_job_turns = Some(f_turns);
            }
            if f_turns == 0 {
                self.failed_job_turns = None;
            }
        }

        if let Some(mut c_turns) = self.cancelled_job_turns {
            if c_turns > 0 {
                c_turns -= 1;
                self.cancelled_job_turns = Some(c_turns);
            }
            if c_turns == 0 {
                self.cancelled_job_turns = None;
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Delivery {
    pub turns_remaining: u32,
    pub target_pos: IVec2,
    pub food_idx: usize,
}

#[derive(Resource, Default)]
pub struct ActiveDelivery {
    pub deliveries: Vec<Delivery>,
}

#[derive(Clone, Copy)]
pub struct FoodItem {
    pub name: &'static str,
    pub price: i32,
    pub hunger: i32,
    pub hp: i32,
    pub strength: i32,
    pub rizz: i32,
    pub effects: &'static str,
}

pub const FOODS: [FoodItem; 11] = [
    FoodItem {
        name: "Burrito",
        price: 8,
        hunger: -60,
        hp: 1,
        strength: 3,
        rizz: 0,
        effects: "-60 hunger, +1hp, +3 strength",
    },
    FoodItem {
        name: "Protein Shake",
        price: 20,
        hunger: -5,
        hp: 0,
        strength: 15,
        rizz: 0,
        effects: "-5 hunger, +15 strength",
    },
    FoodItem {
        name: "Health Salad",
        price: 20,
        hunger: -5,
        hp: 6,
        strength: 0,
        rizz: 0,
        effects: "-5 hunger, +6hp",
    },
    FoodItem {
        name: "Chicken Tenders",
        price: 4,
        hunger: -30,
        hp: 0,
        strength: 0,
        rizz: 0,
        effects: "-30 hunger",
    },
    FoodItem {
        name: "Pizza",
        price: 5,
        hunger: -60,
        hp: 0,
        strength: 0,
        rizz: 0,
        effects: "-60 hunger",
    },
    FoodItem {
        name: "Milkshake",
        price: 5,
        hunger: -100,
        hp: -1,
        strength: 0,
        rizz: 0,
        effects: "-100 hunger, -1 hp",
    },
    FoodItem {
        name: "Poke",
        price: 20,
        hunger: -40,
        hp: 0,
        strength: 10,
        rizz: 0,
        effects: "-40 hunger, +10 strength",
    },
    FoodItem {
        name: "Essentials Hoodie",
        price: 80,
        hunger: 0,
        hp: 0,
        strength: 0,
        rizz: 10,
        effects: "+10 rizz",
    },
    FoodItem {
        name: "Panda Dunks",
        price: 95,
        hunger: 0,
        hp: 0,
        strength: 0,
        rizz: 15,
        effects: "+15 rizz",
    },
    FoodItem {
        name: "AirPods Max",
        price: 250,
        hunger: 0,
        hp: 0,
        strength: 0,
        rizz: 25,
        effects: "+25 rizz",
    },
    FoodItem {
        name: "Stanley Cup",
        price: 60,
        hunger: 0,
        hp: 0,
        strength: 0,
        rizz: 5,
        effects: "+5 rizz",
    },
];

#[derive(Component)]
pub struct Food {
    pub food_idx: usize,
}

pub fn spawn_food(
    commands: &mut Commands,
    assets: &WorldAssets,
    food_idx: usize,
    map_pos: map::MapPos,
) -> Entity {
    let transform = Transform::from_translation(map_pos.to_vec3(CORPSE_Z));
    let sprite = assets.get_ascii_sprite('%', Color::srgb(0.5, 0.25, 0.0));
    let food = FOODS[food_idx];
    let action = if food.rizz > 0 {
        "Equip".to_string()
    } else {
        "Eat".to_string()
    };

    commands
        .spawn((
            Corpse {
                nutrition: 0,
                name: food.name.to_string(),
                kind: crate::game::mapgen::MobKind::Normie,
            },
            DespawnAfterTurns(50),
            Food { food_idx },
            Interactable {
                action,
                description: None,
                kind: InteractionType::Eat,
            },
            sprite,
            map_pos,
            transform,
        ))
        .id()
}

pub(crate) fn process_deliveries(
    mut commands: Commands,
    world: Single<Entity, With<GameWorld>>,
    assets: Res<WorldAssets>,
    mut active_delivery: ResMut<ActiveDelivery>,
    pos_to_creature: Res<PosToCreature>,
    players: Query<Entity, With<Player>>,
    mut damage: ResMut<PendingDamage>,
    mut floating_text: MessageWriter<FloatingTextMessage>,
    mut chat: ResMut<crate::game::chat::ChatHistory>,
    streaming_state: Res<crate::game::chat::StreamingState>,
) {
    let player_entity = players.iter().next();
    let world_entity = world.into_inner();

    let mut to_remove = Vec::new();
    for (i, delivery) in active_delivery.deliveries.iter_mut().enumerate() {
        if delivery.turns_remaining > 0 {
            delivery.turns_remaining -= 1;
        }

        if delivery.turns_remaining == 0 {
            // delivery arrived
            if let Some(mob) = pos_to_creature.0.get(&delivery.target_pos)
                && Some(*mob) != player_entity
            {
                // kill mob
                damage.0.push(DamageInstance {
                    entity: *mob,
                    attacker: player_entity,
                    amount: 9999, // enough to kill
                    ty: DamageType::Physical,
                });
            }

            // Drop off the food delivery
            let map_pos = map::MapPos(delivery.target_pos);
            let drop_id = spawn_food(&mut commands, &assets, delivery.food_idx, map_pos);
            commands.entity(world_entity).add_child(drop_id);
            floating_text.write(FloatingTextMessage {
                entity: Some(drop_id),
                world_pos: None,
                text: format!("{} Delivered!", FOODS[delivery.food_idx].name),
                color: Color::srgb(1.0, 1.0, 1.0),
                ..default()
            });

            // Chat reaction
            crate::game::chat::queue_food_delivery_message(
                &mut chat,
                &streaming_state,
                delivery.food_idx,
            );

            to_remove.push(i);
        }
    }
    for i in to_remove.into_iter().rev() {
        active_delivery.deliveries.remove(i);
    }
}

pub(crate) fn draw_delivery_indicators(
    active_delivery: Res<ActiveDelivery>,
    dd_selection: Res<DungeonDashState>,
    q_camera: Single<(&Camera, &GlobalTransform), With<crate::PrimaryCamera>>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    for delivery in active_delivery.deliveries.iter() {
        if delivery.turns_remaining <= 5 {
            let t = time.elapsed_secs() * 10.0; // blink speed
            if t.sin() > 0.0 {
                let map_pos = map::MapPos(delivery.target_pos);
                let center = map_pos.to_vec3(HIGHLIGHT_Z).truncate();
                gizmos.circle_2d(center, map::TILE_WIDTH / 2.0, Color::srgb(1.0, 0.0, 0.0));
                let offset = map::TILE_WIDTH / 4.0;
                gizmos.line_2d(
                    center + Vec2::new(-offset, -offset),
                    center + Vec2::new(offset, offset),
                    Color::WHITE,
                );
                gizmos.line_2d(
                    center + Vec2::new(-offset, offset),
                    center + Vec2::new(offset, -offset),
                    Color::WHITE,
                );
            }
        }
    }

    if let Some(target) = dd_selection.job_target {
        let map_pos = target;
        let center = map_pos.to_vec3(HIGHLIGHT_Z).truncate();

        let t = time.elapsed_secs() * 5.0; // slower blink
        if t.sin() > 0.0 {
            gizmos.circle_2d(center, map::TILE_WIDTH / 2.0, Color::srgb(0.0, 1.0, 0.0));
            let offset = map::TILE_WIDTH / 4.0;
            gizmos.line_2d(
                center + Vec2::new(-offset, -offset),
                center + Vec2::new(offset, offset),
                Color::WHITE,
            );
            gizmos.line_2d(
                center + Vec2::new(-offset, offset),
                center + Vec2::new(offset, -offset),
                Color::WHITE,
            );
        }

        // Draw offscreen arrow
        let (camera, camera_transform) = *q_camera;
        if let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, center.extend(0.0)) {
            let Some(viewport_size) = camera.logical_viewport_size() else {
                return;
            };

            // Check if point is outside the viewport
            if viewport_pos.x < 0.0
                || viewport_pos.x > viewport_size.x
                || viewport_pos.y < 0.0
                || viewport_pos.y > viewport_size.y
            {
                let viewport_center = viewport_size / 2.0;
                let dir_to_target = (viewport_pos - viewport_center).normalize();

                // Direction in world space for the arrow rotation
                // We use NDC to get world direction as viewport is y-down
                let mut dir_world = Vec2::ZERO;
                if let Some(ndc) = camera.world_to_ndc(camera_transform, center.extend(0.0)) {
                    dir_world = ndc.truncate().normalize();
                }

                // Raycast to find intersection with screen edge in viewport space
                let mut edge_viewport = viewport_center;

                let dx = dir_to_target.x;
                let dy = dir_to_target.y;

                let t_x = if dx > 0.0 {
                    (viewport_size.x - viewport_center.x) / dx
                } else if dx < 0.0 {
                    (0.0 - viewport_center.x) / dx
                } else {
                    f32::MAX
                };

                let t_y = if dy > 0.0 {
                    (viewport_size.y - viewport_center.y) / dy
                } else if dy < 0.0 {
                    (0.0 - viewport_center.y) / dy
                } else {
                    f32::MAX
                };

                let t = t_x.min(t_y);
                edge_viewport += dir_to_target * t * 0.95; // 5% margin

                // Convert back to world space for gizmos
                if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, edge_viewport)
                {
                    let arrow_center = world_pos;
                    let arrow_length = 30.0;
                    let arrow_width = 15.0;

                    let dir = dir_world;
                    let back = arrow_center - dir * arrow_length;
                    let left =
                        arrow_center - dir * arrow_width + Vec2::new(-dir.y, dir.x) * arrow_width;
                    let right =
                        arrow_center - dir * arrow_width + Vec2::new(dir.y, -dir.x) * arrow_width;

                    gizmos.line_2d(back, arrow_center, Color::srgb(0.0, 1.0, 0.0));
                    gizmos.line_2d(left, arrow_center, Color::srgb(0.0, 1.0, 0.0));
                    gizmos.line_2d(right, arrow_center, Color::srgb(0.0, 1.0, 0.0));
                }
            }
        }
    }
}

pub fn process_dungeon_dash_jobs(
    mut dd_state: ResMut<DungeonDashState>,
    mut commands: Commands,
    world: Single<Entity, With<crate::game::GameWorld>>,
    assets: Res<crate::game::assets::WorldAssets>,
    mut floating_text: MessageWriter<crate::game::animation::FloatingTextMessage>,
    player_query: Single<(Entity, &crate::game::map::MapPos, &mut crate::game::Player)>,
    walk_blocked_map: Res<crate::game::map::WalkBlockedMap>,
    mut mob_query: Query<(
        &crate::game::map::MapPos,
        &mut crate::game::Mob,
        &crate::game::Creature,
    )>,
    food_query: Query<&crate::game::map::MapPos, With<crate::game::delivery::Food>>,
) {
    let (player_entity, player_pos, mut player) = player_query.into_inner();

    // Check for customer death
    if let Some(customer_entity) = dd_state.customer_entity {
        let is_dead = match mob_query.get(customer_entity) {
            Ok((_, _, creature)) => creature.is_dead(),
            Err(_) => true,
        };
        if dd_state.active_job_turns.is_some() && is_dead {
            dd_state.fail_job(&mut commands);
            dd_state.cancelled_job_turns = Some(10);
            return;
        }
    }

    // Spawn new customer if needed
    if let Some(target) = dd_state.spawn_customer_at.take() {
        let customer = crate::game::spawn::spawn_mob(
            &mut commands,
            *world,
            target,
            crate::game::mapgen::MobKind::FriendlyNormie,
            &assets,
        );
        dd_state.customer_entity = Some(customer);
    }

    if let Some(food_entity) = dd_state.dropped_food_entity {
        if let Some(customer_entity) = dd_state.customer_entity {
            if let Ok(food_pos) = food_query.get(food_entity) {
                if let Ok((customer_pos, mut mob, _creature)) = mob_query.get_mut(customer_entity) {
                    if customer_pos.0 == food_pos.0 {
                        // Picked up!
                        commands.entity(food_entity).despawn();

                        let dist = dd_state.job_distance as f32;
                        let max_amount = dd_state.active_job_amount.unwrap_or(0) as f32;
                        let max_turns = (dist * 1.5) as u32;
                        let turns_left = dd_state
                            .job_turns_at_completion
                            .or(dd_state.active_job_turns)
                            .unwrap_or(0);
                        let turns_taken = max_turns.saturating_sub(turns_left);

                        let t1 = dist * 1.1;
                        let t2 = dist * 1.5;
                        let min_amount = dist * 1.0;

                        let payout = if (turns_taken as f32) <= t1 {
                            max_amount
                        } else {
                            let t = ((turns_taken as f32) - t1) / (t2 - t1);
                            max_amount - t * (max_amount - min_amount)
                        }
                        .max(0.0)
                        .round() as i32;

                        player.money += payout + 1; // Payout + $1 tip

                        floating_text.write(crate::game::animation::FloatingTextMessage {
                            entity: Some(player_entity),
                            world_pos: None,
                            text: format!("+${} Payout", payout),
                            color: Color::srgb(0.0, 1.0, 0.0),
                            ..default()
                        });

                        let mut tip_pos = customer_pos.to_vec3(crate::game::PLAYER_Z);
                        tip_pos.y += 16.0;
                        floating_text.write(crate::game::animation::FloatingTextMessage {
                            entity: None,
                            world_pos: Some(tip_pos),
                            text: "+$1 Tip".to_string(),
                            color: Color::srgb(0.5, 1.0, 0.5),
                            delay: 1.0,
                            ..default()
                        });

                        dd_state.reset_job();
                        dd_state.deliveries_this_level += 1;

                        // make customer walk away randomly
                        mob.destination = Some(customer_pos.0 + bevy::math::IVec2::new(10, 10));
                        return;
                    } else {
                        mob.destination = Some(food_pos.0);
                    }
                }
            }
        }
    } else if let Some(target) = dd_state.job_target {
        if target.0 == player_pos.0 {
            // Drop off the food
            let mut rng = rand::rng();
            let food_idx = rng.random_range(0..FOODS.len());

            let mut drop_pos = *player_pos;
            for adj in player_pos.adjacent() {
                if !walk_blocked_map.0.contains(&adj.0) {
                    drop_pos = adj;
                    break;
                }
            }

            let drop_id = spawn_food(&mut commands, &assets, food_idx, drop_pos);
            commands.entity(*world).add_child(drop_id);
            dd_state.dropped_food_entity = Some(drop_id);
            dd_state.job_turns_at_completion = dd_state.active_job_turns;
            dd_state.job_target = None;
        }
    }

    // Handle job countdown and other timers
    let turns_before = dd_state.active_job_turns;
    dd_state.decrement_timers();

    if let Some(0) = dd_state.active_job_turns {
        if turns_before != Some(0) {
            player.money -= 10;
            floating_text.write(crate::game::animation::FloatingTextMessage {
                entity: Some(player_entity),
                world_pos: None,
                text: "-$10 Failed Delivery".to_string(),
                color: Color::srgb(1.0, 0.0, 0.0),
                ..default()
            });

            dd_state.fail_job(&mut commands);
        }
    }
}

pub fn update_current_mobs(
    mut dd_state: ResMut<DungeonDashState>,
    map_info: Res<crate::game::mapgen::MapInfo>,
    player_query: Single<&crate::game::map::MapPos, With<crate::game::Player>>,
    mob_query: Query<
        &crate::game::map::MapPos,
        (With<crate::game::Mob>, Without<crate::game::Player>),
    >,
) {
    let player_pos = player_query.into_inner();
    if let Some(level) = map_info.get_level(*player_pos) {
        let count = mob_query
            .iter()
            .filter(|&pos| level.rect.contains(rogue_algebra::Pos::from(pos.0)))
            .count();
        dd_state.current_mobs = count as u32;
    }
}
