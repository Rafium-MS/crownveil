use serde::{Deserialize, Serialize};

pub type ModifierId = usize;

/// Effects are additive across all active modifiers. Percentages are integer
/// percentage points (e.g. `income_pct: -20` = 20% less income).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierEffects {
    /// Added to the unrest target of the province (or of every province of the faction).
    pub unrest: i64,
    /// Percentage change to tax income.
    pub income_pct: i64,
    /// Percentage change to food capacity. Only meaningful on provinces.
    pub food_pct: i64,
    /// Monthly legitimacy change. Only meaningful on factions.
    pub legitimacy: i64,
}

impl ModifierEffects {
    pub fn combine(self, other: Self) -> Self {
        Self {
            unrest: self.unrest + other.unrest,
            income_pct: self.income_pct + other.income_pct,
            food_pct: self.food_pct + other.food_pct,
            legitimacy: self.legitimacy + other.legitimacy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierDef {
    pub key: String,
    pub name: String,
    pub effects: ModifierEffects,
}

/// The effects are copied from the definition so rule functions can work from a
/// province or faction alone, without access to the world's definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveModifier {
    pub modifier: ModifierId,
    /// `None` means permanent.
    pub months_left: Option<i64>,
    pub effects: ModifierEffects,
}

pub fn total_effects(modifiers: &[ActiveModifier]) -> ModifierEffects {
    modifiers
        .iter()
        .fold(ModifierEffects::default(), |total, active| {
            total.combine(active.effects)
        })
}

pub fn has_modifier(modifiers: &[ActiveModifier], modifier: ModifierId) -> bool {
    modifiers.iter().any(|active| active.modifier == modifier)
}

/// Re-applying an existing modifier refreshes its duration instead of stacking.
pub fn add_modifier(
    modifiers: &mut Vec<ActiveModifier>,
    def: &ModifierDef,
    modifier: ModifierId,
    months: Option<i64>,
) {
    let months = months.map(|months| months.max(1));
    if let Some(active) = modifiers
        .iter_mut()
        .find(|active| active.modifier == modifier)
    {
        active.months_left = match (active.months_left, months) {
            (Some(current), Some(new)) => Some(current.max(new)),
            _ => None,
        };
        return;
    }

    modifiers.push(ActiveModifier {
        modifier,
        months_left: months,
        effects: def.effects,
    });
}

pub fn remove_modifier(modifiers: &mut Vec<ActiveModifier>, modifier: ModifierId) {
    modifiers.retain(|active| active.modifier != modifier);
}

pub fn tick_modifiers(modifiers: &mut Vec<ActiveModifier>) {
    for active in modifiers.iter_mut() {
        if let Some(months) = active.months_left.as_mut() {
            *months -= 1;
        }
    }
    modifiers.retain(|active| active.months_left.map_or(true, |months| months > 0));
}

/// Applies a percentage change without letting the result flip sign.
pub fn apply_pct(value: i64, pct: i64) -> i64 {
    value * (100 + pct).max(0) / 100
}
