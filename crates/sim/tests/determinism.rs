use crownveil_sim::World;

fn run(seed: u64, months: usize) -> World {
    let mut world = World::demo(seed);
    for _ in 0..months {
        world.tick_month();
    }
    world
}

#[test]
fn same_seed_produces_identical_history() {
    let a = run(12_345, 1_200);
    let b = run(12_345, 1_200);
    assert_eq!(a, b);
}

#[test]
fn different_seeds_diverge() {
    let a = run(1, 1_200);
    let b = run(2, 1_200);
    assert_ne!(a.provinces, b.provinces);
}

#[test]
fn populations_and_ranges_stay_valid() {
    let world = run(7, 2_400);

    for province in &world.provinces {
        assert!(province.pops.total() >= 0);
        assert!((0..=100).contains(&province.unrest));
    }

    for faction in &world.factions {
        assert!((0..=100).contains(&faction.legitimacy));
    }
}
