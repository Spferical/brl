use crate::game::{
    Creature, DropsCorpse, Interactable, InteractionType, Mob, MobAttrs, MobBundle, PLAYER_Z,
    Player, Resist, Stairs, TILE_Z,
    assets::WorldAssets,
    camera::CameraFollow,
    lighting::Occluder,
    map::{self, MapPos, Tile},
    signal,
};
use bevy::{platform::collections::HashMap, prelude::*};
use rand::{Rng, seq::IndexedRandom};
use rand_8::SeedableRng;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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
    Capybara,
}

const MOBS: &[MobKind] = &[
    MobKind::GiantFrog,
    MobKind::GymBro,
    MobKind::Influencer,
    MobKind::Normie,
    MobKind::Amogus,
    MobKind::Capybara,
];

impl MobKind {
    fn get_bundle(&self, assets: &WorldAssets) -> MobBundle {
        match self {
            MobKind::Capybara => MobBundle {
                name: Name::new("Capybara"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: -1,
                },
                mob: Mob {
                    melee_damage: 1,
                    target: None,
                    ranged: false,
                    attrs: MobAttrs {
                        psychic_resist: Resist::Weak,
                        boredom_resist: Resist::Strong,
                        ..Default::default()
                    },
                },
                sprite: assets.get_ascii_sprite('c', Color::srgb(0.5, 0.3, 0.3)),
                corpse: DropsCorpse(assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2))),
            },

            MobKind::GiantFrog => MobBundle {
                name: Name::new("Giant Frog"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: -1,
                },
                mob: Mob {
                    melee_damage: 1,
                    target: None,
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
                    target: None,
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
                    target: None,
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
                    target: None,
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
                    target: None,
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
    start: rogue_algebra::Pos,
    #[allow(unused)]
    end: rogue_algebra::Pos,
    tiles: HashMap<rogue_algebra::Pos, TileKind>,
    mobs: HashMap<rogue_algebra::Pos, MobKind>,
}

impl LevelDraft {
    fn get_containing_rect(&self) -> rogue_algebra::Rect {
        let min_x = self.tiles.keys().map(|k| k.x).min().expect("Empty level");
        let max_x = self.tiles.keys().map(|k| k.x).max().expect("Empty level");
        let min_y = self.tiles.keys().map(|k| k.y).min().expect("Empty level");
        let max_y = self.tiles.keys().map(|k| k.y).max().expect("Empty level");
        rogue_algebra::Rect::new(min_x, max_x, min_y, max_y)
    }

    fn get_random_floors(&self, n: usize, rng: &mut impl Rng) -> Vec<rogue_algebra::Pos> {
        let all_floors = self
            .tiles
            .iter()
            .filter(|(_p, t)| **t == TileKind::Floor)
            .map(|(p, _t)| *p)
            .collect::<Vec<_>>();
        if all_floors.len() < n {
            panic!("get_random_floors: not enough floors");
        }
        all_floors.choose_multiple(rng, n).copied().collect()
    }

    #[allow(unused)]
    fn fill_rect(&mut self, rect: rogue_algebra::Rect, tk: TileKind) {
        for p in rect {
            self.tiles.insert(p, tk);
        }
    }

    fn with_walls(mut self) -> Self {
        let containing_rect = self.get_containing_rect().expand(1);
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
            self.mobs.insert(*pos, *MOBS.choose(rng).unwrap());
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

#[derive(Debug, Clone, Copy)]
pub struct CarveRoomOpts {
    max_width: i32,
    max_height: i32,
    min_width: i32,
    min_height: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct BspSplitOpts {
    max_width: i32,
    max_height: i32,
    min_width: i32,
    min_height: i32,
}

impl From<CarveRoomOpts> for BspSplitOpts {
    fn from(opts: CarveRoomOpts) -> Self {
        Self {
            max_width: opts.max_width,
            max_height: opts.max_height,
            min_width: opts.min_width,
            min_height: opts.min_height,
        }
    }
}

fn get_connecting_wall(
    room1: rogue_algebra::Rect,
    room2: rogue_algebra::Rect,
) -> Option<rogue_algebra::Rect> {
    // one-tile-wall between them
    for (room1, room2) in &[(room1, room2), (room2, room1)] {
        // room2 right of room1
        if room1.x2 + 2 == room2.x1 {
            let y1 = room1.y1.max(room2.y1);
            let y2 = room1.y2.min(room2.y2);
            if y1 <= y2 {
                return Some(rogue_algebra::Rect {
                    x1: room1.x2 + 1,
                    x2: room1.x2 + 1,
                    y1,
                    y2,
                });
            }
        }
        // room2 under room1
        if room1.y2 + 2 == room2.y1 {
            let x1 = room1.x1.max(room2.x1);
            let x2 = room1.x2.min(room2.x2);
            if x1 <= x2 {
                return Some(rogue_algebra::Rect {
                    x1,
                    x2,
                    y1: room1.y2 + 1,
                    y2: room1.y2 + 1,
                });
            }
        }
    }
    None
}

#[derive(Debug)]
pub enum BspTree {
    Split(Box<BspTree>, Box<BspTree>),
    Room(rogue_algebra::Rect),
}

impl BspTree {
    fn into_room_graph(self) -> RoomGraph {
        match self {
            BspTree::Room(rect) => {
                let mut graph = RoomGraph::new();
                graph.add_room(rect);
                graph
            }
            BspTree::Split(tree1, tree2) => {
                let mut rooms1 = tree1.into_room_graph();
                let rooms2 = tree2.into_room_graph();
                // now figure out how to bridge the trees
                rooms1.extend_bridged(rooms2);
                rooms1
            }
        }
    }
}

struct RoomGraph {
    pub room_adj: HashMap<rogue_algebra::Rect, Vec<rogue_algebra::Rect>>,
}

#[allow(unused)]
impl RoomGraph {
    fn get_adj(&self, rect: rogue_algebra::Rect) -> Option<&[rogue_algebra::Rect]> {
        self.room_adj.get(&rect).map(Vec::as_slice)
    }
    fn choose(&self, rng: &mut impl Rng) -> Option<rogue_algebra::Rect> {
        if self.room_adj.is_empty() {
            return None;
        }
        let idx = rng.random_range(0..self.room_adj.len());
        self.room_adj.keys().nth(idx).copied()
    }
    fn len(&self) -> usize {
        self.room_adj.len()
    }
    fn remove_room(&mut self, rect: rogue_algebra::Rect) {
        self.room_adj.retain(|r, _| *r != rect);
    }
    fn find_spatially_adjacent(&self, rect: rogue_algebra::Rect) -> Option<rogue_algebra::Rect> {
        for room in self.room_adj.keys() {
            if let Some(_wall) = get_connecting_wall(rect, *room) {
                return Some(*room);
            }
        }
        None
    }
    fn extend_bridged(&mut self, mut other: RoomGraph) {
        let mut bridged = false;
        'loop1: for (room1, ref mut adj1) in &mut self.room_adj {
            for (room2, ref mut adj2) in &mut other.room_adj {
                if get_connecting_wall(*room1, *room2).is_some() {
                    bridged = true;
                    adj1.push(*room2);
                    adj2.push(*room1);
                    break 'loop1;
                }
            }
        }
        assert!(bridged);
        self.room_adj.extend(other.room_adj);
    }
    fn new() -> Self {
        Self {
            room_adj: HashMap::new(),
        }
    }
    fn add_room(&mut self, room: rogue_algebra::Rect) {
        self.room_adj.insert(room, vec![]);
    }
    fn add_connection(&mut self, room1: rogue_algebra::Rect, room2: rogue_algebra::Rect) {
        assert!(get_connecting_wall(room1, room2).is_some());
        assert!(self.room_adj.contains_key(&room1));
        assert!(self.room_adj.contains_key(&room2));
        self.room_adj.get_mut(&room2).unwrap().push(room1);
        self.room_adj.get_mut(&room1).unwrap().push(room2);
    }
    fn add_connection_oneway(&mut self, room1: rogue_algebra::Rect, room2: rogue_algebra::Rect) {
        assert!(get_connecting_wall(room1, room2).is_some());
        assert!(self.room_adj.contains_key(&room1));
        self.room_adj.get_mut(&room1).unwrap().push(room2);
    }

    fn iter(&'_ self) -> impl Iterator<Item = rogue_algebra::Rect> + '_ {
        self.room_adj.keys().copied()
    }
}

// returns (rooms, walls between connected rooms in the bsp tree)
pub fn gen_bsp_tree(rect: rogue_algebra::Rect, opts: BspSplitOpts, rng: &mut impl Rng) -> BspTree {
    #[derive(Clone, Copy, Debug)]
    enum Split {
        X,
        Y,
        None,
    }
    assert!(opts.min_width * 2 < opts.max_width);
    assert!(opts.min_height * 2 < opts.max_height);
    let too_wide = (rect.x2 - rect.x1) > opts.max_width;
    let too_tall = (rect.y2 - rect.y1) > opts.max_height;
    let split = match (too_wide, too_tall) {
        (true, true) => *[Split::X, Split::Y].choose(rng).unwrap(),
        (true, false) => Split::X,
        (false, true) => Split::Y,
        _ => Split::None,
    };
    match split {
        Split::X => {
            let split_x =
                rng.random_range(rect.x1 + opts.min_width + 1..(rect.x2 - opts.min_width - 1));
            let left = rogue_algebra::Rect::new(rect.x1, split_x - 1, rect.y1, rect.y2);
            let right = rogue_algebra::Rect::new(split_x + 1, rect.x2, rect.y1, rect.y2);
            BspTree::Split(
                Box::new(gen_bsp_tree(left, opts, rng)),
                Box::new(gen_bsp_tree(right, opts, rng)),
            )
        }
        Split::Y => {
            let split_y =
                rng.random_range(rect.y1 + opts.min_height + 1..(rect.y2 - opts.min_height));
            let top = rogue_algebra::Rect::new(rect.x1, rect.x2, rect.y1, split_y - 1);
            let bottom = rogue_algebra::Rect::new(rect.x1, rect.x2, split_y + 1, rect.y2);
            BspTree::Split(
                Box::new(gen_bsp_tree(top, opts, rng)),
                Box::new(gen_bsp_tree(bottom, opts, rng)),
            )
        }
        Split::None => BspTree::Room(rect),
    }
}

fn gen_offices(rng: &mut impl Rng, rect: rogue_algebra::Rect) -> LevelDraft {
    let max_width = rng.random_range(4..=rect.width().min(8));
    let min_width = max_width / 2 - 1;
    let max_height = rng.random_range(4..=rect.width().min(8));
    let min_height = max_height / 2 - 1;
    let bsp_opts = CarveRoomOpts {
        max_width,
        max_height,
        min_width,
        min_height,
    };
    let tree = gen_bsp_tree(rect, bsp_opts.into(), rng);
    let room_graph = tree.into_room_graph();
    let rooms = room_graph.iter().collect::<Vec<rogue_algebra::Rect>>();
    let mut doors = vec![];
    for room in room_graph.iter() {
        for room2 in room_graph.get_adj(room).into_iter().flatten().copied() {
            if room.topleft() < room2.topleft()
                && let Some(wall) = get_connecting_wall(room, room2)
            {
                doors.push(wall.choose(rng));
            }
        }
    }
    // Add doors for extra loops.
    for _ in 0..room_graph.len() {
        loop {
            let room1 = room_graph.choose(rng).expect("no rooms in offices");
            let room2 = room_graph.choose(rng).expect("no rooms in offices");
            if let Some(wall) = get_connecting_wall(room1, room2) {
                let door = wall.choose(rng);
                doors.push(door);
                break;
            }
        }
    }

    let mut tiles = HashMap::new();
    for p in rect {
        tiles.insert(p, TileKind::Wall);
    }
    for room in rooms.iter() {
        for pos in *room {
            tiles.insert(pos, TileKind::Floor);
        }
    }
    for door in doors {
        tiles.insert(door, TileKind::Floor);
    }

    LevelDraft {
        start: rooms[0].center(),
        end: rooms.iter().last().unwrap().center(),
        tiles,
        mobs: Default::default(),
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
    let signal_map = signal::generate_signal_map(draft.get_containing_rect(), rng.random());

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
    down_pos: rogue_algebra::Pos,
    up_pos: rogue_algebra::Pos,
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
                Interactable {
                    action: "Go Up".to_string(),
                    description: None,
                    kind: InteractionType::Stairs,
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
                Interactable {
                    action: "Go Down".to_string(),
                    description: None,
                    kind: InteractionType::Stairs,
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

pub(crate) fn draft_level_mapgen_simple(rng: &mut impl Rng) -> LevelDraft {
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
    draft_level_mapgen_rs(
        mapgen_builder,
        &mut rand_8::rngs::StdRng::from_seed(rng.random()),
    )
}

pub(crate) fn draft_level_mapgen_drunk(rng: &mut impl Rng) -> LevelDraft {
    let mut mapgen_builder = mapgen::MapBuilder::new(80, 50);
    mapgen_builder
        .with(mapgen::DrunkardsWalk::open_halls())
        .with(mapgen::AreaStartingPosition::new(
            mapgen::XStart::LEFT,
            mapgen::YStart::CENTER,
        ))
        .with(mapgen::CullUnreachable::new())
        .with(mapgen::DistantExit::new());
    draft_level_mapgen_rs(
        mapgen_builder,
        &mut rand_8::rngs::StdRng::from_seed(rng.random()),
    )
}

pub(crate) fn gen_map(world: Entity, mut commands: Commands, assets: Res<WorldAssets>) {
    let rng = &mut rand::rng();

    let level_1_draft = gen_offices(rng, rogue_algebra::Rect::new(0, 40, 0, 40))
        .with_walls()
        .sprinkle_mobs(rng, 10);
    let player_pos = MapPos(IVec2::from(level_1_draft.start));

    let l2_drafts = [
        draft_level_mapgen_drunk(rng)
            .with_walls()
            .sprinkle_mobs(rng, 10),
        draft_level_mapgen_simple(rng)
            .with_walls()
            .sprinkle_mobs(rng, 10),
        gen_offices(rng, rogue_algebra::Rect::new(0, 40, 0, 40))
            .with_walls()
            .sprinkle_mobs(rng, 10),
    ];

    let mut stair_locs = vec![];
    let mut l1_exits = vec![level_1_draft.end];
    l1_exits.extend(level_1_draft.get_random_floors(2, rng));

    let mut levels = vec![(
        rogue_algebra::Offset::ZERO,
        "Level 1".to_string(),
        level_1_draft,
    )];

    for (i, level) in l2_drafts.into_iter().enumerate() {
        let offset = rogue_algebra::Offset::new(i as i32 * 200, 200);
        stair_locs.push((l1_exits[i], level.start + offset));
        levels.push((offset, format!("Level 2-{i}"), level));
    }

    for (offset, name, level) in levels {
        spawn_level(name, rng, world, &mut commands, &assets, &level, offset);
    }

    for (p1, p2) in stair_locs {
        spawn_stairs(world, &mut commands, &assets, p1, p2);
    }

    let player_sprite = assets.get_ascii_sprite('@', Color::WHITE);
    let player = (
        Player {
            brainrot: 0,
            hunger: 0,
            money: 0,
            rizz: 10,
            strength: 20,
            boredom: 30,
            signal: 5,
            money_gain_timer: 0.0,
            last_gain_amount: 0,
            max_depth: 0,
            abilities: vec![],
            upgrades: vec![],
            pending_upgrades: 1,
            upgrade_options: vec![],
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
