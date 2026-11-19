//! Upgrades old saves one format version at a time, editing the lossless
//! document tree before the typed load.
//!
//! To change the save format in a way `#[serde(default)]` cannot absorb:
//! 1. bump `SAVE_FORMAT_VERSION`;
//! 2. add a `Migration { from: <old version>, .. }` here;
//! 3. add `tests/fixtures/save_v<new version>.ron` (keep the old fixtures).

use std::fmt;

use super::doc::{self, DocError, Node};

pub type MigrationFn = fn(&mut Node) -> Result<(), MigrationError>;

/// Turns a save of version `from` into version `from + 1`. `apply` receives
/// the whole save document (`format_version`, `game_version`, `world`).
pub struct Migration {
    pub from: u32,
    pub description: &'static str,
    pub apply: MigrationFn,
}

/// Empty while the format is still at version 1.
pub const MIGRATIONS: &[Migration] = &[];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    Doc(DocError),
    Missing { from: u32 },
    Malformed(&'static str),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::Doc(error) => write!(formatter, "save ilegível: {error}"),
            MigrationError::Missing { from } => {
                write!(formatter, "não há migração para saves da versão {from}")
            }
            MigrationError::Malformed(reason) => write!(formatter, "save inesperado: {reason}"),
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<DocError> for MigrationError {
    fn from(error: DocError) -> Self {
        MigrationError::Doc(error)
    }
}

/// Applies `migrations` in order from `from` up to `to` and returns the
/// upgraded RON text with `format_version` set to `to`.
pub fn migrate(
    source: &str,
    from: u32,
    to: u32,
    migrations: &[Migration],
) -> Result<String, MigrationError> {
    let mut document = doc::parse(source)?;
    for version in from..to {
        let migration = migrations
            .iter()
            .find(|migration| migration.from == version)
            .ok_or(MigrationError::Missing { from: version })?;
        (migration.apply)(&mut document)?;
    }
    if !document.set_field("format_version", Node::atom(to.to_string())) {
        return Err(MigrationError::Malformed("o save não é uma estrutura"));
    }
    Ok(document.to_ron())
}

/// Runs `edit` on every element of `world.<list>`, e.g. every faction.
pub fn for_each_in_world_list(
    document: &mut Node,
    list: &'static str,
    mut edit: impl FnMut(&mut Node) -> Result<(), MigrationError>,
) -> Result<(), MigrationError> {
    let items = document
        .field_mut("world")
        .and_then(|world| world.field_mut(list))
        .and_then(Node::items_mut)
        .ok_or(MigrationError::Malformed(list))?;
    for item in items {
        edit(item)?;
    }
    Ok(())
}
