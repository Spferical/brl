//! Interface for targeting abilities.
use bevy::{platform::collections::HashSet, prelude::*};

use crate::game::{
    HIGHLIGHT_Z, Player, PosToCreature,
    assets::WorldAssets,
    input::InputMode,
    map::{MapPos, WalkBlockedMap},
};

#[derive(Resource, Default)]
pub(crate) struct ValidTargets {
    pub(crate) targets: HashSet<MapPos>,
}

pub(crate) fn update_valid_targets(
    mode: Res<InputMode>,
    mut targets: ResMut<ValidTargets>,
    player: Single<(Entity, &MapPos), With<Player>>,
    walk_blocked_map: Res<WalkBlockedMap>,
    pos_to_creature: Res<PosToCreature>,
) {
    let ability = if let InputMode::Targeting(ref ability, ..) = *mode {
        Some(*ability)
    } else {
        None
    };

    let mut new_targets = HashSet::default();
    if let Some(ability) = ability {
        let starts = &[*player.1];
        match ability.target() {
            super::AbilityTarget::ReachableTile { maxdist } => {
                let reachable = |p: MapPos| {
                    p.adjacent().into_iter().filter(|p| {
                        !walk_blocked_map.0.contains(&p.0) && !pos_to_creature.0.contains_key(&p.0)
                    })
                };
                new_targets.extend(rogue_algebra::path::bfs(
                    starts,
                    maxdist as usize,
                    reachable,
                ));
            }
            super::AbilityTarget::NearbyTile { maxdist } => {
                let reachable = |p: MapPos| {
                    p.adjacent()
                        .into_iter()
                        .filter(|p| !walk_blocked_map.0.contains(&p.0))
                };
                new_targets.extend(rogue_algebra::path::bfs(
                    starts,
                    maxdist as usize,
                    reachable,
                ));
            }
            super::AbilityTarget::NearbyMob { maxdist } => {
                let reachable = |p: MapPos| {
                    p.adjacent()
                        .into_iter()
                        .filter(|p| !walk_blocked_map.0.contains(&p.0))
                };
                new_targets.extend(
                    rogue_algebra::path::bfs(starts, maxdist as usize, reachable)
                        .filter(|p| pos_to_creature.0.contains_key(&p.0))
                        .filter(|p| p != player.1),
                );
            }
            super::AbilityTarget::NoTarget => {}
        }
    }

    if targets.targets != new_targets {
        targets.targets = new_targets;
    }
}

#[derive(Component)]
pub(crate) struct ValidTargetIndicator;

pub(crate) fn update_valid_target_indicators(
    mut commands: Commands,
    targets: Res<ValidTargets>,
    indicators: Query<Entity, With<ValidTargetIndicator>>,
    assets: Res<WorldAssets>,
) {
    for entity in indicators {
        commands.entity(entity).despawn();
    }
    let mut sprite = assets.get_urizen_sprite(7908);
    sprite.color = sprite.color.with_alpha(0.25);
    for pos in targets.targets.iter() {
        commands.spawn((
            Name::new("ValidTargetIndicator"),
            ValidTargetIndicator,
            sprite.clone(),
            Transform::from_translation(pos.to_vec3(HIGHLIGHT_Z)),
        ));
    }
}
