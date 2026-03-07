use crate::game::DamageType;
use crate::game::{
    CORPSE_Z, Corpse, DamageInstance, DespawnAfterTurns, GameWorld, HIGHLIGHT_Z, Interactable,
    InteractionType, PendingDamage, Player, PosToCreature, animation::FloatingTextMessage,
    assets::WorldAssets, map,
};
use bevy::prelude::*;

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
            let transform = Transform::from_translation(map_pos.to_vec3(CORPSE_Z));
            let sprite = assets.get_ascii_sprite('%', Color::srgb(0.5, 0.25, 0.0));
            let food = FOODS[delivery.food_idx];
            let action = if food.rizz > 0 {
                "Equip".to_string()
            } else {
                "Eat".to_string()
            };
            let drop_id = commands
                .spawn((
                    Corpse {
                        nutrition: 0,
                        name: food.name.to_string(),
                        kind: crate::game::mapgen::MobKind::Normie,
                    },
                    DespawnAfterTurns(50),
                    Food {
                        food_idx: delivery.food_idx,
                    },
                    Interactable {
                        action,
                        description: None,
                        kind: InteractionType::Eat,
                    },
                    sprite,
                    map_pos,
                    transform,
                ))
                .id();
            commands.entity(world_entity).add_child(drop_id);
            floating_text.write(FloatingTextMessage {
                entity: Some(drop_id),
                world_pos: None,
                text: format!("{} Delivered!", food.name),
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
    dd_selection: Res<crate::game::mobile_apps::DungeonDashSelection>,
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
        if let Some(ndc) = camera.world_to_ndc(camera_transform, center.extend(0.0)) {
            // Check if point is outside the screen in NDC space (-1 to 1)
            if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 {
                // Direction from center of screen to target in NDC
                let dir = ndc.truncate().normalize();

                // Raycast to find intersection with screen edge in NDC space
                let mut edge_ndc = dir;

                // Scale until we hit an edge
                let scale_x = if dir.x.abs() > 0.0 {
                    1.0 / dir.x.abs()
                } else {
                    f32::MAX
                };
                let scale_y = if dir.y.abs() > 0.0 {
                    1.0 / dir.y.abs()
                } else {
                    f32::MAX
                };
                let scale = scale_x.min(scale_y);

                edge_ndc *= scale;
                // Pull back slightly so arrow isn't cut off
                edge_ndc *= 0.9;

                // Convert back to world space for gizmos
                if let Some(world_pos) = camera.ndc_to_world(camera_transform, edge_ndc.extend(0.0))
                {
                    let arrow_center = world_pos.truncate();
                    let arrow_length = 30.0;
                    let arrow_width = 15.0;

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
