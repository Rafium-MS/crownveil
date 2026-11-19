//! Versioned save files. The whole `World` is stored, including the event,
//! modifier and rite definitions, so a save keeps working after the bundled
//! content changes.

pub mod doc;
pub mod migrations;

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crownveil_sim::World;
use serde::{Deserialize, Serialize};

use migrations::{Migration, MigrationError, MIGRATIONS};

/// Bump when the saved shape changes in a way old saves cannot be read as-is,
/// and add the matching migration in `migrations::MIGRATIONS`.
pub const SAVE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct SaveFile {
    format_version: u32,
    game_version: String,
    world: World,
}

/// Read first, so the version can be checked before touching the rest.
#[derive(Debug, Deserialize)]
struct SaveHeader {
    format_version: u32,
}

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Serialize(ron::Error),
    Parse(ron::error::SpannedError),
    NewerVersion { found: u32, supported: u32 },
    Migration { from: u32, error: MigrationError },
}

impl fmt::Display for SaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Io(error) => write!(formatter, "erro de arquivo: {error}"),
            SaveError::Serialize(error) => write!(formatter, "erro ao gravar o save: {error}"),
            SaveError::Parse(error) => write!(formatter, "save inválido ou corrompido: {error}"),
            SaveError::NewerVersion { found, supported } => write!(
                formatter,
                "save da versão {found}, mas este jogo só lê até a versão {supported}"
            ),
            SaveError::Migration { from, error } => {
                write!(
                    formatter,
                    "não foi possível atualizar o save da versão {from}: {error}"
                )
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl From<io::Error> for SaveError {
    fn from(error: io::Error) -> Self {
        SaveError::Io(error)
    }
}

pub fn save_world(world: &World) -> Result<String, SaveError> {
    let file = SaveFile {
        format_version: SAVE_FORMAT_VERSION,
        game_version: env!("CARGO_PKG_VERSION").to_string(),
        world: world.clone(),
    };
    ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::new()).map_err(SaveError::Serialize)
}

pub fn load_save(source: &str) -> Result<World, SaveError> {
    load_with(source, MIGRATIONS)
}

fn load_with(source: &str, migrations: &[Migration]) -> Result<World, SaveError> {
    let header: SaveHeader = ron::from_str(source).map_err(SaveError::Parse)?;
    let found = header.format_version;
    if found > SAVE_FORMAT_VERSION {
        return Err(SaveError::NewerVersion {
            found,
            supported: SAVE_FORMAT_VERSION,
        });
    }

    let file: SaveFile = if found == SAVE_FORMAT_VERSION {
        ron::from_str(source).map_err(SaveError::Parse)?
    } else {
        let upgraded = migrations::migrate(source, found, SAVE_FORMAT_VERSION, migrations)
            .map_err(|error| SaveError::Migration { from: found, error })?;
        ron::from_str(&upgraded).map_err(SaveError::Parse)?
    };
    Ok(file.world)
}

/// Writes to a temporary file and renames it over `path`, so a crash while
/// saving never leaves a half-written save in place of the previous one.
pub fn write_save_file(path: &Path, world: &World) -> Result<(), SaveError> {
    let contents = save_world(world)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    fs::write(&temporary, contents)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

pub fn read_save_file(path: &Path) -> Result<World, SaveError> {
    load_save(&fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load_events, load_rites, load_world};
    use crownveil_sim::Command;

    const WORLD: &str = include_str!("../../../content/world.ron");
    const EVENTS: &str = include_str!("../../../content/events.ron");
    const RITES: &str = include_str!("../../../content/rites.ron");

    fn bundled(seed: u64) -> World {
        load_world(WORLD, seed)
            .and_then(|world| load_events(EVENTS, world))
            .and_then(|world| load_rites(RITES, world))
            .unwrap()
            .with_player(Some(0))
    }

    fn played(seed: u64, months: usize) -> World {
        let mut world = bundled(seed);
        for month in 0..months {
            if month % 13 == 0 {
                world.queue_command(Command::Recruit {
                    faction: 0,
                    soldiers: 10,
                });
            }
            world.tick_month();
        }
        world
    }

    #[test]
    fn round_trip_preserves_the_whole_world() {
        let world = played(7, 600);
        assert!(!world.event_log.is_empty());
        let restored = load_save(&save_world(&world).unwrap()).unwrap();
        assert_eq!(restored, world);
    }

    #[test]
    fn loaded_game_continues_exactly_like_the_original() {
        let mut original = played(3, 240);
        let mut restored = load_save(&save_world(&original).unwrap()).unwrap();
        for _ in 0..360 {
            original.tick_month();
            restored.tick_month();
        }
        assert_eq!(restored, original);
    }

    #[test]
    fn pending_state_survives_saving() {
        let mut world = played(1, 12);
        world.queue_command(Command::SetTax {
            faction: 0,
            rate: 40,
        });
        let restored = load_save(&save_world(&world).unwrap()).unwrap();
        assert_eq!(restored.pending_commands, world.pending_commands);
        assert_eq!(restored.player, Some(0));
    }

    #[test]
    fn newer_version_is_rejected() {
        let save =
            save_world(&bundled(1))
                .unwrap()
                .replacen("format_version: 1", "format_version: 99", 1);
        assert!(matches!(
            load_save(&save),
            Err(SaveError::NewerVersion { found: 99, .. })
        ));
    }

    #[test]
    fn unknown_old_version_is_rejected() {
        let save =
            save_world(&bundled(1))
                .unwrap()
                .replacen("format_version: 1", "format_version: 0", 1);
        assert!(matches!(
            load_save(&save),
            Err(SaveError::Migration {
                from: 0,
                error: MigrationError::Missing { from: 0 }
            })
        ));
    }

    /// Pretends the previous version called `Faction.mana` "magic", then
    /// checks the migration chain brings the save back exactly.
    #[test]
    fn migrations_upgrade_old_saves() {
        fn rename_magic(document: &mut doc::Node) -> Result<(), MigrationError> {
            migrations::for_each_in_world_list(document, "factions", |faction| {
                faction
                    .rename_field("magic", "mana")
                    .then_some(())
                    .ok_or(MigrationError::Malformed("facção sem 'magic'"))
            })
        }
        let chain = [Migration {
            from: SAVE_FORMAT_VERSION - 1,
            description: "magic -> mana",
            apply: rename_magic,
        }];

        let world = played(5, 120);
        let mut old = doc::parse(&save_world(&world).unwrap()).unwrap();
        old.set_field(
            "format_version",
            doc::Node::atom((SAVE_FORMAT_VERSION - 1).to_string()),
        );
        migrations::for_each_in_world_list(&mut old, "factions", |faction| {
            faction.rename_field("mana", "magic");
            Ok(())
        })
        .unwrap();
        let old = old.to_ron();

        assert!(matches!(load_save(&old), Err(SaveError::Migration { .. })));
        assert_eq!(load_with(&old, &chain).unwrap(), world);
    }

    #[test]
    fn missing_link_in_the_chain_is_reported() {
        let mut old = doc::parse(&save_world(&bundled(1)).unwrap()).unwrap();
        old.set_field("format_version", doc::Node::atom("0"));
        let chain = [Migration {
            from: 0,
            description: "não chega na versão atual sozinha",
            apply: |_| Ok(()),
        }];
        let result = load_with(&old.to_ron(), &chain);
        if SAVE_FORMAT_VERSION > 1 {
            assert!(matches!(
                result,
                Err(SaveError::Migration {
                    error: MigrationError::Missing { from: 1 },
                    ..
                })
            ));
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn document_tree_round_trips_a_real_save() {
        let world = played(9, 300);
        let text = save_world(&world).unwrap();
        let rewritten = doc::parse(&text).unwrap().to_ron();
        assert_ne!(rewritten, text, "a saída é compacta");
        assert_eq!(load_save(&rewritten).unwrap(), world);
    }

    /// Every format version ever released must keep loading. When bumping
    /// `SAVE_FORMAT_VERSION`, add `tests/fixtures/save_v<N>.ron`.
    #[test]
    fn every_released_save_version_still_loads() {
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        for version in 1..=SAVE_FORMAT_VERSION {
            let path = fixtures.join(format!("save_v{version}.ron"));
            let mut world =
                read_save_file(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(!world.factions.is_empty());
            for _ in 0..24 {
                world.tick_month();
            }
        }
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(matches!(
            load_save("não é um save"),
            Err(SaveError::Parse(_))
        ));
        let truncated = save_world(&bundled(1)).unwrap();
        let truncated = &truncated[..truncated.len() / 2];
        assert!(matches!(load_save(truncated), Err(SaveError::Parse(_))));
    }

    #[test]
    fn files_round_trip_and_replace_previous_save() {
        let dir = std::env::temp_dir().join(format!("crownveil-save-test-{}", std::process::id()));
        let path = dir.join("slot").join("teste.ron");

        write_save_file(&path, &bundled(1)).unwrap();
        let later = played(1, 24);
        write_save_file(&path, &later).unwrap();

        assert_eq!(read_save_file(&path).unwrap(), later);
        assert!(!dir.join("slot").join("teste.ron.tmp").exists());
        assert!(matches!(
            read_save_file(&dir.join("inexistente.ron")),
            Err(SaveError::Io(_))
        ));
        fs::remove_dir_all(dir).unwrap();
    }
}
