use crate::game::DamageType;
use crate::game::{
    CORPSE_Z, Corpse, DamageInstance, GameWorld, HIGHLIGHT_Z, Interactable, InteractionType,
    PendingDamage, Player, PosToCreature, animation::FloatingTextMessage, assets::WorldAssets, map,
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
    pub effects: &'static str,
}

pub const FOODS: [FoodItem; 7] = [
    FoodItem {
        name: "Burrito",
        price: 8,
        hunger: -60,
        hp: 1,
        strength: 3,
        effects: "-60 hunger, +1hp, +3 strength",
    },
    FoodItem {
        name: "Protein Shake",
        price: 20,
        hunger: -5,
        hp: 0,
        strength: 15,
        effects: "-5 hunger, +15 strength",
    },
    FoodItem {
        name: "Health Salad",
        price: 20,
        hunger: -5,
        hp: 6,
        strength: 0,
        effects: "-5 hunger, +6hp",
    },
    FoodItem {
        name: "Chicken Tenders",
        price: 4,
        hunger: -30,
        hp: 0,
        strength: 0,
        effects: "-30 hunger",
    },
    FoodItem {
        name: "Pizza",
        price: 5,
        hunger: -60,
        hp: 0,
        strength: 0,
        effects: "-60 hunger",
    },
    FoodItem {
        name: "Milkshake",
        price: 5,
        hunger: -100,
        hp: -1,
        strength: 0,
        effects: "-100 hunger, -1 hp",
    },
    FoodItem {
        name: "Poke",
        price: 20,
        hunger: -40,
        hp: 0,
        strength: 10,
        effects: "-40 hunger, +10 strength",
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
                    amount: 9999, // enough to kill
                    ty: DamageType::Physical,
                });
            }

            // Drop off the food delivery
            let map_pos = map::MapPos(delivery.target_pos);
            let transform = Transform::from_translation(map_pos.to_vec3(CORPSE_Z));
            let sprite = assets.get_ascii_sprite('%', Color::srgb(0.5, 0.25, 0.0));
            let food = FOODS[delivery.food_idx];
            let drop_id = commands
                .spawn((
                    Corpse {
                        nutrition: 0,
                        name: food.name.to_string(),
                        kind: crate::game::mapgen::MobKind::Normie,
                    },
                    Food {
                        food_idx: delivery.food_idx,
                    },
                    Interactable {
                        action: "Eat".to_string(),
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
            });

            // Chat reaction
            crate::game::chat::queue_food_delivery_message(&mut chat, &streaming_state);

            to_remove.push(i);
        }
    }
    for i in to_remove.into_iter().rev() {
        active_delivery.deliveries.remove(i);
    }
}

pub(crate) fn draw_delivery_indicators(
    active_delivery: Res<ActiveDelivery>,
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
}
