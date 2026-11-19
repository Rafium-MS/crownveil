//! Versioned save files. The whole `World` is stored, including the event,
//! modifier and rite definitions, so a save keeps working after the bundled
//! content changes.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crownveil_sim::World;
use serde::{Deserialize, Serialize};

/// Bump when the saved shape changes in a way old saves cannot be read as-is,
/// and add the matching migration in `load_save`.
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
    UnknownVersion(u32),
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
            SaveError::UnknownVersion(version) => {
                write!(formatter, "versão de save desconhecida: {version}")
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
    let header: SaveHeader = ron::from_str(source).map_err(SaveError::Parse)?;
    match header.format_version {
        SAVE_FORMAT_VERSION => {
            let file: SaveFile = ron::from_str(source).map_err(SaveError::Parse)?;
            Ok(file.world)
        }
        found if found > SAVE_FORMAT_VERSION => Err(SaveError::NewerVersion {
            found,
            supported: SAVE_FORMAT_VERSION,
        }),
        found => Err(SaveError::UnknownVersion(found)),
    }
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
            Err(SaveError::UnknownVersion(0))
        ));
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
