use crownveil_sim::{food_capacity, World};

#[test]
fn population_stays_bounded_by_food_capacity() {
    let mut world = World::demo(5);
    for _ in 0..2_400 {
        world.tick_month();
    }

    for province in &world.provinces {
        let capacity = food_capacity(province);
        let total = province.pops.total();
        assert!(
            total <= capacity + capacity / 5 + 1_000,
            "{}: população {} excedeu a capacidade {}",
            province.name,
            total,
            capacity
        );
    }
}

#[test]
fn famine_kills_and_raises_unrest() {
    let mut world = World::demo(1);
    let capacity = food_capacity(&world.provinces[0]);
    world.provinces[0].pops.peasant = capacity * 2;

    let before_population = world.provinces[0].pops.total();
    let before_unrest = world.provinces[0].unrest;
    world.tick_month();

    assert!(world.provinces[0].pops.total() < before_population);
    assert!(world.provinces[0].unrest > before_unrest);
}

#[test]
fn higher_tax_raises_unrest_target() {
    let mut world = World::demo(1);

    world.factions[0].tax_rate = 10;
    let low = world.explain_unrest(0).unwrap().target;

    world.factions[0].tax_rate = 80;
    let high = world.explain_unrest(0).unwrap().target;

    assert!(high > low);
}
