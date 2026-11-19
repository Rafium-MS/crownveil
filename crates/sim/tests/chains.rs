use crownveil_sim::{
    trigger_event, Command, Condition, Effect, EventDef, EventOption, EventScope, EventTarget,
    World,
};

const PLAYER: usize = 0;
const START: usize = 0;
const FOLLOW_UP: usize = 1;

fn event(
    key: &str,
    scope: EventScope,
    conditions: Vec<Condition>,
    effects: Vec<Effect>,
    options: Vec<EventOption>,
) -> EventDef {
    EventDef {
        key: key.to_string(),
        title: key.to_string(),
        description: String::new(),
        scope,
        chance: 0,
        cooldown_months: 0,
        conditions,
        effects,
        options,
    }
}

fn chained(follow_up_scope: EventScope, follow_up_conditions: Vec<Condition>) -> World {
    World::demo(1).with_events(
        Vec::new(),
        vec![
            event(
                "start",
                EventScope::Province,
                Vec::new(),
                vec![Effect::TriggerEvent {
                    event: FOLLOW_UP,
                    months: 3,
                }],
                Vec::new(),
            ),
            event(
                "follow_up",
                follow_up_scope,
                follow_up_conditions,
                vec![Effect::Treasury(1_000)],
                Vec::new(),
            ),
        ],
    )
}

fn follow_up_fired(world: &World) -> bool {
    world
        .event_log
        .iter()
        .any(|record| record.event == FOLLOW_UP)
}

#[test]
fn follow_up_fires_after_delay_even_with_zero_chance() {
    let mut world = chained(EventScope::Province, Vec::new());
    trigger_event(&mut world, START, EventTarget::Province(1));
    assert_eq!(world.scheduled_events.len(), 1);
    assert_eq!(world.scheduled_events[0].source, Some(START));

    for _ in 0..2 {
        world.tick_month();
    }
    assert!(!follow_up_fired(&world));

    world.tick_month();
    assert!(follow_up_fired(&world));
    assert!(world.scheduled_events.is_empty());
    assert_eq!(
        world.event_log.last().unwrap().target,
        EventTarget::Province(1)
    );
}

#[test]
fn follow_up_is_averted_when_conditions_fail() {
    let mut world = chained(EventScope::Province, vec![Condition::UnrestAtLeast(90)]);
    trigger_event(&mut world, START, EventTarget::Province(1));
    for _ in 0..6 {
        world.tick_month();
    }
    assert!(!follow_up_fired(&world));
    assert!(
        world.scheduled_events.is_empty(),
        "crise evitada é descartada"
    );
}

#[test]
fn faction_follow_up_of_province_event_hits_the_owner() {
    let mut world = chained(EventScope::Faction, Vec::new());
    trigger_event(&mut world, START, EventTarget::Province(4));
    assert_eq!(world.scheduled_events[0].target, EventTarget::Faction(1));
    assert_eq!(world.looming_for(1).count(), 1);
    assert_eq!(world.looming_for(PLAYER).count(), 0);
}

#[test]
fn province_follow_up_of_faction_event_is_ignored() {
    let mut world = chained(EventScope::Province, Vec::new());
    world.events[START].scope = EventScope::Faction;
    trigger_event(&mut world, START, EventTarget::Faction(PLAYER));
    assert!(world.scheduled_events.is_empty());
}

#[test]
fn chosen_option_decides_which_chain_continues() {
    let mut world = World::demo(1)
        .with_events(
            Vec::new(),
            vec![
                event(
                    "dilema",
                    EventScope::Province,
                    Vec::new(),
                    Vec::new(),
                    vec![
                        EventOption {
                            label: "Escalar".to_string(),
                            effects: vec![Effect::TriggerEvent {
                                event: FOLLOW_UP,
                                months: 2,
                            }],
                        },
                        EventOption {
                            label: "Apaziguar".to_string(),
                            effects: vec![Effect::Unrest(-5)],
                        },
                    ],
                ),
                event(
                    "consequencia",
                    EventScope::Province,
                    Vec::new(),
                    vec![Effect::Treasury(1_000)],
                    Vec::new(),
                ),
            ],
        )
        .with_player(Some(PLAYER));

    trigger_event(&mut world, START, EventTarget::Province(0));
    let decision = world.pending_decisions[0].id;
    world.queue_command(Command::ChooseOption {
        faction: PLAYER,
        decision,
        option: 1,
    });
    for _ in 0..6 {
        world.tick_month();
    }
    assert!(!follow_up_fired(&world));

    // The AI always picks the first option, so it walks into the chain.
    trigger_event(&mut world, START, EventTarget::Province(4));
    assert_eq!(world.scheduled_events.len(), 1);
    assert_eq!(world.looming_for(1).count(), 1);
}

#[test]
fn chains_can_loop_without_stalling_a_tick() {
    let mut world = World::demo(1).with_events(
        Vec::new(),
        vec![
            event(
                "a",
                EventScope::Province,
                Vec::new(),
                vec![Effect::TriggerEvent {
                    event: 1,
                    months: 1,
                }],
                Vec::new(),
            ),
            event(
                "b",
                EventScope::Province,
                Vec::new(),
                vec![Effect::TriggerEvent {
                    event: 0,
                    months: 1,
                }],
                Vec::new(),
            ),
        ],
    );
    trigger_event(&mut world, 0, EventTarget::Province(0));
    for _ in 0..10 {
        world.tick_month();
    }
    assert_eq!(world.event_log.len(), 11, "um elo por mês");
    assert_eq!(world.scheduled_events.len(), 1);
}

#[test]
fn chains_are_deterministic() {
    let run = || {
        let mut world = chained(EventScope::Faction, vec![Condition::AtPeace]);
        world.events[START].chance = 30;
        for _ in 0..600 {
            world.tick_month();
        }
        world
    };
    let world = run();
    assert!(follow_up_fired(&world));
    assert_eq!(world, run());
}
