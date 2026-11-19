use crownveil_sim::event::DECISION_DEADLINE_MONTHS;
use crownveil_sim::{
    trigger_event, Command, Effect, EventDef, EventOption, EventScope, EventTarget, World,
};

const PLAYER: usize = 0;
const AI: usize = 1;

fn choice_event(scope: EventScope, chance: i64) -> EventDef {
    EventDef {
        key: "dilema".to_string(),
        title: "Dilema".to_string(),
        description: String::new(),
        scope,
        chance,
        cooldown_months: 600,
        conditions: Vec::new(),
        effects: vec![Effect::Unrest(1)],
        options: vec![
            EventOption {
                label: "Ouro".to_string(),
                effects: vec![Effect::Treasury(100)],
            },
            EventOption {
                label: "Exército".to_string(),
                effects: vec![Effect::Army(50)],
            },
        ],
    }
}

fn world(scope: EventScope, chance: i64) -> World {
    World::demo(1)
        .with_events(Vec::new(), vec![choice_event(scope, chance)])
        .with_player(Some(PLAYER))
}

#[test]
fn ai_picks_first_option_immediately() {
    let mut world = world(EventScope::Faction, 0);
    let treasury = world.factions[AI].treasury;

    trigger_event(&mut world, 0, EventTarget::Faction(AI));

    assert!(world.pending_decisions.is_empty());
    assert_eq!(world.factions[AI].treasury, treasury + 100);
    assert_eq!(world.event_log.last().unwrap().choice, Some(0));
}

#[test]
fn player_decision_waits_and_applies_chosen_option() {
    let mut world = world(EventScope::Province, 0);
    let unrest = world.provinces[0].unrest;
    let army = world.factions[PLAYER].army;

    trigger_event(&mut world, 0, EventTarget::Province(0));

    assert_eq!(
        world.provinces[0].unrest,
        unrest + 1,
        "efeitos imediatos já valem"
    );
    assert!(world.event_log.is_empty(), "registra só quando decidido");
    let decision = world.decisions_for(PLAYER).next().unwrap().id;

    world.queue_command(Command::ChooseOption {
        faction: PLAYER,
        decision,
        option: 1,
    });
    world.tick_month();

    assert!(world.pending_decisions.is_empty());
    assert_eq!(world.factions[PLAYER].army, army + 50);
    assert_eq!(world.event_log.last().unwrap().choice, Some(1));
}

#[test]
fn invalid_choices_are_ignored() {
    let mut world = world(EventScope::Province, 0);
    trigger_event(&mut world, 0, EventTarget::Province(0));
    let decision = world.pending_decisions[0].id;
    let before = world.factions.clone();

    for command in [
        Command::ChooseOption {
            faction: AI,
            decision,
            option: 0,
        },
        Command::ChooseOption {
            faction: PLAYER,
            decision,
            option: 2,
        },
        Command::ChooseOption {
            faction: PLAYER,
            decision: decision + 1,
            option: 0,
        },
    ] {
        world.queue_command(command);
    }
    world.tick_month();

    assert_eq!(world.pending_decisions.len(), 1);
    assert_eq!(world.factions[PLAYER].army, before[PLAYER].army);
}

#[test]
fn unanswered_decision_defaults_to_first_option() {
    let mut world = world(EventScope::Province, 0);
    trigger_event(&mut world, 0, EventTarget::Province(0));
    let treasury_before = world.factions[PLAYER].treasury;

    for _ in 0..DECISION_DEADLINE_MONTHS - 1 {
        world.tick_month();
    }
    assert_eq!(world.pending_decisions.len(), 1);

    world.tick_month();
    assert!(world.pending_decisions.is_empty());
    assert_eq!(world.event_log.last().unwrap().choice, Some(0));
    // The +100 lands on top of a year of normal economy; compare with a twin
    // world where the decision never existed.
    let mut twin = self::world(EventScope::Province, 0);
    trigger_event(&mut twin, 0, EventTarget::Province(0));
    twin.pending_decisions.clear();
    for _ in 0..DECISION_DEADLINE_MONTHS {
        twin.tick_month();
    }
    assert_eq!(
        world.factions[PLAYER].treasury,
        twin.factions[PLAYER].treasury + 100
    );
    assert!(world.factions[PLAYER].treasury > treasury_before);
}

#[test]
fn only_the_player_gets_pending_decisions() {
    let mut world = world(EventScope::Faction, 1000);
    world.tick_month();

    assert_eq!(world.pending_decisions.len(), 1);
    assert_eq!(world.pending_decisions[0].faction, PLAYER);
    assert_eq!(world.event_log.len(), world.factions.len() - 1);
}

#[test]
fn set_player_command_switches_and_clears_control() {
    let mut world = world(EventScope::Faction, 0);
    world.queue_command(Command::SetPlayer { faction: Some(AI) });
    world.tick_month();
    assert_eq!(world.player, Some(AI));

    world.queue_command(Command::SetPlayer { faction: Some(99) });
    world.tick_month();
    assert_eq!(world.player, None, "facção inexistente vira IA");

    trigger_event(&mut world, 0, EventTarget::Faction(AI));
    assert!(world.pending_decisions.is_empty());
}

#[test]
fn decisions_are_deterministic() {
    let run = || {
        let mut world = world(EventScope::Province, 20);
        for month in 0..240 {
            if month % 7 == 0 {
                let ids: Vec<_> = world.decisions_for(PLAYER).map(|d| d.id).collect();
                for decision in ids {
                    world.queue_command(Command::ChooseOption {
                        faction: PLAYER,
                        decision,
                        option: 1,
                    });
                }
            }
            world.tick_month();
        }
        world
    };
    let a = run();
    assert!(a.event_log.iter().any(|record| record.choice == Some(1)));
    assert_eq!(a, run());
}
