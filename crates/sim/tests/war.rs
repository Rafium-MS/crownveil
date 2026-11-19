use crownveil_sim::{Command, World};

fn world() -> World {
    World::demo(7)
}

fn first_province_of(world: &World, faction: usize) -> usize {
    world
        .provinces
        .iter()
        .find(|province| province.owner == faction)
        .map(|province| province.id)
        .unwrap()
}

#[test]
fn declare_war_sets_relation_and_state() {
    let mut world = world();
    world.queue_command(Command::DeclareWar {
        attacker: 0,
        defender: 1,
    });
    world.tick_month();

    assert!(world.diplomacy.is_at_war(0, 1));
    assert_eq!(world.diplomacy.relation(0, 1), -100);
}

#[test]
fn cannot_declare_war_on_self_or_twice() {
    let mut world = world();
    assert!(!world.diplomacy.declare_war(0, 0, 1, 1));
    assert!(world.diplomacy.declare_war(0, 1, 1, 1));
    assert!(!world.diplomacy.declare_war(0, 1, 1, 1));
}

#[test]
fn peace_ends_war_and_starts_truce() {
    let mut world = world();
    world.queue_command(Command::DeclareWar {
        attacker: 0,
        defender: 1,
    });
    world.tick_month();

    world.queue_command(Command::MakePeace {
        faction: 0,
        other: 1,
    });
    world.tick_month();

    assert!(!world.diplomacy.is_at_war(0, 1));
    assert!(world.diplomacy.truce_months(0, 1) > 0);

    let (year, month) = (world.year, world.month);
    assert!(!world.diplomacy.declare_war(0, 1, year, month));
}

#[test]
fn truce_counts_down() {
    let mut world = world();
    world.diplomacy.declare_war(0, 1, 1, 1);
    world.diplomacy.make_peace(0, 1, 3);

    let before = world.diplomacy.truce_months(0, 1);
    world.tick_month();

    assert_eq!(world.diplomacy.truce_months(0, 1), before - 1);
}

#[test]
fn strong_army_captures_province() {
    let mut world = world();
    world.factions[0].army = 100_000;
    world.factions[1].army = 0;
    world.diplomacy.declare_war(0, 1, 1, 1);

    let target = first_province_of(&world, 1);
    world.queue_command(Command::Attack {
        faction: 0,
        province: target,
    });
    world.tick_month();

    assert_eq!(world.provinces[target].owner, 0);
}

#[test]
fn weak_army_fails_to_capture() {
    let mut world = world();
    world.factions[0].army = 1;
    world.factions[1].army = 10_000;
    world.diplomacy.declare_war(0, 1, 1, 1);

    let target = first_province_of(&world, 1);
    world.queue_command(Command::Attack {
        faction: 0,
        province: target,
    });
    world.tick_month();

    assert_eq!(world.provinces[target].owner, 1);
}

#[test]
fn war_adds_unrest_and_attrition() {
    let mut world = world();
    world.factions[0].army = 1_000;
    world.factions[1].army = 1_000;

    let target = first_province_of(&world, 1);
    let before_target = world.explain_unrest(target).unwrap().target;

    world.diplomacy.declare_war(0, 1, 1, 1);
    let after_target = world.explain_unrest(target).unwrap().target;
    assert!(after_target > before_target);

    world.tick_month();
    assert!(world.factions[0].army < 1_000);
    assert!(world.factions[1].army < 1_000);
}

#[test]
fn conquest_of_last_province_ends_war() {
    let mut world = world();
    world.factions[0].army = 1_000_000;
    world.diplomacy.declare_war(0, 2, 1, 1);

    let targets: Vec<usize> = world
        .provinces
        .iter()
        .filter(|province| province.owner == 2)
        .map(|province| province.id)
        .collect();

    for target in targets {
        world.queue_command(Command::Attack {
            faction: 0,
            province: target,
        });
    }

    world.tick_month();

    assert_eq!(
        world
            .provinces
            .iter()
            .filter(|province| province.owner == 2)
            .count(),
        0
    );
    assert!(!world.diplomacy.is_at_war(0, 2));
}

#[test]
fn war_history_is_deterministic() {
    let mut a = world();
    let mut b = world();

    for world in [&mut a, &mut b] {
        world.factions[0].army = 5_000;
        world.queue_command(Command::DeclareWar {
            attacker: 0,
            defender: 1,
        });
        let target = first_province_of(world, 1);
        world.queue_command(Command::Attack {
            faction: 0,
            province: target,
        });
    }

    for _ in 0..120 {
        a.tick_month();
        b.tick_month();
    }

    assert_eq!(a, b);
}
