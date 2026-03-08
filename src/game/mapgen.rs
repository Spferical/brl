use crate::game::{
    CookedMeal, Creature, DropsCorpse, ENEMY_FACTION, FRIENDLY_FACTION, Interactable,
    InteractionType, MinSpawnZone, Mob, MobAttrs, MobBundle, PLAYER_FACTION, PLAYER_Z, Player,
    Resist, Summon, TILE_Z,
    assets::WorldAssets,
    camera::CameraFollow,
    lighting::Occluder,
    map::{self, MapPos, Tile},
    signal,
    spawn::{spawn_mob, spawn_stairs},
    upgrades::UPGRADES,
};
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use noisy_bevy::simplex_noise_2d_seeded;
use rand::{
    Rng,
    seq::{IndexedRandom, SliceRandom},
};
use rand_8::SeedableRng;
use rogue_algebra::Pos;

#[derive(PartialEq, Clone, Copy, Debug)]
enum FloorKind {
    Sand,
    Rock,
    Grass,
    Custom(char, Color),
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum TileKind {
    Floor(FloorKind),
    Water,
    Wall,
    WorkoutMachine,
    Reactor,
    MedicalPod,
    Star,
    ArcadeMachine,
    Table,
    Upgrade(Option<&'static str>),
}

impl TileKind {
    fn is_floor(&self) -> bool {
        matches!(self, Self::Floor(..))
    }
}

#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq, Hash, Default)]
pub(crate) enum MobKind {
    #[default]
    GiantFrog,
    SadFrog,
    SmugFrog,
    MadFrog,
    GymBro,
    Influencer,
    Normie,
    FriendlyNormie,
    AmogusCrew,
    AmogusImpostor,
    Capybara,
    KlarnaKop(i32),
    BrainrotEnemy,
    Fortnite(i32),
    Animatronic,
    Streamer,
    Eceleb,
    Fan,
    Stan,
    Whale,
    Drone,
    Zombie,
    Skeleton,
    Spider,
    Enderman,
    ChadGPT,
}

const LVL1_DIST: &[(MobKind, usize)] = &[
    (MobKind::GiantFrog, 1),
    (MobKind::Influencer, 1),
    (MobKind::Normie, 1),
    (MobKind::Capybara, 1),
];
const BACKROOM_DIST: &[(MobKind, usize)] = &[
    (MobKind::SadFrog, 3),
    (MobKind::GymBro, 3),
    (MobKind::Streamer, 3),
    (MobKind::AmogusImpostor, 3),
    (MobKind::Influencer, 1),
    (MobKind::Normie, 1),
    (MobKind::Capybara, 1),
];
const GYM_DIST: &[(MobKind, usize)] = &[
    (MobKind::GymBro, 10),
    (MobKind::Normie, 1),
    (MobKind::Influencer, 2),
    (MobKind::GiantFrog, 1),
];
const FORTNITE_DIST: &[(MobKind, usize)] = &[
    (MobKind::Fortnite(1), 1),
    (MobKind::Fortnite(2), 1),
    (MobKind::Fortnite(3), 1),
];
const AMOGUS_DIST: &[(MobKind, usize)] = &[
    (MobKind::AmogusCrew, 14),
    (MobKind::AmogusImpostor, 2),
    (MobKind::Normie, 1),
];
const MINECRAFT_DIST: &[(MobKind, usize)] = &[
    (MobKind::Zombie, 10),
    (MobKind::Skeleton, 10),
    (MobKind::Spider, 10),
    (MobKind::Enderman, 10),
];
const FREDDY_DIST: &[(MobKind, usize)] = &[(MobKind::Animatronic, 1)];
const CAVES_DIST: &[(MobKind, usize)] = &[
    (MobKind::Zombie, 2),
    (MobKind::Skeleton, 2),
    (MobKind::Spider, 2),
    (MobKind::SmugFrog, 5),
    (MobKind::MadFrog, 5),
    (MobKind::Eceleb, 5),
    (MobKind::Animatronic, 2),
];
const POND_DIST: &[(MobKind, usize)] = &[
    (MobKind::SadFrog, 2),
    (MobKind::MadFrog, 3),
    (MobKind::SmugFrog, 3),
    (MobKind::GiantFrog, 2),
];
const OFFICE_DIST: &[(MobKind, usize)] = &[
    (MobKind::ChadGPT, 1),
    (MobKind::Normie, 1),
    (MobKind::Drone, 1),
];

impl MobKind {
    pub(crate) fn get_cooked_meal(&self) -> (&'static str, CookedMeal) {
        match self {
            MobKind::Animatronic => (
                "Stale Pizza",
                CookedMeal {
                    hunger: 10,
                    hp: 2,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 5,
                },
            ),
            MobKind::Drone => (
                "32GB DDR5 Memory Stick",
                CookedMeal {
                    hunger: 5,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: -5,
                    boredom: 0,
                },
            ),
            MobKind::Zombie => (
                "Rotten Flesh",
                CookedMeal {
                    hunger: 10,
                    hp: 0,
                    strength: -1,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 0,
                },
            ),
            MobKind::Skeleton => (
                "Bonemeal",
                CookedMeal {
                    hunger: 5,
                    hp: -1,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 5,
                },
            ),
            MobKind::Spider => (
                "Cooked Spider",
                CookedMeal {
                    hunger: 10,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: 1,
                    boredom: 0,
                },
            ),
            MobKind::Enderman => (
                "Boiled Ender Pearl",
                CookedMeal {
                    hunger: 10,
                    hp: 0,
                    strength: 0,
                    rizz: 1,
                    brainrot: 0,
                    boredom: 0,
                },
            ),
            MobKind::GymBro | MobKind::ChadGPT => (
                "Beefcake",
                CookedMeal {
                    hunger: 25,
                    hp: 0,
                    strength: 10,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 0,
                },
            ),
            MobKind::GiantFrog | MobKind::SadFrog | MobKind::SmugFrog | MobKind::MadFrog => (
                "Frog Legs",
                CookedMeal {
                    hunger: 30,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 0,
                },
            ),
            MobKind::Influencer => (
                "Cooked Influencer",
                CookedMeal {
                    hunger: 15,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 20,
                },
            ),
            MobKind::Normie | MobKind::FriendlyNormie => (
                "Long Pork",
                CookedMeal {
                    hunger: 40,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 0,
                },
            ),
            MobKind::AmogusCrew | MobKind::AmogusImpostor => (
                "Beefus",
                CookedMeal {
                    hunger: 10,
                    hp: 0,
                    strength: 2,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 0,
                },
            ),
            MobKind::Capybara => (
                "Carne de Chiguiro",
                CookedMeal {
                    hunger: 15,
                    hp: 5,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 30,
                },
            ),
            MobKind::KlarnaKop(_) => (
                "4-Part Interest-Free Burrito",
                CookedMeal {
                    hunger: 20,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 0,
                },
            ),
            MobKind::BrainrotEnemy => (
                "Rotten Brain",
                CookedMeal {
                    hunger: 5,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 50,
                },
            ),
            MobKind::Fortnite(_) => (
                "Chicken Dinner",
                CookedMeal {
                    hunger: 10,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: 1,
                    boredom: 0,
                },
            ),
            MobKind::Streamer | MobKind::Eceleb => (
                "Gamer Juice",
                CookedMeal {
                    hunger: 5,
                    hp: 0,
                    strength: 0,
                    rizz: 10,
                    brainrot: 15,
                    boredom: 0,
                },
            ),
            MobKind::Stan | MobKind::Fan | MobKind::Whale => (
                "Long Pork",
                CookedMeal {
                    hunger: 40,
                    hp: 0,
                    strength: 0,
                    rizz: 0,
                    brainrot: 0,
                    boredom: 0,
                },
            ),
        }
    }

    pub(crate) fn get_bundle(&self, assets: &WorldAssets) -> MobBundle {
        match self {
            MobKind::Animatronic => MobBundle {
                name: Name::new("Animatronic"),
                creature: Creature {
                    hp: 25,
                    max_hp: 25,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: true,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    attrs: MobAttrs {
                        psychic_resist: Resist::Weak,
                        boredom_resist: Resist::Weak,
                        aura_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('A', Color::srgb(0.5, 0.5, 0.5)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.5, 0.5, 0.5)),
                    nutrition: 5,
                    name: "Animatronic".to_string(),
                    kind: *self,
                },
            },
            MobKind::Drone => MobBundle {
                name: Name::new("Drone"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: true,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    attrs: MobAttrs {
                        psychic_resist: Resist::Strong,
                        boredom_resist: Resist::Strong,
                        aura_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('d', Color::srgb(0.5, 0.5, 0.5)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.5, 0.5, 0.5)),
                    nutrition: 5,
                    name: "Drone".to_string(),
                    kind: *self,
                },
            },
            MobKind::ChadGPT => MobBundle {
                name: Name::new("ChadGPT"),
                creature: Creature {
                    hp: 15,
                    max_hp: 15,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 3,
                    keepaway: true,
                    attrs: MobAttrs {
                        summon: Some(Summon {
                            kind: MobKind::Drone,
                            delay: 3,
                        }),
                        physical_resist: Resist::Strong,
                        boredom_resist: Resist::Weak,
                        aura_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('C', Color::srgb(0.769, 0.529, 0.51)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "ChadGPT".to_string(),
                    kind: *self,
                },
            },
            MobKind::Zombie => MobBundle {
                name: Name::new("Zombie"),
                creature: Creature {
                    hp: 5,
                    max_hp: 5,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    attrs: MobAttrs {
                        boredom_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('Z', Color::srgb(0.0, 0.48, 0.07)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.0, 0.48, 0.07)),
                    nutrition: 5,
                    name: "Zombie".to_string(),
                    kind: *self,
                },
            },
            MobKind::Skeleton => MobBundle {
                name: Name::new("Skeleton"),
                creature: Creature {
                    hp: 5,
                    max_hp: 5,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    ranged: true,
                    keepaway: false,
                    attrs: MobAttrs {
                        psychic_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('S', Color::srgb(0.9, 0.9, 0.9)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.9, 0.9, 0.9)),
                    nutrition: 5,
                    name: "Skeleton".to_string(),
                    kind: *self,
                },
            },
            MobKind::Spider => MobBundle {
                name: Name::new("Spider"),
                creature: Creature {
                    hp: 4,
                    max_hp: 4,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 2,
                    keepaway: true,
                    attrs: MobAttrs {
                        psychic_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('S', Color::srgb(0.85, 0.0, 0.57)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "Spider".to_string(),
                    kind: *self,
                },
            },
            MobKind::Enderman => MobBundle {
                name: Name::new("Enderman"),
                creature: Creature {
                    hp: 4,
                    max_hp: 4,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 2,
                    attrs: MobAttrs {
                        sus: true,
                        aura_resist: Resist::Weak,
                        boredom_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('E', Color::srgb(0.38, 0.0, 1.0)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "Enderman".to_string(),
                    kind: *self,
                },
            },
            MobKind::Capybara => MobBundle {
                name: Name::new("Capybara"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    attrs: MobAttrs {
                        psychic_resist: Resist::Weak,
                        boredom_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('c', Color::srgb(0.5, 0.3, 0.3)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 10,
                    name: "Capybara".to_string(),
                    kind: *self,
                },
            },

            MobKind::GiantFrog => MobBundle {
                name: Name::new("Giant Frog"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 5,
                    attrs: MobAttrs {
                        based: true,
                        aura_resist: Resist::Weak,
                        psychic_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('f', Color::srgb(0.2, 0.8, 0.2)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "Giant Frog".to_string(),
                    kind: *self,
                },
            },
            MobKind::SadFrog => MobBundle {
                name: Name::new("Sad Frog"),
                creature: Creature {
                    hp: 8,
                    max_hp: 8,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 10,
                    attrs: MobAttrs {
                        based: true,
                        aura_resist: Resist::Weak,
                        psychic_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('f', Color::srgb(0.3, 0.9, 0.3)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "Sad Frog".to_string(),
                    kind: *self,
                },
            },
            MobKind::SmugFrog => MobBundle {
                name: Name::new("Smug Frog"),
                creature: Creature {
                    hp: 12,
                    max_hp: 12,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 20,
                    attrs: MobAttrs {
                        based: true,
                        aura_resist: Resist::Weak,
                        psychic_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('f', Color::srgb(0.4, 1.0, 0.4)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "Smug Frog".to_string(),
                    kind: *self,
                },
            },
            MobKind::MadFrog => MobBundle {
                name: Name::new("Mad Frog"),
                creature: Creature {
                    hp: 8,
                    max_hp: 8,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 10,
                    ranged: true,
                    keepaway: true,
                    attrs: MobAttrs {
                        based: true,
                        aura_resist: Resist::Weak,
                        psychic_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('f', Color::srgb(0.4, 1.0, 0.4)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "Mad Frog".to_string(),
                    kind: *self,
                },
            },
            MobKind::GymBro => MobBundle {
                name: Name::new("Gym Bro"),
                creature: Creature {
                    hp: 3,
                    max_hp: 3,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    attrs: MobAttrs {
                        physical_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('g', Color::srgb(0.8, 0.3, 0.3)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 15,
                    name: "Gym Bro".to_string(),
                    kind: *self,
                },
            },
            MobKind::Influencer => MobBundle {
                name: Name::new("Influencer"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 5,
                    attrs: MobAttrs {
                        summon: Some(Summon {
                            kind: MobKind::Fan,
                            delay: 4,
                        }),
                        mog_risk: true,
                        aura_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('i', Color::srgb(0.2, 0.5, 0.8)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 2,
                    name: "Influencer".to_string(),
                    kind: *self,
                },
            },
            MobKind::Normie => MobBundle {
                name: Name::new("Normie"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 2,
                    attrs: MobAttrs {
                        basic: true,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('n', Color::srgb(0.5, 0.5, 0.5)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 3,
                    name: "Normie".to_string(),
                    kind: *self,
                },
            },
            MobKind::FriendlyNormie => MobBundle {
                name: Name::new("Normie"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: FRIENDLY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    attrs: MobAttrs {
                        basic: true,
                        friendly: true,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('n', Color::srgb(0.5, 0.5, 1.0)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 3,
                    name: "Normie".to_string(),
                    kind: *self,
                },
            },
            MobKind::AmogusCrew => MobBundle {
                name: Name::new("Amogus"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: FRIENDLY_FACTION, // Crew faction
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    attrs: MobAttrs {
                        sus: true,
                        friendly: true,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('a', Color::srgb(1.0, 0.1, 0.1)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 1,
                    name: "Amogus".to_string(),
                    kind: *self,
                },
            },
            MobKind::AmogusImpostor => MobBundle {
                name: Name::new("Amogus"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 4,
                    attrs: MobAttrs {
                        sus: true,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('a', Color::srgb(1.0, 0.1, 0.1)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 1,
                    name: "Amogus".to_string(),
                    kind: *self,
                },
            },
            MobKind::KlarnaKop(level) => {
                let level = (*level).max(1);
                let hp = 3 + (level - 1) * 5;
                let damage = 1 + (level - 1);
                let name = if level > 1 {
                    format!("Klarna Kop (Lvl {level})")
                } else {
                    "Klarna Kop".to_string()
                };
                MobBundle {
                    name: Name::new(name.clone()),
                    creature: Creature {
                        hp,
                        max_hp: hp,
                        faction: -1,
                        killed_by_player: false,
                        machine: false,
                        friend_of_machines: false,
                    },
                    mob: Mob {
                        melee_damage: damage,
                        target: None,
                        destination: None,
                        ranged: false,
                        keepaway: false,
                        attrs: MobAttrs {
                            aura_resist: Resist::Weak,
                            knows_player_location: true,
                            ..Default::default()
                        },
                    },
                    sprite: assets.get_ascii_sprite('k', Color::srgb(0.2, 0.2, 0.8)),
                    corpse: DropsCorpse {
                        sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                        nutrition: 4,
                        name,
                        kind: *self,
                    },
                }
            }
            MobKind::BrainrotEnemy => MobBundle {
                name: Name::new("????"),
                creature: Creature {
                    hp: 5,
                    max_hp: 5,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 2,
                    attrs: MobAttrs {
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite(' ', Color::NONE),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 1,
                    name: "Brainrot".to_string(),
                    kind: *self,
                },
            },
            MobKind::Fortnite(faction) => MobBundle {
                name: Name::new("Shooty McShootFace"),
                creature: Creature {
                    hp: 5,
                    max_hp: 5,
                    faction: *faction,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 2,
                    ranged: true,
                    keepaway: false,
                    attrs: MobAttrs {
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('@', Color::srgb(0.8, 0.8, 1.0)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 4,
                    name: "Battler".to_string(),
                    kind: *self,
                },
            },
            MobKind::Streamer => MobBundle {
                name: Name::new("Streamer"),
                creature: Creature {
                    hp: 4,
                    max_hp: 4,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 20,
                    keepaway: true,
                    attrs: MobAttrs {
                        based: true,
                        raids_player: true,
                        summon: Some(Summon {
                            kind: MobKind::Stan,
                            delay: 4,
                        }),
                        physical_resist: Resist::Weak,
                        aura_resist: Resist::Strong,
                        psychic_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('S', Color::srgb(0.5, 0.2, 0.8)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "Streamer".to_string(),
                    kind: *self,
                },
            },
            MobKind::Eceleb => MobBundle {
                name: Name::new("E-Celeb"),
                creature: Creature {
                    hp: 8,
                    max_hp: 8,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 30,
                    keepaway: true,
                    attrs: MobAttrs {
                        based: true,
                        raids_player: true,
                        summon: Some(Summon {
                            kind: MobKind::Whale,
                            delay: 4,
                        }),
                        physical_resist: Resist::Weak,
                        aura_resist: Resist::Strong,
                        psychic_resist: Resist::Strong,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('E', Color::srgb(0.5, 0.2, 0.8)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 5,
                    name: "E-Celeb".to_string(),
                    kind: *self,
                },
            },
            MobKind::Fan => MobBundle {
                name: Name::new("Fan"),
                creature: Creature {
                    hp: 2,
                    max_hp: 2,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 1,
                    attrs: MobAttrs {
                        aura_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('f', Color::srgb(0.7, 0.4, 0.9)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 3,
                    name: "Fan".to_string(),
                    kind: *self,
                },
            },
            MobKind::Stan => MobBundle {
                name: Name::new("Stan"),
                creature: Creature {
                    hp: 4,
                    max_hp: 4,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 2,
                    attrs: MobAttrs {
                        aura_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('s', Color::srgb(0.7, 0.4, 0.9)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 3,
                    name: "Stan".to_string(),
                    kind: *self,
                },
            },
            MobKind::Whale => MobBundle {
                name: Name::new("Whale"),
                creature: Creature {
                    hp: 8,
                    max_hp: 8,
                    faction: ENEMY_FACTION,
                    killed_by_player: false,
                    machine: false,
                    friend_of_machines: false,
                },
                mob: Mob {
                    melee_damage: 4,
                    attrs: MobAttrs {
                        aura_resist: Resist::Weak,
                        ..Default::default()
                    },
                    ..default()
                },
                sprite: assets.get_ascii_sprite('w', Color::srgb(0.62, 0.82, 1.0)),
                corpse: DropsCorpse {
                    sprite: assets.get_ascii_sprite('%', Color::srgb(0.8, 0.2, 0.2)),
                    nutrition: 3,
                    name: "Whale".to_string(),
                    kind: *self,
                },
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelTitle {
    Caves,
    Gym,
    Dungeon,
    Entrance,
    Backrooms,
    Island,
    AmogusSpaceship,
    Freddy,
    Minecraft,
    FrogPond,
    Office,
}

impl std::fmt::Display for LevelTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LevelTitle::Caves => "Some Caves",
            LevelTitle::Gym => "Dungeon Fitness",
            LevelTitle::Dungeon => "The Dungeon",
            LevelTitle::Entrance => "Dungeon Entrance",
            LevelTitle::Backrooms => "Tastelessly Carpeted Backrooms",
            LevelTitle::Island => "Mysterious Island",
            LevelTitle::AmogusSpaceship => "Sussy Ship",
            LevelTitle::Freddy => "Friendo's Pizza & Prizes",
            LevelTitle::Minecraft => "A Blocky Wilderness",
            LevelTitle::FrogPond => "A Peaceful Pond",
            LevelTitle::Office => "An Abandoned Office Space ",
        })
    }
}

pub struct LevelDraft {
    title: LevelTitle,
    entrances: Vec<rogue_algebra::Pos>,
    exits: Vec<rogue_algebra::Pos>,
    tiles: HashMap<rogue_algebra::Pos, TileKind>,
    mobs: HashMap<rogue_algebra::Pos, MobKind>,
    destinations: Vec<rogue_algebra::Pos>,
    override_rect: Option<rogue_algebra::Rect>,
}

impl LevelDraft {
    fn with_upgrade(mut self, rng: &mut impl Rng, name: Option<&'static str>) -> Self {
        self.tiles
            .insert(self.get_random_floor(rng), TileKind::Upgrade(name));
        self
    }
    fn get_random_floor(&self, rng: &mut impl Rng) -> Pos {
        let all_floors = self
            .tiles
            .iter()
            .filter(|(_p, t)| t.is_floor())
            .map(|(p, _t)| *p)
            .collect::<Vec<_>>();
        *all_floors.choose(rng).expect("get_random_floor: no floors")
    }
    fn add_random_stairs(&mut self, min_entrances: usize, min_exits: usize, rng: &mut impl Rng) {
        let mut all_floors = self
            .tiles
            .iter()
            .filter(|(_p, t)| t.is_floor())
            .map(|(p, _t)| *p)
            .collect::<HashSet<_>>();
        for e in self.entrances.iter().chain(self.exits.iter()) {
            all_floors.remove(e);
        }
        let all_floors = all_floors.into_iter().collect::<Vec<_>>();
        let needed_entrances = min_entrances.saturating_sub(self.entrances.len());
        let needed_exits = min_exits.saturating_sub(self.entrances.len());
        let new_stairs: Vec<rogue_algebra::Pos> = all_floors
            .choose_multiple(rng, needed_entrances + needed_exits)
            .copied()
            .collect();
        self.entrances
            .extend(new_stairs[0..needed_entrances].iter().cloned());
        self.exits
            .extend(new_stairs[needed_entrances..].iter().cloned());
    }
    fn get_containing_rect(&self) -> rogue_algebra::Rect {
        if let Some(rect) = self.override_rect {
            return rect;
        }
        let min_x = self.tiles.keys().map(|k| k.x).min().expect("Empty level");
        let max_x = self.tiles.keys().map(|k| k.x).max().expect("Empty level");
        let min_y = self.tiles.keys().map(|k| k.y).min().expect("Empty level");
        let max_y = self.tiles.keys().map(|k| k.y).max().expect("Empty level");
        rogue_algebra::Rect::new(min_x, max_x, min_y, max_y)
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

    fn sprinkle_mobs(
        mut self,
        rng: &mut impl Rng,
        dist: &[(MobKind, usize)],
        num_mobs: usize,
    ) -> Self {
        let floors = self
            .tiles
            .iter()
            .filter(|(_, tk)| tk.is_floor())
            .map(|(pos, _)| pos)
            .copied()
            .collect::<Vec<rogue_algebra::Pos>>();
        for pos in floors.choose_multiple(rng, num_mobs) {
            self.mobs
                .insert(*pos, dist.choose_weighted(rng, |m| m.1).unwrap().0);
        }
        self
    }
}

fn draft_level_mapgen_rs(
    mut builder: mapgen::MapBuilder,
    rng: &mut rand_8::rngs::StdRng,
    ty: LevelTitle,
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
                    TileKind::Floor(FloorKind::Rock)
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
            .filter(|p| tiles.get(p).unwrap_or(&TileKind::Wall).is_floor())
    })
    .collect::<HashMap<_, _>>();
    let mut furthest_tile = start_pos;
    for (&pos, &dist) in dijkstra_map.iter() {
        if dist == usize::MAX {
            tiles.insert(pos, TileKind::Wall);
        } else if dist > *dijkstra_map.get(&furthest_tile).unwrap() {
            furthest_tile = pos;
        }
    }
    LevelDraft {
        title: ty,
        entrances: vec![start_pos],
        exits: vec![furthest_tile],
        tiles,
        mobs: HashMap::new(),
        destinations: vec![],
        override_rect: None,
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

fn gen_backrooms(rng: &mut impl Rng, rect: rogue_algebra::Rect) -> LevelDraft {
    let mut draft = gen_entrance(rng, rect);
    draft.title = LevelTitle::Backrooms;
    for t in draft.tiles.values_mut() {
        if t.is_floor() {
            *t = TileKind::Floor(FloorKind::Custom('.', Color::srgb(0.929, 0.749, 0.549)));
        }
    }
    draft
}

fn gen_entrance(rng: &mut impl Rng, rect: rogue_algebra::Rect) -> LevelDraft {
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
            tiles.insert(pos, TileKind::Floor(FloorKind::Rock));
        }
    }
    for door in doors {
        tiles.insert(door, TileKind::Floor(FloorKind::Rock));
    }

    let stairs = rooms
        .choose_multiple(rng, 6)
        .map(|room| room.center())
        .collect::<Vec<_>>();

    LevelDraft {
        title: LevelTitle::Entrance,
        entrances: stairs[0..3].to_vec(),
        exits: stairs[3..].to_vec(),
        tiles,
        mobs: Default::default(),
        destinations: vec![],
        override_rect: None,
    }
}

fn gen_office(rng: &mut impl Rng) -> LevelDraft {
    let rect = rogue_algebra::Rect::new(0, 40, 0, 40);
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
            tiles.insert(pos, TileKind::Floor(FloorKind::Custom('.', Color::WHITE)));
        }
    }
    for door in doors {
        tiles.insert(door, TileKind::Floor(FloorKind::Rock));
    }

    let stairs = rooms
        .choose_multiple(rng, 6)
        .map(|room| room.center())
        .collect::<Vec<_>>();

    LevelDraft {
        title: LevelTitle::Office,
        entrances: stairs[0..3].to_vec(),
        exits: stairs[3..].to_vec(),
        tiles,
        mobs: Default::default(),
        destinations: vec![],
        override_rect: None,
    }
}

fn gen_dungeon_fitness(rng: &mut impl Rng) -> LevelDraft {
    let rect = rogue_algebra::Rect::new(0, 80, 0, 25);
    let max_width = 20;
    let min_width = max_width / 2 - 1;
    let max_height = 10;
    let min_height = max_height / 2 - 1;
    let bsp_opts = CarveRoomOpts {
        max_width,
        max_height,
        min_width,
        min_height,
    };
    let tree = gen_bsp_tree(rect, bsp_opts.into(), rng);
    let room_graph = tree.into_room_graph();
    let mut rooms = room_graph.iter().collect::<Vec<rogue_algebra::Rect>>();
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
            let room1 = room_graph.choose(rng).expect("no rooms in df");
            let room2 = room_graph.choose(rng).expect("no rooms in df");
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
            tiles.insert(pos, TileKind::Floor(FloorKind::Rock));
        }
    }
    for door in doors {
        tiles.insert(door, TileKind::Floor(FloorKind::Rock));
    }

    rooms.shuffle(rng);

    let stairs = rooms[0..6]
        .iter()
        .map(|room| room.center())
        .collect::<Vec<_>>();
    for room in rooms[6..].iter() {
        let room_tiles = room.into_iter().collect::<Vec<Pos>>();
        let num_workout_machines = room_tiles.len() / 8;
        for pos in room_tiles[0..num_workout_machines].iter() {
            tiles.insert(*pos, TileKind::WorkoutMachine);
        }
    }

    LevelDraft {
        title: LevelTitle::Gym,
        entrances: stairs[0..3].to_vec(),
        exits: stairs[3..].to_vec(),
        tiles,
        mobs: HashMap::new(),
        destinations: vec![],
        override_rect: None,
    }
}

fn create_prefab_room(
    tiles: &mut HashMap<Pos, TileKind>,
    start_pos: Pos,
    prefab: &str,
) -> rogue_algebra::Rect {
    let mut max_x = 0;
    let mut max_y = 0;
    for (y, line) in prefab.lines().rev().enumerate() {
        for (x, c) in line.chars().enumerate() {
            let pos = start_pos + rogue_algebra::Offset::new(x as i32, y as i32);
            let tk = match c {
                '#' => TileKind::Wall,
                '.' => TileKind::Floor(FloorKind::Rock),
                '$' => TileKind::ArcadeMachine,
                '&' => TileKind::WorkoutMachine,
                '*' => TileKind::Reactor,
                '+' => TileKind::MedicalPod,
                'T' => TileKind::Table,
                ' ' => TileKind::Wall,
                _ => {
                    warn!("create_prefab_room: unexpected '{c}'");
                    TileKind::Wall
                }
            };
            tiles.insert(pos, tk);
            max_x = max_x.max(x as i32);
            max_y = max_y.max(y as i32);
        }
    }
    rogue_algebra::Rect::new(
        start_pos.x,
        start_pos.x + max_x,
        start_pos.y,
        start_pos.y + max_y,
    )
}

fn gen_amogus_spaceship(rng: &mut impl Rng) -> LevelDraft {
    let mut tiles = HashMap::new();
    let ship_bounds = rogue_algebra::Rect::new(0, 95, 0, 48);
    let world_bounds = ship_bounds.expand(20);
    for p in world_bounds {
        if ship_bounds.contains(p) {
            tiles.insert(p, TileKind::Wall);
        } else if rng.random_bool(0.01) {
            tiles.insert(p, TileKind::Star);
        }
    }

    let cafeteria_prefab = "
#################
#...............#
#...#.......#...#
#...............#
#.......#.......#
#...............#
#...#.......#...#
#...............#
#################";
    let storage_prefab = "
#################
#...............#
#...#########...#
#...#########...#
#...#########...#
#...............#
#################";
    let reactor_prefab = "
###########
#.........#
#...***...#
#...***...#
#.........#
###########";
    let engine_prefab = "
#############
#...........#
#.....&.....#
#...........#
#############";
    let medbay_prefab = "
###########
#.........#
#..+...+..#
#.........#
###########";
    let small_room_prefab = "
###########
#.........#
#.........#
#.........#
###########";
    let electrical_prefab = "
###########
#.........#
#..#####..#
#..&...&..#
#.........#
###########";
    let admin_navigation_prefab = "
###########
#.........#
#....&....#
#.........#
###########";

    // Place larger rooms tightly
    let reactor = create_prefab_room(&mut tiles, Pos::new(2, 20), reactor_prefab);
    let upper_engine = create_prefab_room(&mut tiles, Pos::new(15, 5), engine_prefab);
    let lower_engine = create_prefab_room(&mut tiles, Pos::new(15, 35), engine_prefab);
    let security = create_prefab_room(&mut tiles, Pos::new(17, 20), small_room_prefab);
    let medbay = create_prefab_room(&mut tiles, Pos::new(32, 5), medbay_prefab);
    let electrical = create_prefab_room(&mut tiles, Pos::new(32, 35), electrical_prefab);
    let cafeteria = create_prefab_room(&mut tiles, Pos::new(48, 2), cafeteria_prefab);
    let storage = create_prefab_room(&mut tiles, Pos::new(48, 35), storage_prefab);
    let admin = create_prefab_room(&mut tiles, Pos::new(68, 18), admin_navigation_prefab);
    let weapons = create_prefab_room(&mut tiles, Pos::new(72, 5), small_room_prefab);
    let shields = create_prefab_room(&mut tiles, Pos::new(72, 35), small_room_prefab);
    let navigation = create_prefab_room(&mut tiles, Pos::new(82, 20), admin_navigation_prefab);

    // Connect rooms with 1-tile wide corridors
    let mut corridors = vec![];
    corridors.push((reactor.center(), upper_engine.center()));
    corridors.push((reactor.center(), lower_engine.center()));
    corridors.push((upper_engine.center(), security.center()));
    corridors.push((lower_engine.center(), security.center()));
    corridors.push((security.center(), medbay.center()));
    corridors.push((security.center(), electrical.center()));
    corridors.push((medbay.center(), cafeteria.center()));
    corridors.push((electrical.center(), storage.center()));
    corridors.push((cafeteria.center(), weapons.center()));
    corridors.push((weapons.center(), navigation.center()));
    corridors.push((navigation.center(), shields.center()));
    corridors.push((shields.center(), storage.center()));
    corridors.push((cafeteria.center(), admin.center()));
    corridors.push((admin.center(), storage.center()));
    corridors.push((cafeteria.center(), storage.center()));

    for (p1, p2) in corridors {
        let mut curr = p1;
        while curr != p2 {
            tiles.insert(curr, TileKind::Floor(FloorKind::Rock));
            if curr.x != p2.x {
                curr.x += (p2.x - curr.x).signum();
            } else if curr.y != p2.y {
                curr.y += (p2.y - curr.y).signum();
            }
        }
        tiles.insert(p2, TileKind::Floor(FloorKind::Rock));
    }

    LevelDraft {
        title: LevelTitle::AmogusSpaceship,
        entrances: vec![cafeteria.center()],
        exits: vec![navigation.center()],
        tiles,
        mobs: HashMap::new(),
        destinations: vec![
            reactor.center(),
            upper_engine.center(),
            lower_engine.center(),
            security.center(),
            medbay.center(),
            electrical.center(),
            cafeteria.center(),
            storage.center(),
            admin.center(),
            weapons.center(),
            shields.center(),
            navigation.center(),
        ],
        override_rect: Some(ship_bounds),
    }
}

fn gen_island(rng: &mut impl Rng) -> LevelDraft {
    let ocean_rect = rogue_algebra::Rect::new(-20, 60, -20, 60);

    let mut mapgen_builder = mapgen::MapBuilder::new(40, 40);
    mapgen_builder
        .with(mapgen::NoiseGenerator::uniform())
        .with(mapgen::CellularAutomata::new())
        .with(mapgen::AreaStartingPosition::new(
            mapgen::XStart::CENTER,
            mapgen::YStart::CENTER,
        ))
        .with(mapgen::CullUnreachable::new());
    let mut draft = draft_level_mapgen_rs(
        mapgen_builder,
        &mut rand_8::rngs::StdRng::from_seed(rng.random()),
        LevelTitle::Island,
    );

    let mut new_tiles = HashMap::<Pos, TileKind>::new();
    for p in ocean_rect {
        new_tiles.insert(
            p,
            match draft.tiles.get(&p) {
                Some(TileKind::Floor(..)) => TileKind::Floor(FloorKind::Sand),
                _ => TileKind::Water,
            },
        );
    }
    draft.tiles = new_tiles;
    draft.override_rect = None;
    draft
}

fn gen_freddy(_rng: &mut impl Rng) -> LevelDraft {
    let mut tiles = HashMap::new();

    let prefab = "
           #################
           #...............#
           #..........#....#
           #..........#....#######
           #####..#####..........#
           #..........#....#.....#
############..........###.####.###
#...........................#..#
#....#......................#..#
#....#..T..T..T.............#..#
#....#..T..T..T.............#..#
#....#..T..T..T................#
#....#......................#..#
#....#..T..T..T.............#..#
#....#..T..T..T.............#..#####
######..T..T..T.............#......#
#....#......................#..#...#
#....#......................#..#...#
#................$.$.$.$.$..#..#####
#....#......................#......#
#....#...........$.$.$.$.$..#..#...#
#....#......................#..#...#
#....######.###########.#####.######
#.........#.###########.#..........#
######....#.###########.#..........#
     #....#.###########.#..........#
     #......###########.#..........#
     #....#.###########............#
     #....#.###########.############
     #....#.#.........#.#
     ######.............#
          #.............#
          #.#.........#.#
          #.#.........#.#
          ###############";

    let _room = create_prefab_room(&mut tiles, Pos::new(0, 0), prefab);
    for t in tiles.values_mut() {
        if let TileKind::Floor(..) = t {
            *t = TileKind::Floor(FloorKind::Custom('.', Color::srgb(0.0, 0.0, 1.0)));
        }
    }

    LevelDraft {
        title: LevelTitle::Freddy,
        entrances: vec![],
        exits: vec![],
        tiles,
        mobs: HashMap::new(),
        destinations: vec![],
        override_rect: None,
    }
}

fn gen_minecraft(rng: &mut impl Rng) -> LevelDraft {
    let rect = rogue_algebra::Rect::new(0, 80, 0, 50);
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
    let mut draft = draft_level_mapgen_rs(
        mapgen_builder,
        &mut rand_8::rngs::StdRng::from_seed(rng.random()),
        LevelTitle::Minecraft,
    );

    // second pass, generate mountain/grass/sand/water based on CA
    let seed = rng.random();
    for pos in rect {
        let frequency = 1.0f32 / 20.0f32;
        let n = simplex_noise_2d_seeded(
            Vec2::new(pos.x as f32 * frequency, pos.y as f32 * frequency),
            seed,
        );
        if n < -0.8 {
            draft.tiles.insert(pos, TileKind::Water);
        } else if n < -0.5 {
            draft.tiles.insert(pos, TileKind::Floor(FloorKind::Sand));
        } else if n < 0.0 {
            draft.tiles.insert(pos, TileKind::Floor(FloorKind::Grass));
        }
    }

    // bfs from entrance and cull any unreachable floors
    let accessible_floors = rogue_algebra::path::bfs(&draft.entrances, 99, |p| {
        rogue_algebra::CARDINALS
            .into_iter()
            .map(move |o| p + o)
            .filter(|p| draft.tiles.get(p).map(|t| t.is_floor()).unwrap_or(false))
    })
    .collect::<HashSet<_>>();
    draft
        .tiles
        .retain(|k, v| !v.is_floor() || accessible_floors.contains(k));
    draft
}

fn gen_frog_pond(rng: &mut impl Rng) -> LevelDraft {
    let rect = rogue_algebra::Rect::new(0, 80, 0, 50);
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
    let mut draft = draft_level_mapgen_rs(
        mapgen_builder,
        &mut rand_8::rngs::StdRng::from_seed(rng.random()),
        LevelTitle::FrogPond,
    );

    // second pass, generate mountain/grass/sand/water based on CA
    let seed = rng.random();
    for pos in rect {
        let is_wall = draft.tiles.get(&pos).map(|t| !t.is_floor()).unwrap_or(true);
        let frequency = 1.0f32 / 30.0f32;
        let n = simplex_noise_2d_seeded(
            Vec2::new(pos.x as f32 * frequency, pos.y as f32 * frequency),
            seed,
        );
        if is_wall {
            if n < -0.0 {
                draft.tiles.insert(pos, TileKind::Water);
            } else {
                draft.tiles.insert(pos, TileKind::Floor(FloorKind::Rock));
            }
        }
    }

    // bfs from entrance and cull any unreachable floors
    let accessible_floors = rogue_algebra::path::bfs(&draft.entrances, 99, |p| {
        rogue_algebra::CARDINALS
            .into_iter()
            .map(move |o| p + o)
            .filter(|p| draft.tiles.get(p).map(|t| t.is_floor()).unwrap_or(false))
    })
    .collect::<HashSet<_>>();
    draft
        .tiles
        .retain(|k, v| !v.is_floor() || accessible_floors.contains(k));
    draft
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
    let (strength, frequency, edge_attenuation) = match draft.title {
        LevelTitle::Island => (1.0, 0.1, false),
        LevelTitle::AmogusSpaceship => (0.7, 0.1, true),
        _ => (1.0, 0.1, true),
    };
    let signal_map = signal::generate_signal_map(
        draft.get_containing_rect() + offset,
        rng.random(),
        strength,
        frequency,
        edge_attenuation,
    );

    let level_entity_cmds = commands.spawn((
        Name::new(name),
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        InheritedVisibility::VISIBLE,
        signal_map,
    ));
    let level_entity = level_entity_cmds.id();
    commands.entity(world).add_child(level_entity);

    // Spawn floor background
    let level_rect = draft.get_containing_rect() + offset;
    let width = level_rect.width() as f32 * map::TILE_WIDTH;
    let height = level_rect.height() as f32 * map::TILE_HEIGHT;
    let center_x = (level_rect.x1 + level_rect.x2) as f32 * map::TILE_WIDTH / 2.0;
    let center_y = (level_rect.y1 + level_rect.y2) as f32 * map::TILE_HEIGHT / 2.0;

    commands.entity(level_entity).with_children(|parent| {
        parent.spawn((
            Sprite {
                image: assets.get_solid_mask(),
                color: Color::srgb(0.1, 0.1, 0.1),
                custom_size: Some(Vec2::new(width + 2.0, height + 2.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(center_x, center_y, TILE_Z - 0.1)),
        ));
    });

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
            TileKind::Floor(fk) => {
                let r = rng.random::<f32>();
                let r2 = rng.random::<f32>();
                let (color, ch) = match fk {
                    FloorKind::Custom(ch, color) => (color, ch),
                    FloorKind::Rock => (
                        if r2 < 0.1 {
                            Color::srgb(0.7, 0.7, 0.7)
                        } else {
                            Color::srgb(0.4, 0.4, 0.4)
                        },
                        if r <= 0.1 {
                            '.'
                        } else if r <= 0.2 {
                            ','
                        } else {
                            ' '
                        },
                    ),
                    FloorKind::Grass => {
                        tile.insert(map::Grass);
                        (
                            if r2 < 0.1 {
                                Color::srgb(0.255, 0.576, 0.431)
                            } else {
                                Color::srgb(0.2, 0.8, 0.2)
                            },
                            if r <= 0.2 {
                                '.'
                            } else if r <= 0.6 {
                                '\''
                            } else {
                                '"'
                            },
                        )
                    }
                    FloorKind::Sand => (
                        if r2 < 0.1 {
                            Color::srgb(1.0, 0.941, 0.894)
                        } else {
                            Color::srgb(0.929, 0.749, 0.549)
                        },
                        if r <= 0.6 { '.' } else { ',' },
                    ),
                };
                tile.insert(assets.get_ascii_sprite(ch, color));
            }
            TileKind::Wall => {
                let sprite = assets.get_ascii_sprite('#', Color::srgb(0.6, 0.6, 0.6));
                tile.insert((sprite, map::BlocksMovement, Occluder));
            }
            TileKind::ArcadeMachine => {
                let sprite = assets.get_ascii_sprite('$', Color::srgb(0.5, 0.5, 0.2));
                tile.insert((
                    sprite,
                    Name::new("Arcade Machine".to_string()),
                    Interactable {
                        action: "Use".to_string(),
                        description: None,
                        kind: InteractionType::Arcade,
                        require_on_top: false,
                    },
                ));
            }
            TileKind::WorkoutMachine => {
                let sprite = assets.get_ascii_sprite('&', Color::srgb(0.2, 0.2, 0.8));
                tile.insert((
                    sprite,
                    Name::new("Workout Machine".to_string()),
                    Interactable {
                        action: "Use".to_string(),
                        description: None,
                        kind: InteractionType::Workout,
                        require_on_top: false,
                    },
                ));
            }
            TileKind::Water => {
                let r = rng.random::<f32>();
                let color = if r < 0.05 {
                    Color::srgb(0.4, 0.6, 1.0)
                } else if r < 0.1 {
                    Color::srgb(0.4, 0.90, 1.0)
                } else {
                    Color::srgb(0.4, 0.4, 1.0)
                };
                let sprite = assets.get_ascii_sprite('~', color);
                tile.insert((Name::new("Water"), sprite, map::BlocksMovement));
            }
            TileKind::Table => {
                let color = Color::srgb(0.6, 0.6, 0.2);
                let sprite = assets.get_ascii_sprite('T', color);
                tile.insert((Name::new("Table"), sprite, map::BlocksMovement));
            }
            TileKind::Reactor => {
                let sprite = assets.get_ascii_sprite('*', Color::srgb(0.0, 1.0, 0.0));
                tile.insert((
                    sprite,
                    Name::new("Reactor".to_string()),
                    Interactable {
                        action: "Expose to Radiation".to_string(),
                        description: Some("Increases brainrot, decreases strength".to_string()),
                        kind: InteractionType::Irradiate,
                        require_on_top: false,
                    },
                    Occluder,
                    map::BlocksMovement,
                ));
            }
            TileKind::MedicalPod => {
                let sprite = assets.get_ascii_sprite('+', Color::srgb(0.0, 0.8, 0.8));
                tile.insert((
                    sprite,
                    Name::new("Medical Pod".to_string()),
                    Interactable {
                        action: "Heal".to_string(),
                        description: Some("Heals 5 HP".to_string()),
                        kind: InteractionType::MedicalPod,
                        require_on_top: false,
                    },
                    map::BlocksMovement,
                ));
            }
            TileKind::Star => {
                let color = Color::srgb(0.8, 0.8, 0.8);
                let sprite = assets.get_ascii_sprite('.', color);
                tile.insert((Name::new("Star"), sprite));
            }
            TileKind::Upgrade(name) => {
                let color = Color::srgb(0.2, 0.2, 1.0);
                let sprite = assets.get_ascii_sprite('*', color);
                tile.insert((
                    Name::new("Upgrade"),
                    sprite,
                    Interactable {
                        action: "Upgrade".to_string(),
                        description: None,
                        kind: InteractionType::Upgrade(
                            name.map(|name| UPGRADES.iter().position(|x| x.name == name).unwrap()),
                        ),
                        require_on_top: false,
                    },
                ));
            }
        }

        tiles.push(tile.id());
    }
    commands.entity(level_entity).add_children(&tiles);

    for (&pos, &mob_kind) in draft.mobs.iter() {
        let pos = pos + offset;
        spawn_mob(
            commands,
            level_entity,
            MapPos(IVec2::from(pos)),
            mob_kind,
            assets,
        );
    }

    if matches!(draft.title, LevelTitle::Island) {
        commands.entity(level_entity).with_children(|parent| {
            parent.spawn(MinSpawnZone {
                rect: draft.get_containing_rect() + offset,
                min_units: 30,
                distribution: FORTNITE_DIST,
            });
        });
    }
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
        LevelTitle::Dungeon,
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
        LevelTitle::Caves,
    )
}

#[derive(Clone, Debug)]
pub(crate) struct LevelInfo {
    #[allow(unused)]
    pub name: String,
    pub ty: LevelTitle,
    #[allow(unused)]
    pub depth: usize,
    pub rect: rogue_algebra::Rect,
    pub destinations: Vec<rogue_algebra::Pos>,
}

#[derive(Resource, Default, Debug)]
pub(crate) struct MapInfo {
    pub levels: Vec<LevelInfo>,
}

impl MapInfo {
    pub fn get_level(&self, pos: MapPos) -> Option<&LevelInfo> {
        self.levels
            .iter()
            .find(|&level| level.rect.contains(Pos::from(pos.0)))
    }
}

pub(crate) fn gen_map(
    world: Entity,
    commands: &mut Commands,
    assets: Res<WorldAssets>,
    map_info: &mut MapInfo,
) {
    let rng = &mut rand::rng();
    map_info.levels.clear();

    // Generate drafts for each level.
    let level_1_draft = gen_entrance(rng, rogue_algebra::Rect::new(0, 40, 0, 40))
        .with_walls()
        .sprinkle_mobs(rng, LVL1_DIST, 10);
    let player_pos = MapPos(IVec2::from(level_1_draft.entrances[0]));
    let mut level_drafts_per_depth = vec![
        vec![level_1_draft],
        vec![
            gen_backrooms(rng, rogue_algebra::Rect::new(0, 40, 0, 40))
                .with_walls()
                .sprinkle_mobs(rng, BACKROOM_DIST, 20),
            gen_dungeon_fitness(rng)
                .with_walls()
                .sprinkle_mobs(rng, GYM_DIST, 20),
        ],
        vec![
            gen_island(rng)
                .with_walls()
                .sprinkle_mobs(rng, FORTNITE_DIST, 30)
                .with_upgrade(rng, Some("Gun")),
            gen_freddy(rng)
                .with_walls()
                .sprinkle_mobs(rng, FREDDY_DIST, 4)
                .with_upgrade(rng, Some("Animatronic Bear Mask")),
        ],
        vec![
            gen_amogus_spaceship(rng)
                .with_walls()
                .sprinkle_mobs(rng, AMOGUS_DIST, 20)
                .with_upgrade(rng, None),
            gen_minecraft(rng)
                .with_walls()
                .sprinkle_mobs(rng, MINECRAFT_DIST, 25)
                .with_upgrade(rng, None),
        ],
        vec![
            draft_level_mapgen_drunk(rng)
                .with_walls()
                .sprinkle_mobs(rng, CAVES_DIST, 30)
                .with_upgrade(rng, None),
            draft_level_mapgen_simple(rng)
                .with_walls()
                .sprinkle_mobs(rng, CAVES_DIST, 30)
                .with_upgrade(rng, None),
        ],
        vec![
            gen_frog_pond(rng)
                .with_walls()
                .sprinkle_mobs(rng, POND_DIST, 40)
                .with_upgrade(rng, None),
            gen_office(rng)
                .with_walls()
                .sprinkle_mobs(rng, OFFICE_DIST, 20)
                .with_upgrade(rng, None),
        ],
    ];

    let mut stair_locs = vec![];
    // Make sure each level has enough stair locations.
    for depth in 0..level_drafts_per_depth.len() {
        let num_higher_levels = if depth > 0 {
            level_drafts_per_depth[depth - 1].len()
        } else {
            0
        };
        let num_lower_levels = level_drafts_per_depth
            .get(depth + 1)
            .map(|drafts| drafts.len())
            .unwrap_or(0);
        for level in &mut level_drafts_per_depth[depth] {
            level.add_random_stairs(num_higher_levels, num_lower_levels, rng);
        }
    }
    // Figure out locations of up/down stair pairs.
    for depth in 0..level_drafts_per_depth.len() - 1 {
        for (i, level) in level_drafts_per_depth[depth].iter().enumerate() {
            let upper_offset = rogue_algebra::Offset::new(i as i32 * 200, depth as i32 * 200);
            for (j, deeper_level) in level_drafts_per_depth[depth + 1].iter().enumerate() {
                let lower_offset =
                    rogue_algebra::Offset::new(j as i32 * 200, (depth + 1) as i32 * 200);
                stair_locs.push((
                    level.exits[j] + upper_offset,
                    deeper_level.entrances[i] + lower_offset,
                ));
            }
        }
    }
    // Calculate offsets and entity names for levels. Also, update MapInfo metadata.
    let mut levels = vec![];
    for (depth, level_drafts) in level_drafts_per_depth.into_iter().enumerate() {
        for (i, level) in level_drafts.into_iter().enumerate() {
            // note: we measure depth reached by y value for progression
            let offset = rogue_algebra::Offset::new(i as i32 * 200, depth as i32 * 200);
            let name = format!("Level {depth}-{i}");
            map_info.levels.push(LevelInfo {
                ty: level.title,
                name: name.clone(),
                depth,
                rect: level.get_containing_rect() + offset,
                destinations: level.destinations.iter().map(|&p| p + offset).collect(),
            });
            levels.push((offset, name, level));
        }
    }

    // Spawn everything.
    for (offset, name, level) in levels {
        spawn_level(name, rng, world, commands, &assets, &level, offset);
    }
    for (p1, p2) in stair_locs {
        spawn_stairs(world, commands, &assets, p1, p2);
    }

    // Spawn the player.
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
            ability_cooldowns: HashMap::default(),
            upgrades: vec![],
            pending_upgrades: 1,
            upgrade_options: vec![],
            subscriptions: vec![],
            food_cooldowns: HashMap::default(),
            is_raided: false,
            high_metabolism: false,
        },
        Creature {
            hp: 10,
            max_hp: 10,
            faction: PLAYER_FACTION,
            killed_by_player: false,
            friend_of_machines: false,
            machine: false,
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
