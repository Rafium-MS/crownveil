use serde::{Deserialize, Serialize};

use crate::world::FactionId;

pub const MIN_RELATION: i64 = -100;
pub const MAX_RELATION: i64 = 100;
pub const DEFAULT_TRUCE_MONTHS: i64 = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct War {
    pub attacker: FactionId,
    pub defender: FactionId,
    pub started_year: i64,
    pub started_month: u32,
}

impl War {
    pub fn involves(&self, a: FactionId, b: FactionId) -> bool {
        (self.attacker == a && self.defender == b) || (self.attacker == b && self.defender == a)
    }

    pub fn has(&self, faction: FactionId) -> bool {
        self.attacker == faction || self.defender == faction
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diplomacy {
    pub relations: Vec<Vec<i64>>,
    pub truces: Vec<Vec<i64>>,
    pub wars: Vec<War>,
}

impl Diplomacy {
    pub fn new(faction_count: usize) -> Self {
        Self {
            relations: vec![vec![0; faction_count]; faction_count],
            truces: vec![vec![0; faction_count]; faction_count],
            wars: Vec::new(),
        }
    }

    fn valid(&self, a: FactionId, b: FactionId) -> bool {
        a != b && a < self.relations.len() && b < self.relations.len()
    }

    pub fn relation(&self, a: FactionId, b: FactionId) -> i64 {
        if !self.valid(a, b) {
            return 0;
        }
        self.relations[a][b]
    }

    pub fn set_relation(&mut self, a: FactionId, b: FactionId, value: i64) {
        if !self.valid(a, b) {
            return;
        }
        let value = value.clamp(MIN_RELATION, MAX_RELATION);
        self.relations[a][b] = value;
        self.relations[b][a] = value;
    }

    pub fn adjust_relation(&mut self, a: FactionId, b: FactionId, delta: i64) {
        let value = self.relation(a, b) + delta;
        self.set_relation(a, b, value);
    }

    pub fn truce_months(&self, a: FactionId, b: FactionId) -> i64 {
        if !self.valid(a, b) {
            return 0;
        }
        self.truces[a][b]
    }

    pub fn is_at_war(&self, a: FactionId, b: FactionId) -> bool {
        self.wars.iter().any(|war| war.involves(a, b))
    }

    pub fn war(&self, a: FactionId, b: FactionId) -> Option<&War> {
        self.wars.iter().find(|war| war.involves(a, b))
    }

    pub fn war_count_for(&self, faction: FactionId) -> usize {
        self.wars.iter().filter(|war| war.has(faction)).count()
    }

    pub fn declare_war(
        &mut self,
        attacker: FactionId,
        defender: FactionId,
        year: i64,
        month: u32,
    ) -> bool {
        if !self.valid(attacker, defender) {
            return false;
        }
        if self.is_at_war(attacker, defender) || self.truce_months(attacker, defender) > 0 {
            return false;
        }

        self.set_relation(attacker, defender, MIN_RELATION);
        self.wars.push(War {
            attacker,
            defender,
            started_year: year,
            started_month: month,
        });
        true
    }

    pub fn make_peace(&mut self, a: FactionId, b: FactionId, truce_months: i64) -> bool {
        if !self.valid(a, b) {
            return false;
        }

        let before = self.wars.len();
        self.wars.retain(|war| !war.involves(a, b));
        if self.wars.len() == before {
            return false;
        }

        let truce = truce_months.max(0);
        self.truces[a][b] = truce;
        self.truces[b][a] = truce;
        self.set_relation(a, b, MIN_RELATION / 2);
        true
    }

    pub fn tick(&mut self) {
        let count = self.relations.len();
        for a in 0..count {
            for b in (a + 1)..count {
                if self.truces[a][b] > 0 {
                    self.truces[a][b] -= 1;
                    self.truces[b][a] = self.truces[a][b];
                }

                if !self.is_at_war(a, b) {
                    let relation = self.relations[a][b];
                    let next = if relation < 0 {
                        relation + 1
                    } else if relation > 0 {
                        relation - 1
                    } else {
                        0
                    };
                    self.relations[a][b] = next;
                    self.relations[b][a] = next;
                }
            }
        }
    }
}
