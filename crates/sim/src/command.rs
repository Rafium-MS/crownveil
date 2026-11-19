use serde::{Deserialize, Serialize};

use crate::event::DecisionId;
use crate::world::{FactionId, ProvinceId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    SetTax {
        faction: FactionId,
        rate: i64,
    },
    Recruit {
        faction: FactionId,
        soldiers: i64,
    },
    Disband {
        faction: FactionId,
        soldiers: i64,
    },
    DeclareWar {
        attacker: FactionId,
        defender: FactionId,
    },
    MakePeace {
        faction: FactionId,
        other: FactionId,
    },
    Attack {
        faction: FactionId,
        province: ProvinceId,
    },
    /// `None` hands every faction to the AI.
    SetPlayer {
        faction: Option<FactionId>,
    },
    ChooseOption {
        faction: FactionId,
        decision: DecisionId,
        option: usize,
    },
}
