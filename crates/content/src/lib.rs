use std::collections::HashMap;
use std::fmt;

use crownveil_sim::{Faction, Pops, Province, ProvinceId, World};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WorldData {
    pub factions: Vec<FactionData>,
    pub provinces: Vec<ProvinceData>,
}

#[derive(Debug, Deserialize)]
pub struct FactionData {
    pub name: String,
    pub treasury: i64,
    pub legitimacy: i64,
    #[serde(default = "default_tax_rate")]
    pub tax_rate: i64,
    #[serde(default)]
    pub army: i64,
    #[serde(default = "default_court_upkeep")]
    pub court_upkeep: i64,
}

fn default_tax_rate() -> i64 {
    20
}

fn default_court_upkeep() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
pub struct ProvinceData {
    pub name: String,
    pub owner: String,
    pub development: i64,
    pub unrest: i64,
    pub peasant: i64,
    pub burgher: i64,
    pub noble: i64,
    pub clergy: i64,
    #[serde(default)]
    pub map: Option<MapPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MapPosition {
    pub col: i32,
    pub row: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapLayout {
    positions: Vec<MapPosition>,
}

impl MapLayout {
    pub fn position(&self, id: ProvinceId) -> Option<MapPosition> {
        self.positions.get(id).copied()
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn grid(count: usize) -> Self {
        Self {
            positions: (0..count).map(default_map_position).collect(),
        }
    }
}

fn default_map_position(index: usize) -> MapPosition {
    MapPosition {
        col: (index % 4) as i32,
        row: (index / 4) as i32,
    }
}

#[derive(Debug)]
pub enum LoadError {
    Parse(ron::error::SpannedError),
    UnknownFaction { province: String, faction: String },
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Parse(error) => write!(formatter, "erro ao ler RON: {error}"),
            LoadError::UnknownFaction { province, faction } => write!(
                formatter,
                "província '{province}' referencia facção desconhecida '{faction}'"
            ),
        }
    }
}

impl std::error::Error for LoadError {}

pub fn load_world(source: &str, seed: u64) -> Result<World, LoadError> {
    load_world_with_map(source, seed).map(|(world, _)| world)
}

pub fn load_world_with_map(source: &str, seed: u64) -> Result<(World, MapLayout), LoadError> {
    let data: WorldData = ron::from_str(source).map_err(LoadError::Parse)?;

    let mut faction_ids = HashMap::new();
    let mut factions = Vec::with_capacity(data.factions.len());
    for (id, faction) in data.factions.iter().enumerate() {
        faction_ids.insert(faction.name.as_str(), id);
        factions.push(
            Faction::new(
                id,
                faction.name.clone(),
                faction.treasury,
                faction.legitimacy,
            )
            .with_tax(faction.tax_rate)
            .with_army(faction.army)
            .with_court_upkeep(faction.court_upkeep),
        );
    }

    let mut provinces = Vec::with_capacity(data.provinces.len());
    let mut positions = Vec::with_capacity(data.provinces.len());
    for (id, province) in data.provinces.iter().enumerate() {
        let owner =
            *faction_ids
                .get(province.owner.as_str())
                .ok_or_else(|| LoadError::UnknownFaction {
                    province: province.name.clone(),
                    faction: province.owner.clone(),
                })?;

        positions.push(province.map.unwrap_or_else(|| default_map_position(id)));
        provinces.push(Province::new(
            id,
            province.name.clone(),
            owner,
            province.development,
            province.unrest,
            Pops::new(
                province.peasant,
                province.burgher,
                province.noble,
                province.clergy,
            ),
        ));
    }

    Ok((
        World::new(seed, factions, provinces),
        MapLayout { positions },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORLD: &str = include_str!("../../../content/world.ron");

    #[test]
    fn bundled_world_loads() {
        let world = load_world(WORLD, 1).expect("o mundo embutido deve carregar");
        assert_eq!(world.factions.len(), 3);
        assert_eq!(world.provinces.len(), 9);
        assert!(world
            .provinces
            .iter()
            .all(|p| p.owner < world.factions.len()));
    }

    #[test]
    fn map_layout_matches_provinces() {
        let (world, layout) =
            load_world_with_map(WORLD, 1).expect("o mundo embutido deve carregar");
        assert_eq!(layout.len(), world.provinces.len());
        assert_eq!(layout.position(0), Some(MapPosition { col: 0, row: 0 }));
        assert_eq!(layout.position(8), Some(MapPosition { col: 3, row: 2 }));
    }

    #[test]
    fn missing_map_falls_back_to_grid() {
        let source = r#"(
            factions: [(name: "A", treasury: 0, legitimacy: 50)],
            provinces: [
                (name: "P", owner: "A", development: 50, unrest: 0, peasant: 1, burgher: 0, noble: 0, clergy: 0),
                (name: "Q", owner: "A", development: 50, unrest: 0, peasant: 1, burgher: 0, noble: 0, clergy: 0),
            ],
        )"#;
        let (_, layout) = load_world_with_map(source, 1).unwrap();
        assert_eq!(layout.position(0), Some(MapPosition { col: 0, row: 0 }));
        assert_eq!(layout.position(1), Some(MapPosition { col: 1, row: 0 }));
    }

    #[test]
    fn unknown_owner_is_rejected() {
        let source = r#"(
            factions: [(name: "A", treasury: 0, legitimacy: 50)],
            provinces: [(name: "P", owner: "B", development: 50, unrest: 0, peasant: 1, burgher: 0, noble: 0, clergy: 0)],
        )"#;
        assert!(matches!(
            load_world(source, 1),
            Err(LoadError::UnknownFaction { .. })
        ));
    }
}
