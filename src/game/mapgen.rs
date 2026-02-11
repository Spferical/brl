use crate::game::{
    Creature, DropsCorpse, Mob, MobBundle, MobSpawner, PLAYER_Z, Player, TILE_Z,
    assets::WorldAssets,
    camera::CameraFollow,
    lighting::Occluder,
    map::{self, MAP_HEIGHT, MapPos},
};
use bevy::{platform::collections::HashMap, prelude::*};
use rand::Rng;

enum TileKind {
    Floor,
    Wall,
    TopSpawner,
    BottomSpawner,
}

#[derive(Component)]
pub struct Tile;

pub(crate) fn gen_map(world: Entity, mut commands: Commands, assets: Res<WorldAssets>) {
    let rng = &mut rand::rng();

    let player_sprite = assets.get_urizen_sprite(104);
    let map_pos = MapPos(IVec2::new(3, MAP_HEIGHT / 2));
    let player = (
        Player,
        Creature {
            hp: 6,
            max_hp: 6,
            faction: 0,
        },
        Name::new("Player"),
        CameraFollow,
        player_sprite,
        map_pos,
        Transform::from_translation(map_pos.to_vec3(PLAYER_Z)),
    );

    let world_rect = rogue_algebra::Rect {
        x1: 0,
        y1: 0,
        x2: map::MAP_WIDTH,
        y2: map::MAP_HEIGHT,
    };

    let mut draft = HashMap::new();
    // Fill in with walls
    for pos in world_rect.expand(1) {
        draft.insert(pos, TileKind::Wall);
    }
    // Dig a tunnel
    for pos in world_rect {
        let tile_kind = if rng.random_bool(0.1) {
            TileKind::Wall
        } else {
            TileKind::Floor
        };
        draft.insert(pos, tile_kind);
    }
    // Spawners on top and bottom
    for pos in world_rect.top_edge() {
        if rng.random_bool(1.0) {
            draft.insert(pos, TileKind::TopSpawner);
        }
    }
    for pos in world_rect.bottom_edge() {
        if rng.random_bool(1.0) {
            draft.insert(pos, TileKind::BottomSpawner);
        }
    }

    let mut tiles = vec![];
    let goblin = MobBundle {
        name: Name::new("Goblin"),
        creature: Creature {
            hp: 1,
            max_hp: 1,
            faction: -1,
        },
        mob: Mob {
            strength: 1,
            ranged: false,
        },
        sprite: assets.get_urizen_sprite(976),
        corpse: DropsCorpse(assets.get_urizen_sprite(1025)),
    };
    let kobold = MobBundle {
        name: Name::new("Kobold"),
        creature: Creature {
            hp: 1,
            max_hp: 1,
            faction: -1,
        },
        mob: Mob {
            strength: 1,
            ranged: true,
        },
        sprite: assets.get_urizen_sprite(1598),
        corpse: DropsCorpse(assets.get_urizen_sprite(1643)),
    };
    let orc = MobBundle {
        name: Name::new("Orc"),
        creature: Creature {
            hp: 2,
            max_hp: 2,
            faction: -1,
        },
        mob: Mob {
            strength: 1,
            ranged: false,
        },
        sprite: assets.get_urizen_sprite(1166),
        corpse: DropsCorpse(assets.get_urizen_sprite(1231)),
    };
    let devil = MobBundle {
        name: Name::new("Devil"),
        creature: Creature {
            hp: 3,
            max_hp: 3,
            faction: -1,
        },
        mob: Mob {
            strength: 2,
            ranged: false,
        },
        sprite: assets.get_urizen_sprite(1390),
        corpse: DropsCorpse(assets.get_urizen_sprite(1437)),
    };
    let dwarf = MobBundle {
        name: Name::new("Hammerdwarf"),
        creature: Creature {
            hp: 3,
            max_hp: 3,
            faction: 1,
        },
        mob: Mob {
            strength: 2,
            ranged: false,
        },
        sprite: assets.get_urizen_colored_sprite(2785, Color::srgb(0.659, 0.173, 0.918)),
        corpse: DropsCorpse(
            assets.get_urizen_colored_sprite(2879, Color::srgb(0.659, 0.173, 0.918)),
        ),
    };
    let dwarf_ranger = MobBundle {
        name: Name::new("Crossbowdwarf"),
        creature: Creature {
            hp: 2,
            max_hp: 2,
            faction: 1,
        },
        mob: Mob {
            strength: 1,
            ranged: true,
        },
        sprite: assets.get_urizen_colored_sprite(2835, Color::srgb(0.478, 0.710, 0.286)),
        corpse: DropsCorpse(
            assets.get_urizen_colored_sprite(2879, Color::lch(0.679, 0.581, 0.1254)),
        ),
    };

    let bottom_spawns = vec![goblin, orc, devil, kobold];
    let top_spawns = vec![dwarf, dwarf_ranger];

    for (rogue_algebra::Pos { x, y }, tile_kind) in draft.into_iter() {
        let map_pos = MapPos(IVec2::new(x, y));
        let transform = Transform::from_translation(map_pos.to_vec3(TILE_Z));
        let mut tile = commands.spawn((Tile, map_pos, transform));
        match tile_kind {
            TileKind::Floor => {
                let sprite_idx = if rng.random_bool(0.1) {
                    rng.random_range(1857..=1859)
                } else {
                    1043
                };
                let sprite = assets.get_urizen_sprite(sprite_idx);
                tile.insert(sprite);
            }
            TileKind::Wall => {
                let sprite = assets.get_urizen_sprite(rng.random_range(0..=1));
                tile.insert((sprite, map::BlocksMovement, Occluder));
            }
            TileKind::TopSpawner => {
                let sprite = assets.get_urizen_sprite(207);
                tile.insert((
                    sprite,
                    MobSpawner {
                        spawns: top_spawns.clone(),
                        odds: 0.004,
                    },
                ));
            }
            TileKind::BottomSpawner => {
                let sprite = assets.get_urizen_sprite(207);
                tile.insert((
                    sprite,
                    MobSpawner {
                        spawns: bottom_spawns.clone(),
                        odds: 0.004,
                    },
                ));
            }
        }
        tiles.push(tile.id());
    }
    let player = commands.spawn(player).id();
    commands
        .entity(world)
        .add_child(player)
        .add_children(&tiles);
}
