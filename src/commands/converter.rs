use crate::{
    db::Database,
    models::commands::{
        CommandEvent, CommandExecution, EventCoherenceFailure, EventConversionError,
        EventConversionFailures, ExecutionConversionResult, RawCommandEvent, RawCommandExecution,
    },
};
use anyhow::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use itertools::{Either, Itertools};
use std::convert::TryFrom;

use strum::VariantNames;

type EventConversionResult = std::result::Result<CommandEvent, EventConversionError>;

impl CommandEvent {
    pub fn new(raw_event: RawCommandEvent) -> EventConversionResult {
        let event_name = raw_event.event_name.as_str().to_lowercase();

        if Self::VARIANTS.contains(&event_name.as_str()) {
            deserialize_recognized_event(raw_event)
        } else {
            Err(EventConversionError::UnrecognizedEvent(raw_event))
        }
    }
}

impl TryFrom<RawCommandEvent> for CommandEvent {
    type Error = EventConversionError;

    fn try_from(raw_event: RawCommandEvent) -> Result<Self, Self::Error> {
        CommandEvent::new(raw_event)
    }
}

/// Internal struct to hold the narrative parts of the
/// RawCommandExecution to minimize clones.
struct Narrative {
    valid: bool,
    reason: Option<String>,
    narration: String,
}

fn from_raw_success(raw: Narrative, events: Vec<CommandEvent>) -> CommandExecution {
    CommandExecution {
        events,
        valid: raw.valid,
        reason: match &raw.reason {
            Some(reason) if !raw.valid && reason.is_empty() => {
                Some("invalid for unknown reason".to_string())
            }
            Some(_) if !raw.valid => raw.reason.clone(),
            _ => None,
        },
        narration: raw.narration.clone(),
    }
}

pub async fn convert_raw_execution(
    mut raw_exec: RawCommandExecution,
    db: &Database,
) -> ExecutionConversionResult {
    if !raw_exec.valid {
        return ExecutionConversionResult::Success(CommandExecution::from_raw_invalid(raw_exec));
    }

    let narrative = Narrative {
        valid: raw_exec.valid,
        reason: raw_exec.reason.take(),
        narration: std::mem::take(&mut raw_exec.narration),
    };

    let conversions: Vec<_> = raw_exec
        .events
        .into_iter()
        .map(|raw_event| CommandEvent::new(raw_event))
        .collect();

    let (converted, conversion_failures): (Vec<_>, Vec<_>) =
        conversions.into_iter().partition_map(|res| match res {
            Ok(converted_event) => Either::Left(converted_event),
            Err(err) => Either::Right(err),
        });

    // Coherence validation of converted events.
    let (successes, incoherent_events): (Vec<_>, Vec<_>) = stream::iter(converted.into_iter())
        .then(|event| validate_event_coherence(db, event))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .partition_map(|res| match res {
            Ok(event) => Either::Left(event),
            Err(err) => Either::Right(err),
        });

    let failure_len = conversion_failures.len() + incoherent_events.len();

    if successes.len() > 0 && failure_len == 0 {
        ExecutionConversionResult::Success(from_raw_success(narrative, successes))
    } else if successes.len() > 0 && failure_len > 0 {
        let converted_execution = from_raw_success(narrative, successes);
        let failures =
            EventConversionFailures::from_failures(conversion_failures, incoherent_events);
        ExecutionConversionResult::PartialSuccess(converted_execution, failures)
    } else {
        ExecutionConversionResult::Failure(EventConversionFailures::from_failures(
            conversion_failures,
            incoherent_events,
        ))
    }
}

fn deserialize_recognized_event(
    raw_event: RawCommandEvent,
) -> Result<CommandEvent, EventConversionError> {
    let event_name = raw_event.event_name.as_str().to_lowercase();
    let event_name = event_name.as_str();

    match event_name {
        // scene-related
        "change_scene" => Ok(CommandEvent::ChangeScene {
            scene_key: raw_event
                .parameter
                .strip_prefix("scenes/") // Mini coherence check
                .map(String::from)
                .unwrap_or(raw_event.parameter)
                .clone(),
        }),

        // bodily position-related
        "stand" => Ok(CommandEvent::Stand {
            target: raw_event.applies_to,
        }),
        "sit" => Ok(CommandEvent::Sit {
            target: raw_event.applies_to,
        }),
        "prone" => Ok(CommandEvent::Prone {
            target: raw_event.applies_to,
        }),
        "crouch" => Ok(CommandEvent::Crouch {
            target: raw_event.applies_to,
        }),

        // combat-related
        "take_damage" => deserialize_take_damage(raw_event),

        // miscellaneous
        "narration" => Ok(CommandEvent::Narration(raw_event.parameter)),

        // unrecognized
        _ => Err(EventConversionError::UnrecognizedEvent(raw_event)),
    }
}

fn deserialize_take_damage(
    raw_event: RawCommandEvent,
) -> Result<CommandEvent, EventConversionError> {
    match raw_event.parameter.parse::<u32>() {
        Ok(dmg) => Ok(CommandEvent::TakeDamage {
            target: raw_event.applies_to,
            amount: dmg,
        }),
        Err(_) => Err(EventConversionError::InvalidParameter(raw_event)),
    }
}

async fn validate_event_coherence<'a>(
    db: &Database,
    event: CommandEvent,
) -> std::result::Result<CommandEvent, EventCoherenceFailure> {
    match event {
        CommandEvent::ChangeScene { ref scene_key } => match db.stage_exists(&scene_key).await {
            Ok(exists) => match exists {
                true => Ok(event),
                false => Err(invalid_converted_event(event).unwrap()),
            },
            Err(err) => Err(invalid_converted_event_because_err(event, err)),
        },
        _ => Ok(event),
    }
}

/// The event was converted from the raw response properly, but the
/// information contained in the response is not valid.
fn invalid_converted_event(event: CommandEvent) -> Option<EventCoherenceFailure> {
    match event {
        CommandEvent::ChangeScene { .. } => Some(EventCoherenceFailure::TargetDoesNotExist(event)),
        _ => None,
    }
}

/// The event was converted from the raw response properly, but
/// something went wrong with attempting to check the coherence of the
/// converted event.
fn invalid_converted_event_because_err(
    event: CommandEvent,
    err: anyhow::Error,
) -> EventCoherenceFailure {
    EventCoherenceFailure::OtherError(event, format!("{}", err))
}
