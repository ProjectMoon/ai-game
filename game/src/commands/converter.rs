use super::coherence::strip_prefixes;
use crate::{
    db::Database,
    models::commands::{
        AiCommand, CommandEvent, EventCoherenceFailure, EventParsingFailure,
        ExecutionConversionResult, Narrative, RawCommandEvent, RawCommandExecution,
    },
};
use anyhow::Result;
use std::convert::TryFrom;

use strum::VariantNames;

type EventParsingResult = std::result::Result<CommandEvent, EventParsingFailure>;

impl CommandEvent {
    pub fn new(raw_event: RawCommandEvent) -> EventParsingResult {
        let event_name = raw_event.event_name.as_str().to_lowercase();

        if Self::VARIANTS.contains(&event_name.as_str()) {
            deserialize_recognized_event(raw_event)
        } else {
            Err(EventParsingFailure::UnrecognizedEvent(raw_event))
        }
    }
}

impl TryFrom<RawCommandEvent> for CommandEvent {
    type Error = EventParsingFailure;

    fn try_from(raw_event: RawCommandEvent) -> Result<Self, Self::Error> {
        CommandEvent::new(raw_event)
    }
}

pub async fn convert_raw_execution(
    mut raw_exec: RawCommandExecution,
    db: &Database,
) -> ExecutionConversionResult {
    if !raw_exec.valid {
        return Ok(AiCommand::from_raw_invalid(raw_exec));
    }

    if raw_exec.event.is_none() {
        return Ok(AiCommand::empty());
    }

    let raw_event = raw_exec.event.unwrap();

    let narrative = Narrative {
        valid: raw_exec.valid,
        reason: raw_exec.reason.take(),
        narration: std::mem::take(&mut raw_exec.narration),
    };

    let converted_event = CommandEvent::new(raw_event)?;
    let cmd = AiCommand::from_raw_success(narrative, converted_event);
    validate_event_coherence(db, cmd)
        .await
        .map_err(|e| e.into())
}

fn deserialize_recognized_event(
    raw_event: RawCommandEvent,
) -> Result<CommandEvent, EventParsingFailure> {
    let event_name = raw_event.event_name.as_str().to_lowercase();
    let event_name = event_name.as_str();

    match event_name {
        // informational-related
        "narration" => Ok(CommandEvent::Narration(raw_event.parameter)),
        "look_at_entity" => Ok(CommandEvent::LookAtEntity(
            deserialize_and_normalize(raw_event),
        )),

        // scene-related
        "change_scene" => Ok(CommandEvent::ChangeScene {
            scene_key: strip_prefixes(raw_event.parameter),
        }),

        // bodily position-related
        "stand" => Ok(CommandEvent::Stand {
            target: strip_prefixes(raw_event.applies_to),
        }),
        "sit" => Ok(CommandEvent::Sit {
            target: strip_prefixes(raw_event.applies_to),
        }),
        "prone" => Ok(CommandEvent::Prone {
            target: strip_prefixes(raw_event.applies_to),
        }),
        "crouch" => Ok(CommandEvent::Crouch {
            target: strip_prefixes(raw_event.applies_to),
        }),

        // combat-related
        "take_damage" => deserialize_take_damage(raw_event),

        // unrecognized
        _ => Err(EventParsingFailure::UnrecognizedEvent(raw_event)),
    }
}

/// Deserialize and normalize an expected UUID parameter.
fn deserialize_and_normalize(raw_event: RawCommandEvent) -> String {
    let mut key = if !raw_event.applies_to.is_empty() {
        raw_event.applies_to
    } else {
        raw_event.parameter
    };

    let mut key = strip_prefixes(key);
    super::coherence::normalize_keys(&mut [&mut key]);

    key
}

fn deserialize_single(raw_event: RawCommandEvent) -> String {
    if !raw_event.applies_to.is_empty() {
        raw_event.applies_to
    } else {
        raw_event.parameter
    }
}

fn deserialize_take_damage(
    raw_event: RawCommandEvent,
) -> Result<CommandEvent, EventParsingFailure> {
    match raw_event.parameter.parse::<u32>() {
        Ok(dmg) => Ok(CommandEvent::TakeDamage {
            target: strip_prefixes(raw_event.applies_to),
            amount: dmg,
        }),
        Err(_) => Err(EventParsingFailure::InvalidParameter(raw_event)),
    }
}

pub(super) async fn validate_event_coherence<'a>(
    db: &Database,
    cmd: AiCommand,
) -> std::result::Result<AiCommand, EventCoherenceFailure> {
    if cmd.event.is_none() {
        return Ok(cmd);
    }

    match cmd.event.as_ref().unwrap() {
        CommandEvent::LookAtEntity(ref entity_key) => match db.entity_exists(&entity_key).await {
            Ok(exists) => match exists {
                true => Ok(cmd),
                false => Err(invalid_converted_event(cmd).unwrap()),
            },
            Err(err) => Err(invalid_converted_event_because_err(cmd, err)),
        },
        CommandEvent::ChangeScene { ref scene_key } => match db.stage_exists(&scene_key).await {
            Ok(exists) => match exists {
                true => Ok(cmd),
                false => Err(invalid_converted_event(cmd).unwrap()),
            },
            Err(err) => Err(invalid_converted_event_because_err(cmd, err)),
        },
        _ => Ok(cmd),
    }
}

/// The event was converted from the raw response properly, but the
/// information contained in the response is not valid.
fn invalid_converted_event(mut cmd: AiCommand) -> Option<EventCoherenceFailure> {
    match cmd.event.as_mut().unwrap() {
        CommandEvent::LookAtEntity { .. } => Some(EventCoherenceFailure::TargetDoesNotExist(cmd)),
        CommandEvent::ChangeScene { .. } => Some(EventCoherenceFailure::TargetDoesNotExist(cmd)),
        _ => None,
    }
}

/// The event was converted from the raw response properly, but
/// something went wrong with attempting to check the coherence of the
/// converted event.
fn invalid_converted_event_because_err(
    cmd: AiCommand,
    err: anyhow::Error,
) -> EventCoherenceFailure {
    EventCoherenceFailure::OtherError(cmd, format!("{}", err))
}
