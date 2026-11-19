use crate::modifier::{apply_pct, total_effects};
use crate::world::{Faction, Province};

pub const MONTHS_PER_YEAR: u32 = 12;
pub const FOOD_PER_DEVELOPMENT: i64 = 400;

pub fn food_capacity(province: &Province) -> i64 {
    let base = province.development.max(0) * FOOD_PER_DEVELOPMENT;
    apply_pct(base, total_effects(&province.modifiers).food_pct)
}

pub fn population_density(province: &Province) -> i64 {
    let capacity = food_capacity(province);
    if capacity <= 0 {
        return 100;
    }
    province.pops.total() * 100 / capacity
}

pub fn is_famine(province: &Province) -> bool {
    province.pops.total() > food_capacity(province)
}

pub fn tax_base(province: &Province) -> i64 {
    let base = province.pops.peasant / 100 + province.pops.burgher / 50 + province.pops.noble / 20;
    base * province.development / 50
}

pub fn province_income(province: &Province, faction: &Faction) -> i64 {
    let pct = total_effects(&province.modifiers).income_pct
        + total_effects(&faction.modifiers).income_pct;
    apply_pct(tax_base(province) * faction.tax_rate / 100, pct)
}

pub fn province_upkeep(province: &Province) -> i64 {
    province.pops.clergy / 100 + province.pops.noble / 100
}

pub fn army_upkeep(faction: &Faction) -> i64 {
    faction.army / 10
}

pub fn garrison(province: &Province) -> i64 {
    province.development.max(0) * 10 + province.pops.total() / 500
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnrestBreakdown {
    pub legitimacy: i64,
    pub treasury: i64,
    pub tax: i64,
    pub food: i64,
    pub war: i64,
    /// Sum of province and owner modifiers; may be negative.
    pub modifiers: i64,
}

impl UnrestBreakdown {
    pub fn target(&self) -> i64 {
        (self.legitimacy + self.treasury + self.tax + self.food + self.war + self.modifiers)
            .clamp(0, 100)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnrestExplanation {
    pub current: i64,
    pub target: i64,
    pub breakdown: UnrestBreakdown,
}

pub fn unrest_breakdown(province: &Province, faction: &Faction, at_war: bool) -> UnrestBreakdown {
    let legitimacy = (100 - faction.legitimacy) / 2;
    let treasury = if faction.treasury < 0 { 20 } else { 0 };
    let tax = faction.tax_rate / 5;
    let food = if is_famine(province) {
        10
    } else {
        let density = population_density(province);
        if density > 90 {
            (density - 90) / 2
        } else {
            0
        }
    };
    let war = if at_war { 8 } else { 0 };
    let modifiers =
        total_effects(&province.modifiers).unrest + total_effects(&faction.modifiers).unrest;

    UnrestBreakdown {
        legitimacy,
        treasury,
        tax,
        food,
        war,
        modifiers,
    }
}

pub fn unrest_target(province: &Province, faction: &Faction, at_war: bool) -> i64 {
    unrest_breakdown(province, faction, at_war).target()
}

pub fn legitimacy_from_modifiers(faction: &Faction) -> i64 {
    total_effects(&faction.modifiers).legitimacy
}
