use crownveil_sim::{Command, World};

fn world() -> World {
    World::demo(99)
}

#[test]
fn set_tax_is_applied_on_next_tick_and_clamped() {
    let mut world = world();
    world.queue_command(Command::SetTax {
        faction: 0,
        rate: 250,
    });
    world.tick_month();

    assert_eq!(world.factions[0].tax_rate, 100);
    assert!(world.pending_commands.is_empty());
}

#[test]
fn recruit_spends_treasury_and_adds_army() {
    let mut world = world();
    world.factions[0].treasury = 1000;
    world.queue_command(Command::Recruit {
        faction: 0,
        soldiers: 100,
    });
    world.tick_month();

    assert_eq!(world.factions[0].army, 100);
}

#[test]
fn recruit_without_funds_does_nothing() {
    let mut world = world();
    world.factions[0].treasury = 0;
    world.queue_command(Command::Recruit {
        faction: 0,
        soldiers: 100,
    });
    world.tick_month();

    assert_eq!(world.factions[0].army, 0);
}

#[test]
fn disband_cannot_go_below_zero() {
    let mut world = world();
    world.factions[1].army = 50;
    world.queue_command(Command::Disband {
        faction: 1,
        soldiers: 999,
    });
    world.tick_month();

    assert_eq!(world.factions[1].army, 0);
}

#[test]
fn commands_are_deterministic() {
    let mut a = world();
    let mut b = world();

    for world in [&mut a, &mut b] {
        world.queue_command(Command::SetTax {
            faction: 0,
            rate: 40,
        });
        world.queue_command(Command::Recruit {
            faction: 0,
            soldiers: 50,
        });
    }

    for _ in 0..120 {
        a.tick_month();
        b.tick_month();
    }

    assert_eq!(a, b);
}
