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

#[test]
fn corruption_drains_hoarded_gold_but_not_debt() {
    use crownveil_sim::rules::{corruption, CORRUPTION_DIVISOR};

    let mut world = World::demo(1);
    world.factions[0].treasury = 10_000;
    assert_eq!(corruption(&world.factions[0]), 10_000 / CORRUPTION_DIVISOR);
    world.factions[0].treasury = -800;
    assert_eq!(corruption(&world.factions[0]), 0);
}

#[test]
fn budget_matches_the_actual_monthly_change() {
    let mut world = World::demo(1);
    world.factions[0].treasury = 5_000;
    world.factions[0].army = 400;
    let budget = world.budget(0).unwrap();
    assert!(budget.corruption > 0 && budget.army == 40);

    let before = world.factions[0].treasury;
    world.tick_month();
    // Tick order: income first, then costs; corruption sees the treasury
    // after income, so it can differ from the estimate by a point or two.
    let change = world.factions[0].treasury - before;
    assert!(
        (change - budget.net()).abs() <= 3,
        "saldo previsto {} vs real {change}",
        budget.net()
    );
}

#[test]
fn treasury_settles_instead_of_growing_forever() {
    let mut world = World::demo(1);
    for _ in 0..1_200 {
        world.tick_month();
    }
    let first_century = world.factions[0].treasury;
    for _ in 0..1_200 {
        world.tick_month();
    }
    let second_century = world.factions[0].treasury;
    assert!(
        first_century < 20_000,
        "tesouro após 100 anos: {first_century}"
    );
    assert!(
        (second_century - first_century).abs() < first_century / 5,
        "deveria estar estável: {first_century} -> {second_century}"
    );
}
