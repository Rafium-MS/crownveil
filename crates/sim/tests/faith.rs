use crownveil_sim::rules::{province_faith, province_mana, MAX_RESOURCE};
use crownveil_sim::{
    check_rite, Command, Condition, Effect, EventScope, EventTarget, ModifierDef, ModifierEffects,
    RiteBlock, RiteDef, World,
};

const PLAYER: usize = 0;
const BLESSING: usize = 0;

fn blessing() -> ModifierDef {
    ModifierDef {
        key: "blessing".to_string(),
        name: "Bênção".to_string(),
        effects: ModifierEffects {
            faith: 5,
            mana: 3,
            ..Default::default()
        },
    }
}

fn rite(scope: EventScope, faith: i64, mana: i64, gold: i64, effects: Vec<Effect>) -> RiteDef {
    RiteDef {
        key: "rito".to_string(),
        name: "Rito".to_string(),
        description: String::new(),
        scope,
        faith_cost: faith,
        mana_cost: mana,
        gold_cost: gold,
        cooldown_months: 6,
        conditions: Vec::new(),
        effects,
    }
}

fn world(rites: Vec<RiteDef>) -> World {
    let mut world = World::demo(1)
        .with_events(vec![blessing()], Vec::new())
        .with_rites(rites);
    world.factions[PLAYER] = world.factions[PLAYER].clone().with_resources(300, 300);
    world
}

fn perform(world: &mut World, rite: usize, province: Option<usize>) {
    world.queue_command(Command::PerformRite {
        faction: PLAYER,
        rite,
        province,
    });
    world.tick_month();
}

#[test]
fn faith_comes_from_clergy_and_mana_from_development() {
    let mut world = World::demo(1);
    let faith: i64 = world
        .provinces
        .iter()
        .filter(|p| p.owner == PLAYER)
        .map(province_faith)
        .sum();
    let mana: i64 = world
        .provinces
        .iter()
        .filter(|p| p.owner == PLAYER)
        .map(province_mana)
        .sum();
    assert_eq!(faith, 400 / 100 + 250 / 100 + 350 / 100);
    assert_eq!(mana, 55 / 25 + 45 / 25 + 60 / 25);

    world.tick_month();
    assert_eq!(world.factions[PLAYER].faith, faith);
    assert_eq!(world.factions[PLAYER].mana, mana);
}

#[test]
fn resources_are_capped() {
    let mut world = world(Vec::new());
    let mut flood = blessing();
    flood.effects.faith = 500;
    flood.effects.mana = 500;
    crownveil_sim::modifier::add_modifier(
        &mut world.factions[PLAYER].modifiers,
        &flood,
        BLESSING,
        None,
    );
    world.factions[PLAYER].faith = MAX_RESOURCE;
    world.factions[PLAYER].mana = MAX_RESOURCE;
    world.tick_month();
    assert_eq!(world.factions[PLAYER].faith, MAX_RESOURCE);
    assert_eq!(world.factions[PLAYER].mana, MAX_RESOURCE);
}

#[test]
fn unspent_resources_decay_towards_equilibrium() {
    let mut world = World::demo(1);
    world.factions[PLAYER].faith = 1000;
    world.tick_month();
    // 1000 - 1000/50 + 9 (clergy)
    assert_eq!(world.factions[PLAYER].faith, 989);

    for _ in 0..600 {
        world.tick_month();
    }
    let faith = world.factions[PLAYER].faith;
    assert!(
        (300..700).contains(&faith),
        "estoque deve se estabilizar perto de renda * 50, veio {faith}"
    );
}

#[test]
fn modifiers_add_monthly_faith_and_mana() {
    let mut plain = world(Vec::new());
    let mut blessed = world(Vec::new());
    crownveil_sim::modifier::add_modifier(
        &mut blessed.factions[PLAYER].modifiers,
        &blessing(),
        BLESSING,
        None,
    );
    crownveil_sim::modifier::add_modifier(
        &mut blessed.provinces[0].modifiers,
        &blessing(),
        BLESSING,
        None,
    );
    plain.tick_month();
    blessed.tick_month();
    assert_eq!(
        blessed.factions[PLAYER].faith,
        plain.factions[PLAYER].faith + 10
    );
    assert_eq!(
        blessed.factions[PLAYER].mana,
        plain.factions[PLAYER].mana + 6
    );
}

#[test]
fn faith_calms_and_magic_frightens() {
    let mut world = World::demo(1);
    let baseline = world.explain_unrest(0).unwrap();
    assert_eq!((baseline.breakdown.faith, baseline.breakdown.magic), (0, 0));

    world.factions[PLAYER].faith = 1000;
    world.factions[PLAYER].mana = 450;
    let explanation = world.explain_unrest(0).unwrap();
    assert_eq!(explanation.breakdown.faith, -5);
    assert_eq!(explanation.breakdown.magic, 2);
    assert_eq!(explanation.target, baseline.target - 3);
}

#[test]
fn faction_rite_pays_costs_and_applies_effects() {
    let mut world = world(vec![rite(
        EventScope::Faction,
        100,
        50,
        200,
        vec![Effect::Legitimacy(10), Effect::Unrest(-30)],
    )]);
    let before = world.clone();

    perform(&mut world, 0, None);

    let mut twin = before.clone();
    twin.tick_month();
    let caster = &world.factions[PLAYER];
    // Paying first means slightly less decay (2% of the cost) this month.
    assert_eq!(caster.faith, twin.factions[PLAYER].faith - 100 + 2);
    assert_eq!(caster.mana, twin.factions[PLAYER].mana - 50 + 1);
    assert_eq!(caster.treasury, twin.factions[PLAYER].treasury - 200);
    assert!(caster.legitimacy > twin.factions[PLAYER].legitimacy);
    for province in world.provinces.iter().filter(|p| p.owner == PLAYER) {
        assert!(province.unrest < twin.provinces[province.id].unrest);
    }
}

#[test]
fn province_rite_requires_owned_province() {
    let mut world = world(vec![rite(
        EventScope::Province,
        10,
        0,
        0,
        vec![Effect::Development(5)],
    )]);
    assert_eq!(
        check_rite(&world, PLAYER, 0, EventTarget::Province(3)),
        Err(RiteBlock::NotOwner)
    );
    assert_eq!(
        check_rite(&world, PLAYER, 0, EventTarget::Faction(PLAYER)),
        Err(RiteBlock::WrongScope)
    );

    perform(&mut world, 0, Some(3));
    assert_eq!(
        world.provinces[3].development, 50,
        "província alheia não muda"
    );

    perform(&mut world, 0, Some(1));
    assert_eq!(world.provinces[1].development, 50);
}

#[test]
fn rite_is_blocked_without_resources_and_on_cooldown() {
    let mut world = world(vec![rite(
        EventScope::Faction,
        200,
        0,
        0,
        vec![Effect::Treasury(1)],
    )]);
    let target = EventTarget::Faction(PLAYER);
    assert_eq!(check_rite(&world, PLAYER, 0, target), Ok(()));

    // Performed during month m: available again in month m + 6, i.e. five
    // ticks after the one that applied it.
    perform(&mut world, 0, None);
    assert_eq!(
        check_rite(&world, PLAYER, 0, target),
        Err(RiteBlock::OnCooldown { months_left: 5 })
    );

    world.factions[PLAYER].faith = 1000;
    for _ in 0..4 {
        world.tick_month();
    }
    assert!(matches!(
        check_rite(&world, PLAYER, 0, target),
        Err(RiteBlock::OnCooldown { months_left: 1 })
    ));
    world.tick_month();
    assert_eq!(check_rite(&world, PLAYER, 0, target), Ok(()));

    world.factions[PLAYER].faith = 199;
    assert_eq!(
        check_rite(&world, PLAYER, 0, target),
        Err(RiteBlock::NotEnoughFaith)
    );
}

#[test]
fn rite_conditions_are_checked() {
    let mut conditioned = rite(EventScope::Province, 0, 0, 0, vec![Effect::Unrest(-5)]);
    conditioned.conditions = vec![Condition::HasModifier(BLESSING)];
    let mut world = world(vec![conditioned]);

    assert_eq!(
        check_rite(&world, PLAYER, 0, EventTarget::Province(0)),
        Err(RiteBlock::ConditionsUnmet)
    );
    crownveil_sim::modifier::add_modifier(
        &mut world.provinces[0].modifiers,
        &blessing(),
        BLESSING,
        None,
    );
    assert_eq!(
        check_rite(&world, PLAYER, 0, EventTarget::Province(0)),
        Ok(())
    );
}

#[test]
fn faith_and_mana_effects_and_conditions_work_in_events() {
    let mut world = world(Vec::new());
    world.events = vec![crownveil_sim::EventDef {
        key: "surto".to_string(),
        title: "Surto".to_string(),
        description: String::new(),
        scope: EventScope::Faction,
        chance: 1000,
        cooldown_months: 600,
        conditions: vec![Condition::ManaAtLeast(250), Condition::FaithAtLeast(250)],
        effects: vec![Effect::Mana(-2000), Effect::Faith(50)],
        options: Vec::new(),
    }];

    world.tick_month();
    assert_eq!(world.event_log.len(), 1);
    assert_eq!(world.factions[PLAYER].mana, 0);
    assert!(world.factions[PLAYER].faith > 250);
}
