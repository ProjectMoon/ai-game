use super::converter::validate_event_coherence;
use super::partition;
use crate::{
    ai::logic::AiLogic,
    db::Database,
    models::{
        commands::{AiCommand, CommandEvent, EventCoherenceFailure, ExecutionConversionResult},
        world::scenes::{root_scene_id, Stage},
    },
};
use anyhow::{anyhow, Result as AnyhowResult};
use futures::stream::{self, StreamExt};
use futures::{future, TryFutureExt};
use std::rc::Rc;
use uuid::Uuid;

type CoherenceResult = Result<CommandEvent, EventCoherenceFailure>;

pub struct CommandCoherence<'a> {
    logic: Rc<AiLogic>,
    db: Rc<Database>,
    stage: &'a Stage,
}

impl CommandCoherence<'_> {
    pub fn new<'a>(
        logic: &Rc<AiLogic>,
        db: &Rc<Database>,
        stage: &'a Stage,
    ) -> CommandCoherence<'a> {
        CommandCoherence {
            logic: logic.clone(),
            db: db.clone(),
            stage,
        }
    }

    pub async fn fix_incoherent_events(
        &self,
        failures: Vec<EventCoherenceFailure>,
    ) -> ExecutionConversionResult {
        let (successes, failures) = partition!(
            stream::iter(failures.into_iter()).then(|failure| self.cohere_event(failure))
        );

        // TODO we need to use LLM on events that have failed non-LLM coherence.

        if successes.len() > 0 && failures.len() == 0 {
            ExecutionConversionResult::Success(AiCommand::from_events(successes))
        } else if successes.len() > 0 && failures.len() > 0 {
            ExecutionConversionResult::PartialSuccess(
                AiCommand::from_events(successes),
                failures.into(),
            )
        } else {
            ExecutionConversionResult::Failure(failures.into())
        }
    }

    async fn cohere_event(&self, failure: EventCoherenceFailure) -> CoherenceResult {
        let event_fix = async {
            match failure {
                EventCoherenceFailure::TargetDoesNotExist(event) => {
                    self.fix_target_does_not_exist(event).await
                }
                EventCoherenceFailure::OtherError(event, _) => future::ok(event).await,
            }
        };

        event_fix
            .and_then(|e| validate_event_coherence(&self.db, e))
            .await
    }

    async fn fix_target_does_not_exist(&self, mut event: CommandEvent) -> CoherenceResult {
        if let CommandEvent::LookAtEntity {
            ref mut entity_key,
            ref mut scene_key,
        } = event
        {
            let res = cohere_scene_and_entity(&self.db, &self.stage, entity_key, scene_key).await;

            match res {
                Ok(_) => Ok(event),
                Err(err) => Err(EventCoherenceFailure::OtherError(event, err.to_string())),
            }
        } else {
            Ok(event)
        }
    }
}

/// Directly mutates an entity and scene key to make sense, if
/// possible.
async fn cohere_scene_and_entity(
    db: &Database,
    stage: &Stage,
    entity_key: &mut String,
    scene_key: &mut String,
) -> AnyhowResult<()> {
    // Normalize UUIDs, assuming that they are proper UUIDs.
    normalize_keys(scene_key, entity_key);

    // Sometimes scene key is actually the entity key, and the entity
    // key is blank.
    if !scene_key.is_empty() && scene_key != &stage.key {
        // Check if scene key is an entity
        if db.entity_exists(&stage.key, &scene_key).await? {
            entity_key.clear();
            entity_key.push_str(&scene_key);
            scene_key.clear();
            scene_key.push_str(&stage.key);
        }
    }

    // If scene key isn't valid, override it from known-good
    // information.
    if !is_valid_scene_key(scene_key) {
        scene_key.clear();
        scene_key.push_str(&stage.key);
    }

    // If entity key is not a valid UUID at this point, then we have
    // entered a weird situation.
    if Uuid::try_parse(&entity_key).is_err() {
        return Err(anyhow!("Entity key is not a UUID"));
    }

    // If the scene key and entity key are the same at this point,
    // then we have entered a weird situation.
    if scene_key == entity_key {
        return Err(anyhow!("Scene key and entity key are the same"));
    }

    // It is often likely that the scene key and entity key are reversed.
    if db.entity_exists(&entity_key, &scene_key).await? {
        std::mem::swap(entity_key, scene_key);
    }

    Ok(())
}

fn is_valid_scene_key(scene_key: &str) -> bool {
    scene_key == root_scene_id() || Uuid::try_parse(&scene_key).is_ok()
}

/// Used as basic sanitization for raw command parameters and
/// applies_to.
#[inline]
pub fn strip_prefixes(value: String) -> String {
    value
        .strip_prefix("scenes/")
        .and_then(|s| s.strip_prefix("people/"))
        .and_then(|s| s.strip_prefix("items/"))
        .map(String::from)
        .unwrap_or(value)
}

/// Make sure entity keys are valid UUIDs, and fix them if possible.
fn normalize_keys(scene_key: &mut String, entity_key: &mut String) {
    if let Some(normalized) = normalize_uuid(&scene_key) {
        scene_key.clear();
        scene_key.push_str(&normalized);
    }

    if let Some(normalized) = normalize_uuid(&entity_key) {
        entity_key.clear();
        entity_key.push_str(&normalized);
    }
}

fn normalize_uuid(uuid_str: &str) -> Option<String> {
    Uuid::parse_str(uuid_str.replace("-", "").as_ref())
        .ok()
        .map(|parsed| parsed.as_hyphenated().to_string())
}
