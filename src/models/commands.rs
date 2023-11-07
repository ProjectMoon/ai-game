use serde::{Deserialize, Serialize};
use strum::{EnumString, EnumVariantNames};
use thiserror::Error;

/// Stored in the database to bypass AI 'parsing' when possible.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedCommand {
    pub raw: String,
    pub scene_key: String,
    pub commands: Commands,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commands {
    pub commands: Vec<Command>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Command {
    pub verb: String,
    pub target: String,
    pub location: String,
    pub using: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VerbsResponse {
    pub verbs: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VerbsAndTargets {
    pub entries: Vec<VerbAndTargetEntry>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VerbAndTargetEntry {
    pub verb: String,
    pub target: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawCommandExecution {
    pub valid: bool,
    pub reason: Option<String>,
    pub narration: String,
    pub events: Vec<RawCommandEvent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawCommandEvent {
    pub event_name: String,
    pub applies_to: String,
    pub parameter: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, EnumString, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CommandEvent {
    ChangeScene {
        scene_key: String,
    },
    TakeDamage {
        target: String,
        amount: u32,
    },
    Narration(String),
    Stand {
        target: String,
    },
    Sit {
        target: String,
    },
    Prone {
        target: String,
    },
    Crouch {
        target: String,
    },
    Unrecognized {
        event_name: String,
        narration: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandExecution {
    pub valid: bool,
    pub reason: Option<String>,
    pub narration: String,
    pub events: Vec<CommandEvent>,
}

impl CommandExecution {
    pub fn empty() -> CommandExecution {
        CommandExecution {
            valid: true,
            reason: None,
            narration: "".to_string(),
            events: vec![],
        }
    }

    pub fn from_raw_invalid(raw: RawCommandExecution) -> CommandExecution {
        CommandExecution {
            valid: raw.valid,
            reason: raw.reason,
            narration: "".to_string(),
            events: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExecutionConversionResult {
    Success(CommandExecution),
    PartialSuccess(CommandExecution, EventConversionFailures),
    Failure(EventConversionFailures),
}

#[derive(Clone, Debug)]
pub struct EventConversionFailures {
    pub conversion_failures: Vec<EventConversionError>,
    pub coherence_failures: Vec<EventCoherenceFailure>,
}

impl EventConversionFailures {
    pub fn from_failures(
        conversion_failures: Vec<EventConversionError>,
        coherence_failures: Vec<EventCoherenceFailure>,
    ) -> EventConversionFailures {
        EventConversionFailures {
            conversion_failures,
            coherence_failures,
        }
    }
}

#[derive(Error, Clone, Debug)]
pub enum EventConversionError {
    #[error("invalid parameter for {0:?}")]
    InvalidParameter(RawCommandEvent),

    #[error("unrecognized event - {0:?}")]
    UnrecognizedEvent(RawCommandEvent),
}

#[derive(Error, Clone, Debug)]
pub enum EventCoherenceFailure {
    #[error("target of command does not exist")]
    TargetDoesNotExist(CommandEvent),

    #[error("uncategorized coherence failure: {1}")]
    OtherError(CommandEvent, String),
}
