use crownveil_sim::rules::{province_income, unrest_target};
use crownveil_sim::{
    food_capacity, trigger_event, Condition, Effect, EventDef, EventScope, EventTarget,
    ModifierDef, ModifierEffects, World,
};

const BLESSING: usize = 0;
const PLAGUE: usize = 1;

fn modifiers() -> Vec<ModifierDef> {
    vec![
        ModifierDef {
            key: "blessing".to_string(),
            name: "Bênção".to_string(),
            effects: ModifierEffects {
                unrest: -10,
                income_pct: 50,
                food_pct: 25,
                legitimacy: 2,
            },
        },
        ModifierDef {
            key: "plague".to_string(),
            name: "Peste".to_string(),
            effects: ModifierEffects {
                unrest: 15,
                income_pct: -50,
                food_pct: -100,
                legitimacy: -1,
            },
        },
    ]
}

fn event(
    scope: EventScope,
    chance: i64,
    conditions: Vec<Condition>,
    effects: Vec<Effect>,
) -> EventDef {
    EventDef {
        key: "test".to_string(),
        title: "Teste".to_string(),
        description: String::new(),
        scope,
        chance,
        cooldown_months: 12,
        conditions,
        effects,
    }
}

fn world_with(events: Vec<EventDef>) -> World {
    World::demo(1).with_events(modifiers(), events)
}

fn add_blessing(months: Option<i64>) -> Effect {
    Effect::AddModifier {
        modifier: BLESSING,
        months,
    }
}

#[test]
fn province_modifier_changes_food_income_and_unrest() {
    let mut world = world_with(vec![event(
        EventScope::Province,
        0,
        vec![],
        vec![add_blessing(None)],
    )]);
    let province = world.provinces[0].clone();
    let faction = world.factions[province.owner].clone();
    let base_food = food_capacity(&province);
    let base_income = province_income(&province, &faction);
    let base_unrest = unrest_target(&province, &faction, false);

    trigger_event(&mut world, 0, EventTarget::Province(0));

    let province = &world.provinces[0];
    assert_eq!(food_capacity(province), base_food * 125 / 100);
    assert_eq!(province_income(province, &faction), base_income * 150 / 100);
    assert_eq!(
        unrest_target(province, &faction, false),
        (base_unrest - 10).max(0)
    );
    assert_eq!(world.explain_unrest(0).unwrap().breakdown.modifiers, -10);
}

#[test]
fn faction_modifier_applies_to_all_provinces_and_legitimacy() {
    let mut world = world_with(vec![event(
        EventScope::Faction,
        0,
        vec![],
        vec![Effect::AddModifier {
            modifier: PLAGUE,
            months: None,
        }],
    )]);
    trigger_event(&mut world, 0, EventTarget::Faction(0));

    for province in world.provinces.iter().filter(|p| p.owner == 0) {
        assert_eq!(
            world
                .explain_unrest(province.id)
                .unwrap()
                .breakdown
                .modifiers,
            15
        );
        assert!(
            food_capacity(province) > 0,
            "food_pct de facção não afeta províncias"
        );
    }
    for province in world.provinces.iter().filter(|p| p.owner != 0) {
        assert_eq!(
            world
                .explain_unrest(province.id)
                .unwrap()
                .breakdown
                .modifiers,
            0
        );
    }

    // Faction 0 starts at 60: -1 natural drift towards 50, -1 from the modifier.
    world.tick_month();
    assert_eq!(world.factions[0].legitimacy, 58);
}

#[test]
fn food_capacity_never_goes_negative() {
    let mut world = world_with(vec![event(
        EventScope::Province,
        0,
        vec![],
        vec![Effect::AddModifier {
            modifier: PLAGUE,
            months: None,
        }],
    )]);
    trigger_event(&mut world, 0, EventTarget::Province(0));
    assert_eq!(food_capacity(&world.provinces[0]), 0);
}

#[test]
fn timed_modifier_expires_and_reapplying_refreshes() {
    let mut world = world_with(vec![event(
        EventScope::Province,
        0,
        vec![],
        vec![add_blessing(Some(3))],
    )]);
    trigger_event(&mut world, 0, EventTarget::Province(0));

    for _ in 0..2 {
        world.tick_month();
    }
    assert_eq!(world.provinces[0].modifiers.len(), 1);
    assert_eq!(world.provinces[0].modifiers[0].months_left, Some(1));

    trigger_event(&mut world, 0, EventTarget::Province(0));
    assert_eq!(world.provinces[0].modifiers.len(), 1, "não acumula");
    assert_eq!(world.provinces[0].modifiers[0].months_left, Some(3));

    for _ in 0..3 {
        world.tick_month();
    }
    assert!(world.provinces[0].modifiers.is_empty());
}

#[test]
fn certain_event_fires_once_per_target_until_cooldown_ends() {
    let mut world = world_with(vec![event(
        EventScope::Province,
        1000,
        vec![],
        vec![Effect::Treasury(1)],
    )]);
    let provinces = world.provinces.len();

    world.tick_month();
    assert_eq!(world.event_log.len(), provinces);
    assert_eq!(world.event_cooldowns.len(), provinces);

    for _ in 0..11 {
        world.tick_month();
    }
    assert_eq!(world.event_log.len(), provinces, "em cooldown por 12 meses");

    world.tick_month();
    assert_eq!(world.event_log.len(), provinces * 2);
}

#[test]
fn conditions_gate_events() {
    let mut world = world_with(vec![
        event(
            EventScope::Province,
            1000,
            vec![Condition::UnrestAtLeast(101)],
            vec![Effect::Unrest(50)],
        ),
        event(
            EventScope::Faction,
            1000,
            vec![Condition::AtWar],
            vec![Effect::Legitimacy(-20)],
        ),
    ]);

    world.tick_month();
    assert!(world.event_log.is_empty());
}

#[test]
fn province_only_conditions_are_false_for_factions() {
    let mut world = world_with(vec![event(
        EventScope::Faction,
        1000,
        vec![Condition::NoFamine],
        vec![Effect::Treasury(1)],
    )]);
    world.tick_month();
    assert!(world.event_log.is_empty());
}

#[test]
fn modifier_conditions_see_active_modifiers() {
    let mut world = world_with(vec![
        event(EventScope::Province, 0, vec![], vec![add_blessing(None)]),
        event(
            EventScope::Province,
            1000,
            vec![Condition::HasModifier(BLESSING)],
            vec![Effect::RemoveModifier(BLESSING)],
        ),
    ]);
    trigger_event(&mut world, 0, EventTarget::Province(4));

    world.tick_month();
    assert_eq!(world.event_log.len(), 2);
    assert_eq!(world.event_log[1].target, EventTarget::Province(4));
    assert!(world.provinces[4].modifiers.is_empty());
}

#[test]
fn faction_effects_spread_to_provinces_and_clamp() {
    let mut world = world_with(vec![event(
        EventScope::Faction,
        0,
        vec![],
        vec![
            Effect::Unrest(500),
            Effect::Population(-50),
            Effect::Development(-1000),
            Effect::Army(-1_000_000),
        ],
    )]);
    let before = world.provinces[3].pops.total();
    trigger_event(&mut world, 0, EventTarget::Faction(1));

    for province in world.provinces.iter().filter(|p| p.owner == 1) {
        assert_eq!(province.unrest, 100);
        assert_eq!(province.development, 1);
    }
    assert!(world.provinces[3].pops.total() <= before / 2 + 4);
    assert_eq!(world.factions[1].army, 0);
}

#[test]
fn event_log_is_bounded() {
    let mut world = world_with(vec![EventDef {
        cooldown_months: 0,
        ..event(EventScope::Province, 1000, vec![], vec![])
    }]);
    for _ in 0..40 {
        world.tick_month();
    }
    assert_eq!(world.event_log.len(), crownveil_sim::event::EVENT_LOG_LIMIT);
}

#[test]
fn events_are_deterministic() {
    let run = || {
        let mut world = world_with(vec![
            event(
                EventScope::Province,
                50,
                vec![],
                vec![add_blessing(Some(6))],
            ),
            event(EventScope::Faction, 30, vec![], vec![Effect::Treasury(-25)]),
        ]);
        for _ in 0..600 {
            world.tick_month();
        }
        world
    };
    let a = run();
    assert!(!a.event_log.is_empty());
    assert_eq!(a, run());
}
