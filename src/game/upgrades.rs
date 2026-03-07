use std::fmt::Write;
use std::sync::LazyLock;

use bevy::prelude::*;
use rand::seq::IndexedRandom;

use crate::game::{Ability, Creature, Player, Subscription};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Attr {
    MaxHp,
    Brainrot,
    Money,
    Rizz,
    Strength,
    Boredom,
}

impl std::fmt::Display for Attr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Attr::MaxHp => "Max HP",
            Attr::Brainrot => "Brainrot",
            Attr::Money => "Money",
            Attr::Rizz => "Rizz",
            Attr::Strength => "Strength",
            Attr::Boredom => "Boredom",
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Effect {
    AttrChange(Attr, i32),
    GainAbility(Ability),
    FriendOfMachines,
    Subscription(Subscription),
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effect::AttrChange(attr, amt) => write!(f, "{amt:+} {attr}"),
            Effect::GainAbility(ability) => write!(f, "Learn {ability}: {}", ability.describe()),
            Effect::FriendOfMachines => write!(f, "Become friendly to machines"),
            Effect::Subscription(sub) => match sub {
                Subscription::DungeonDashPlatinum => write!(
                    f,
                    "DungeonDash Platinum Subscription: -75% food cost, 5 turn delivery, ${}/100 turns",
                    sub.cost()
                ),
                Subscription::UndergroundTVPro => write!(
                    f,
                    "UndergroundTV Pro Subscription: 3x viewer growth, ${}/100 turns",
                    sub.cost()
                ),
                Subscription::FiveGLTE => {
                    write!(
                        f,
                        "5G LTE Subscription: Guaranteed signal, ${}/100 turns",
                        sub.cost()
                    )
                }
                Subscription::DungeonFitness => {
                    write!(
                        f,
                        "Dungeon Fitness Membership: Passively gain Strength (up to a point), ${}/100 turns",
                        sub.cost()
                    )
                }
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Upgrade {
    pub name: &'static str,
    pub effects: Vec<Effect>,
    pub frequency: f64,
}
impl Upgrade {
    pub fn describe(&self) -> String {
        let mut desc = String::new();
        for effect in &self.effects {
            writeln!(desc, "{effect}").unwrap();
        }
        desc
    }
}

#[derive(Message)]
pub(crate) struct UpgradeMessage {
    pub(crate) upgrade: usize,
}

pub(crate) fn handle_upgrades(
    player: Single<(&mut Player, &mut Creature)>,
    mut msg_upgrade: MessageReader<UpgradeMessage>,
) {
    let (mut player, mut player_creature) = player.into_inner();
    let rng = &mut rand::rng();
    let upgrades: &[Upgrade] = &UPGRADES;

    for UpgradeMessage { upgrade } in msg_upgrade.read() {
        player.pending_upgrades -= 1;
        player.upgrade_options.clear();
        player.upgrades.push(*upgrade);
        let upgrade = &upgrades[*upgrade];
        for effect in &upgrade.effects {
            match effect {
                Effect::AttrChange(attr, amt) => match attr {
                    Attr::MaxHp => {
                        player_creature.max_hp += amt;
                        player_creature.hp += amt;
                    }
                    Attr::Brainrot => player.brainrot += amt,
                    Attr::Money => player.money += amt,
                    Attr::Rizz => player.rizz += amt,
                    Attr::Strength => player.strength += amt,
                    Attr::Boredom => player.boredom += amt,
                },
                Effect::GainAbility(ability) => player.abilities.push(*ability),
                Effect::Subscription(sub) => player.subscriptions.push(*sub),
                Effect::FriendOfMachines => player_creature.friend_of_machines = true,
            }
        }
    }

    if player.upgrade_options.len() < 3 && player.pending_upgrades > 0 {
        let num_needed = 3 - player.upgrade_options.len();
        let valid_options = (0..upgrades.len())
            .filter(|i| !player.upgrades.contains(i))
            .collect::<Vec<_>>();
        if let Ok(choices) =
            valid_options.choose_multiple_weighted(rng, num_needed, |u| UPGRADES[*u].frequency)
        {
            player.upgrade_options.extend(choices);
        }
    }
}

pub static UPGRADES: LazyLock<Vec<Upgrade>> = LazyLock::new(|| {
    vec![
        Upgrade {
            name: "Cardio",
            effects: vec![Effect::AttrChange(Attr::MaxHp, 10)],
            frequency: 5.0,
        },
        Upgrade {
            name: "Trust Fund",
            effects: vec![Effect::AttrChange(Attr::Money, 100)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Group Chat",
            effects: vec![Effect::AttrChange(Attr::Boredom, -25)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Organic",
            effects: vec![Effect::AttrChange(Attr::MaxHp, 10)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Protein Goblin",
            effects: vec![Effect::AttrChange(Attr::Strength, 5)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Grip Strengthener",
            effects: vec![Effect::AttrChange(Attr::Strength, 5)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Mewing",
            effects: vec![Effect::AttrChange(Attr::Rizz, 20)],
            frequency: 5.0,
        },
        Upgrade {
            name: "Sprint",
            effects: vec![Effect::GainAbility(Ability::Sprint)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Shoulder Check",
            effects: vec![Effect::GainAbility(Ability::ShoulderCheck)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Mog",
            effects: vec![Effect::GainAbility(Ability::Mog)],
            frequency: 5.0,
        },
        Upgrade {
            name: "Cook",
            effects: vec![Effect::GainAbility(Ability::Cook)],
            frequency: 5.0,
        },
        Upgrade {
            name: "Memelord",
            effects: vec![Effect::AttrChange(Attr::Brainrot, 50)],
            frequency: 1.0,
        },
        Upgrade {
            name: "DungeonDash Platinum",
            effects: vec![Effect::Subscription(Subscription::DungeonDashPlatinum)],
            frequency: 1.0,
        },
        Upgrade {
            name: "UndergroundTV Pro",
            effects: vec![Effect::Subscription(Subscription::UndergroundTVPro)],
            frequency: 1.0,
        },
        Upgrade {
            name: "5G LTE",
            effects: vec![Effect::Subscription(Subscription::FiveGLTE)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Dungeon Fitness Membership",
            effects: vec![Effect::Subscription(Subscription::DungeonFitness)],
            frequency: 1.0,
        },
        Upgrade {
            name: "Library Card",
            effects: vec![Effect::GainAbility(Ability::ReadBook)],
            frequency: 5.0,
        },
        Upgrade {
            name: "Yap",
            effects: vec![Effect::GainAbility(Ability::Yap)],
            frequency: 5.0,
        },
        Upgrade {
            name: "Animatronic Bear Mask",
            effects: vec![
                Effect::AttrChange(Attr::Rizz, -10),
                Effect::FriendOfMachines,
            ],
            frequency: 0.0,
        },
    ]
});
