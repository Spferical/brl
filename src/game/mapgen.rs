use crate::game::{
    Creature, DropsCorpse, Mob, MobAttrs, MobBundle, PLAYER_Z, Player, Resist, Stairs, TILE_Z,
    assets::WorldAssets,
    camera::CameraFollow,
    lighting::Occluder,
    map::{self, MapPos, Tile},
    signal,
};
use bevy::{platform::collections::HashMap, prelude::*};
use rand::{Rng, seq::IndexedRandom};
use rand_8::SeedableRng;

#[derive(PartialEq, Eq, Clone, Copy)]
enum TileKind {
    Floor,
    Wall,
}

#[derive(Clone, Copy)]
enum MobKind {
    GiantFrog,
    GymBro,
    Influencer,
    Normie,
    Amogus,
}

impl MobKind {
    fn get_bundle(&self, assets: &WorldAssets) -> MobBundle {
        match self {
            MobKind::GiantFrog => MobBundle {
                name: Name::new("Giant Frog"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: -1,
                },
                mob: Mob {
                    melee_damage: 1,
                    ranged: false,
                    attrs: MobAttrs {
                        based: true,
                        aura_resist: Resist::Weak,
                        psychic_resist: Resist::Weak,
                        ..Default::default()
                    },
                },
                sprite: assets.get_ascii_sprite('f', Color::srgb(0.2, 0.8, 0.2)),
                corpse: DropsCorpse(assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2))),
            },
            MobKind::GymBro => MobBundle {
                name: Name::new("Gym Bro"),
                creature: Creature {
                    hp: 4,
                    max_hp: 4,
                    faction: -1,
                },
                mob: Mob {
                    melee_damage: 1,
                    ranged: false,
                    attrs: MobAttrs {
                        physical_resist: Resist::Strong,
                        ..Default::default()
                    },
                },
                sprite: assets.get_ascii_sprite('g', Color::srgb(0.8, 0.3, 0.3)),
                corpse: DropsCorpse(assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2))),
            },
            MobKind::Influencer => MobBundle {
                name: Name::new("Influencer"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: -1,
                },
                mob: Mob {
                    melee_damage: 1,
                    ranged: false,
                    attrs: MobAttrs {
                        mog_risk: true,
                        aura_resist: Resist::Weak,
                        ..Default::default()
                    },
                },
                sprite: assets.get_ascii_sprite('i', Color::srgb(0.2, 0.5, 0.8)),
                corpse: DropsCorpse(assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2))),
            },
            MobKind::Normie => MobBundle {
                name: Name::new("Normie"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: -1,
                },
                mob: Mob {
                    melee_damage: 1,
                    ranged: false,
                    attrs: MobAttrs {
                        basic: true,
                        ..Default::default()
                    },
                },
                sprite: assets.get_ascii_sprite('n', Color::srgb(0.5, 0.5, 0.5)),
                corpse: DropsCorpse(assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2))),
            },
            MobKind::Amogus => MobBundle {
                name: Name::new("Amogus"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: -1,
                },
                mob: Mob {
                    melee_damage: 4,
                    ranged: false,
                    attrs: MobAttrs {
                        sus: true,
                        ..Default::default()
                    },
                },
                sprite: assets.get_ascii_sprite('a', Color::srgb(1.0, 0.1, 0.1)),
                corpse: DropsCorpse(assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2))),
            },
        }
    }
}

pub struct LevelDraft {
    width: u32,
    height: u32,
    start: rogue_algebra::Pos,
    #[allow(unused)]
    end: rogue_algebra::Pos,
    tiles: HashMap<rogue_algebra::Pos, TileKind>,
    mobs: HashMap<rogue_algebra::Pos, MobKind>,
}

impl LevelDraft {
    fn with_walls(mut self) -> Self {
        let min_x = self.tiles.keys().map(|k| k.x).min().expect("Empty level");
        let max_x = self.tiles.keys().map(|k| k.x).max().expect("Empty level");
        let min_y = self.tiles.keys().map(|k| k.y).min().expect("Empty level");
        let max_y = self.tiles.keys().map(|k| k.y).max().expect("Empty level");
        let containing_rect = rogue_algebra::Rect::new(min_x, max_x, min_y, max_y).expand(1);
        // Fill in with walls
        for edge in [
            containing_rect.left_edge(),
            containing_rect.right_edge(),
            containing_rect.top_edge(),
            containing_rect.bottom_edge(),
        ] {
            for pos in edge {
                self.tiles.insert(pos, TileKind::Wall);
            }
        }
        self
    }

    fn sprinkle_mobs(mut self, rng: &mut impl Rng, num_mobs: usize) -> Self {
        let floors = self
            .tiles
            .iter()
            .filter(|(_, tk)| **tk == TileKind::Floor)
            .map(|(pos, _)| pos)
            .copied()
            .collect::<Vec<rogue_algebra::Pos>>();
        for pos in floors.choose_multiple(rng, num_mobs) {
            use MobKind::*;
            let mob_kinds = [GiantFrog, GymBro, Influencer, Normie, Amogus];
            self.mobs.insert(*pos, *mob_kinds.choose(rng).unwrap());
        }
        self
    }
}

fn draft_level_mapgen_rs(
    mut builder: mapgen::MapBuilder,
    rng: &mut rand_8::rngs::StdRng,
) -> LevelDraft {
    use rogue_algebra::Pos;
    let mut tiles = HashMap::<Pos, TileKind>::new();
    let buf = builder.build_with_rng(rng);
    for x in 0..buf.width {
        for y in 0..buf.height {
            let pos = Pos {
                x: x as i32,
                y: y as i32,
            };
            tiles.insert(
                pos,
                if buf.is_walkable(x, y) {
                    TileKind::Floor
                } else {
                    TileKind::Wall
                },
            );
        }
    }

    let start = buf.starting_point.unwrap();
    let start_pos = Pos {
        x: start.x as i32,
        y: start.y as i32,
    };
    assert!(buf.is_walkable(start.x, start.y));

    // Mapgen assumes diagonal movement, which we don't have.
    // So, roll our own unreachable culling and exit detection.
    let dijkstra_map = rogue_algebra::path::build_dijkstra_map(&[start_pos], usize::MAX, |p| {
        rogue_algebra::CARDINALS
            .map(|o| p + o)
            .into_iter()
            .filter(|p| *tiles.get(p).unwrap_or(&TileKind::Wall) == TileKind::Floor)
    });
    let mut furthest_tile = start_pos;
    for (&pos, &dist) in dijkstra_map.iter() {
        if dist == usize::MAX {
            tiles.insert(pos, TileKind::Wall);
        } else if dist > *dijkstra_map.get(&furthest_tile).unwrap() {
            furthest_tile = pos;
        }
    }
    for y in 0..buf.height {
        for x in 0..buf.width {
            let c = match tiles.get(&Pos::new(x as i32, y as i32)) {
                Some(TileKind::Floor) => '.',
                Some(TileKind::Wall) => '#',
                None => ' ',
            };
            print!("{c}");
        }
        println!();
    }

    LevelDraft {
        width: buf.width as u32,
        height: buf.height as u32,
        start: start_pos,
        end: furthest_tile,
        tiles,
        mobs: HashMap::new(),
    }
}

pub(crate) fn spawn_level(
    name: String,
    rng: &mut impl rand::Rng,
    world: Entity,
    commands: &mut Commands,
    assets: &WorldAssets,
    draft: &LevelDraft,
    offset: rogue_algebra::Offset,
) {
    let signal_map = signal::generate_signal_map(
        draft.width as i32,
        draft.height as i32,
        rng.random(),
        IVec2::from(offset),
    );

    let level_entity = commands
        .spawn((
            Name::new(name),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            InheritedVisibility::VISIBLE,
            signal_map,
        ))
        .id();
    commands.entity(world).add_child(level_entity);

    let mut tiles = vec![];
    for (&pos, &tile_kind) in draft.tiles.iter() {
        let pos = pos + offset;
        let map_pos = MapPos(IVec2::from(pos));
        let transform = Transform::from_translation(map_pos.to_vec3(TILE_Z));
        let mut tile = commands.spawn((
            Tile,
            map_pos,
            transform,
            GlobalTransform::IDENTITY,
            InheritedVisibility::VISIBLE,
        ));
        match tile_kind {
            TileKind::Floor => {
                let r = rng.random::<f32>();
                let color = Color::srgb(0.4, 0.4, 0.4);
                let sprite = if r <= 0.1 {
                    assets.get_ascii_sprite('.', color)
                } else if r <= 0.2 {
                    assets.get_ascii_sprite(',', color)
                } else {
                    assets.get_ascii_sprite(' ', color)
                };
                tile.insert(sprite);
                tile.with_children(|parent| {
                    parent.spawn((
                        Sprite {
                            image: assets.get_solid_mask(),
                            color: Color::srgb(0.1, 0.1, 0.1),
                            custom_size: Some(Vec2::new(
                                map::TILE_WIDTH + 1.0,
                                map::TILE_HEIGHT + 1.0,
                            )),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
                    ));
                });
            }
            TileKind::Wall => {
                let sprite = assets.get_ascii_sprite('#', Color::srgb(0.6, 0.6, 0.6));
                tile.insert((sprite, map::BlocksMovement, Occluder));
                tile.with_children(|parent| {
                    parent.spawn((
                        Sprite {
                            image: assets.get_solid_mask(),
                            color: Color::srgb(0.15, 0.15, 0.15),
                            custom_size: Some(Vec2::new(
                                map::TILE_WIDTH + 1.0,
                                map::TILE_HEIGHT + 1.0,
                            )),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
                    ));
                });
            }
        }
        tiles.push(tile.id());
    }
    commands.entity(level_entity).add_children(&tiles);

    for (&pos, &mob_kind) in draft.mobs.iter() {
        let pos = pos + offset;
        let bundle = mob_kind.get_bundle(assets);
        let map_pos = MapPos(IVec2::from(pos));
        let transform = Transform::from_translation(map_pos.to_vec3(PLAYER_Z));
        let new_mob = commands.spawn((bundle, map_pos, transform)).id();
        commands.entity(level_entity).add_child(new_mob);
    }
}

pub(crate) fn spawn_stairs(
    world: Entity,
    commands: &mut Commands,
    assets: &WorldAssets,
    up_pos: rogue_algebra::Pos,
    down_pos: rogue_algebra::Pos,
) {
    let up_pos = MapPos(IVec2::from(up_pos));
    let down_pos = MapPos(IVec2::from(down_pos));
    let color = Color::srgb(0.4, 0.4, 0.4);
    commands.entity(world).with_children(|parent| {
        parent
            .spawn((
                Name::new("Up Stairs"),
                up_pos,
                Transform::from_translation(up_pos.to_vec3(TILE_Z)),
                Stairs {
                    destination: down_pos,
                },
                assets.get_ascii_sprite('<', color),
                GlobalTransform::IDENTITY,
                InheritedVisibility::VISIBLE,
            ))
            .with_children(|p| {
                p.spawn((
                    Sprite {
                        image: assets.get_solid_mask(),
                        color: Color::srgb(0.1, 0.1, 0.1),
                        custom_size: Some(Vec2::new(map::TILE_WIDTH, map::TILE_HEIGHT)),
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
                ));
            });
        parent
            .spawn((
                Name::new("Down Stairs"),
                down_pos,
                Transform::from_translation(down_pos.to_vec3(TILE_Z)),
                Stairs {
                    destination: up_pos,
                },
                assets.get_ascii_sprite('>', color),
                GlobalTransform::IDENTITY,
                InheritedVisibility::VISIBLE,
            ))
            .with_children(|p| {
                p.spawn((
                    Sprite {
                        image: assets.get_solid_mask(),
                        color: Color::srgb(0.1, 0.1, 0.1),
                        custom_size: Some(Vec2::new(map::TILE_WIDTH, map::TILE_HEIGHT)),
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
                ));
            });
    });
}

pub(crate) fn gen_map(world: Entity, mut commands: Commands, assets: Res<WorldAssets>) {
    let rng = &mut rand::rng();

    let mut mapgen_builder = mapgen::MapBuilder::new(80, 50);
    mapgen_builder
        .with(mapgen::SimpleRooms::new())
        .with(mapgen::NearestCorridors::new())
        .with(mapgen::AreaStartingPosition::new(
            mapgen::XStart::LEFT,
            mapgen::YStart::CENTER,
        ))
        .with(mapgen::CullUnreachable::new())
        .with(mapgen::DistantExit::new());
    let level_1_draft = draft_level_mapgen_rs(
        mapgen_builder,
        &mut rand_8::rngs::StdRng::from_seed(rng.random()),
    )
    .with_walls()
    .sprinkle_mobs(rng, 8);

    let mut mapgen_builder = mapgen::MapBuilder::new(80, 50);
    mapgen_builder
        .with(mapgen::DrunkardsWalk::open_halls())
        .with(mapgen::AreaStartingPosition::new(
            mapgen::XStart::LEFT,
            mapgen::YStart::CENTER,
        ))
        .with(mapgen::CullUnreachable::new())
        .with(mapgen::DistantExit::new());
    let level_2_draft = draft_level_mapgen_rs(
        mapgen_builder,
        &mut rand_8::rngs::StdRng::from_seed(rng.random()),
    )
    .with_walls()
    .sprinkle_mobs(rng, 8);

    spawn_level(
        "Level 1".into(),
        rng,
        world,
        &mut commands,
        &assets,
        &level_1_draft,
        rogue_algebra::Offset::ZERO,
    );
    let level_2_offset = rogue_algebra::NORTH * 1000;
    spawn_level(
        "Level 2".into(),
        rng,
        world,
        &mut commands,
        &assets,
        &level_2_draft,
        level_2_offset,
    );

    spawn_stairs(
        world,
        &mut commands,
        &assets,
        level_2_draft.start + level_2_offset,
        level_1_draft.start,
    );

    let player_sprite = assets.get_ascii_sprite('@', Color::WHITE);
    let player_pos = MapPos(IVec2::from(level_1_draft.start));
    let player = (
        Player {
            brainrot: 85,
            hunger: 0,
            money: 0,
            rizz: 10,
            strength: 20,
            boredom: 30,
            signal: 5,
            money_gain_timer: 0.0,
            last_gain_amount: 0,
        },
        Creature {
            hp: 6,
            max_hp: 6,
            faction: 0,
        },
        Name::new("Player"),
        CameraFollow,
        player_sprite,
        player_pos,
        Transform::from_translation(player_pos.to_vec3(PLAYER_Z)),
        GlobalTransform::IDENTITY,
        InheritedVisibility::VISIBLE,
    );

    let player = commands.spawn(player).id();
    commands.entity(world).add_child(player);
}
