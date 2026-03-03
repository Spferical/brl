use crate::game::{
    CORPSE_Z, Corpse, DamageInstance, GameWorld, HIGHLIGHT_Z, PendingDamage, Player, PosToCreature,
    animation::DamageAnimationMessage, assets::WorldAssets, map,
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
    mut damage_animation: MessageWriter<DamageAnimationMessage>,
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
            if let Some(mob) = pos_to_creature.0.get(&delivery.target_pos) {
                if Some(*mob) != player_entity {
                    // kill mob
                    damage.0.push(DamageInstance {
                        entity: *mob,
                        hp: 9999, // enough to kill
                    });
                }
            }

            // Drop off the food delivery
            let map_pos = map::MapPos(delivery.target_pos);
            let transform = Transform::from_translation(map_pos.to_vec3(CORPSE_Z));
            let sprite = assets.get_ascii_sprite('%', Color::srgb(0.5, 0.25, 0.0));
            let drop_id = commands
                .spawn((
                    Corpse,
                    Food {
                        food_idx: delivery.food_idx,
                    },
                    sprite,
                    map_pos,
                    transform,
                ))
                .id();
            commands.entity(world_entity).add_child(drop_id);
            damage_animation.write(DamageAnimationMessage { entity: drop_id });

            to_remove.push(i);
        }
    }
    for i in to_remove.into_iter().rev() {
        active_delivery.deliveries.remove(i);
    }
}

use bevy_egui::EguiContexts;
use bevy_egui::egui;

pub(crate) fn draw_eat_popup(
    mut contexts: EguiContexts,
    player_query: Single<(
        Entity,
        &map::MapPos,
        &mut Player,
        &mut crate::game::Creature,
    )>,
    food_query: Query<(Entity, &map::MapPos, &Food)>,
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q_camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let (_player_entity, player_pos, mut player, mut creature) = player_query.into_inner();
    let (camera, camera_transform) = *q_camera;

    for (food_entity, food_pos, food) in food_query.iter() {
        if food_pos.0 == player_pos.0 {
            let food_item = FOODS[food.food_idx];

            // Get screen position
            let world_pos = player_pos.to_vec3(crate::game::PLAYER_Z);
            let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, world_pos) else {
                continue;
            };

            let Ok(ctx) = contexts.ctx_mut() else {
                return;
            };

            egui::Area::new(egui::Id::new("eat_popup"))
                .fixed_pos(egui::pos2(viewport_pos.x - 100.0, viewport_pos.y - 120.0))
                .show(ctx, |ui| {
                    egui::Frame::window(ui.style())
                        .fill(egui::Color32::from_rgba_premultiplied(30, 30, 30, 240))
                        .show(ui, |ui| {
                            ui.set_width(200.0);
                            ui.vertical_centered(|ui| {
                                ui.label(crate::game::apply_brainrot_ui(
                                    egui::RichText::new(format!("Eat {}? (e)", food_item.name))
                                        .size(18.0)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                    player.brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                                ui.label(crate::game::apply_brainrot_ui(
                                    egui::RichText::new(food_item.effects)
                                        .size(14.0)
                                        .color(egui::Color32::LIGHT_GRAY),
                                    player.brainrot,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                ));
                            });
                        });
                });

            if keyboard_input.just_pressed(KeyCode::KeyE) {
                player.hunger = (player.hunger + food_item.hunger).clamp(0, 100);
                player.strength += food_item.strength;
                creature.hp = (creature.hp + food_item.hp).clamp(0, creature.max_hp);
                commands.entity(food_entity).despawn();
            }
        }
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
