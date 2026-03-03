use crate::game::{
    Creature, DropsCorpse, Mob, MobBundle, PLAYER_Z, Player, TILE_Z,
    assets::WorldAssets,
    camera::CameraFollow,
    lighting::Occluder,
    map::{self, MapPos},
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
}

#[derive(Component)]
pub struct Tile;

#[derive(Component)]
pub struct Stairs {
    pub(crate) destination: MapPos,
}

pub struct LevelDraft {
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
            self.mobs.insert(*pos, MobKind::GiantFrog);
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
    let level_entity = commands
        .spawn((
            Name::new(name),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            InheritedVisibility::VISIBLE,
        ))
        .id();
    commands.entity(world).add_child(level_entity);

    let mut tiles = vec![];
    for (&pos, &tile_kind) in draft.tiles.iter() {
        let pos = pos + offset;
        let map_pos = MapPos(IVec2::from(pos));
        let transform = Transform::from_translation(map_pos.to_vec3(TILE_Z));
        let mut tile = commands.spawn((Tile, map_pos, transform));
        match tile_kind {
            TileKind::Floor => {
                let r = rng.random::<f32>();
                let sprite = if r <= 0.1 {
                    assets.get_ascii_sprite('.', Color::srgb(0.4, 0.4, 0.4))
                } else if r <= 0.2 {
                    assets.get_ascii_sprite(',', Color::srgb(0.4, 0.4, 0.4))
                } else {
                    assets.get_ascii_sprite(' ', Color::srgb(0.3, 0.3, 0.3))
                };
                tile.insert(sprite);
            }
            TileKind::Wall => {
                let sprite = assets.get_ascii_sprite('#', Color::srgb(0.6, 0.6, 0.6));
                tile.insert((sprite, map::BlocksMovement, Occluder));
            }
        }
        tiles.push(tile.id());
    }
    commands.entity(level_entity).add_children(&tiles);

    for (&pos, &mob_kind) in draft.mobs.iter() {
        let pos = pos + offset;
        let bundle = match mob_kind {
            MobKind::GiantFrog => MobBundle {
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
                sprite: assets.get_ascii_sprite('g', Color::srgb(0.2, 0.8, 0.2)),
                corpse: DropsCorpse(assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2))),
            },
        };
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
        parent.spawn((
            Tile,
            Name::new("Up Stairs"),
            up_pos,
            Transform::from_translation(up_pos.to_vec3(TILE_Z)),
            Stairs {
                destination: down_pos,
            },
            assets.get_ascii_sprite('<', color),
        ));
        parent.spawn((
            Tile,
            Name::new("Down Stairs"),
            down_pos,
            Transform::from_translation(down_pos.to_vec3(TILE_Z)),
            Stairs {
                destination: up_pos,
            },
            assets.get_ascii_sprite('>', color),
        ));
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
    );

    let player = commands.spawn(player).id();
    commands.entity(world).add_child(player);
}
