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

type CoherenceResult = Result<AiCommand, EventCoherenceFailure>;

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

    pub async fn fix_incoherent_event(
        &self,
        failure: EventCoherenceFailure,
    ) -> ExecutionConversionResult {
        // TODO we need to use LLM on events that have failed non-LLM coherence.
        let coherent_event = self.cohere_event(failure).await?;
        Ok(coherent_event)
    }

    async fn cohere_event(&self, failure: EventCoherenceFailure) -> CoherenceResult {
        let event_fix = async {
            match failure {
                EventCoherenceFailure::TargetDoesNotExist(cmd) => {
                    self.fix_target_does_not_exist(cmd).await
                }
                EventCoherenceFailure::OtherError(event, _) => future::ok(event).await,
            }
        };

        event_fix
            .and_then(|e| validate_event_coherence(&self.db, e))
            .await
    }

    async fn fix_target_does_not_exist(&self, mut cmd: AiCommand) -> CoherenceResult {
        if cmd.event.is_none() {
            return Ok(cmd);
        }

        let event: &mut CommandEvent = cmd.event.as_mut().unwrap();

        if let CommandEvent::LookAtEntity(ref mut entity_key) = event {
            let res = cohere_scene_and_entity(&self.db, &self.stage, entity_key).await;

            match res {
                Ok(_) => Ok(cmd),
                Err(err) => Err(EventCoherenceFailure::OtherError(cmd, err.to_string())),
            }
        } else {
            Ok(cmd)
        }
    }
}

/// Directly mutates an entity and scene key to make sense, if
/// possible.
async fn cohere_scene_and_entity(
    db: &Database,
    stage: &Stage,
    entity_key: &mut String,
) -> AnyhowResult<()> {
    // Normalize UUIDs, assuming that they are proper UUIDs.
    normalize_keys(&mut [entity_key]);

    let scene_key = &stage.key;

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

    // Final result is if the entity actually exists or not now.
    db.entity_exists(entity_key).await.map(|_| ())
}

#[allow(dead_code)]
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
pub(super) fn normalize_keys(keys: &mut [&mut String]) {
    for key in keys {
        if let Some(normalized) = normalize_uuid(&key) {
            key.clear();
            key.push_str(&normalized);
        }
    }
}

fn normalize_uuid(uuid_str: &str) -> Option<String> {
    Uuid::parse_str(uuid_str.replace("-", "").as_ref())
        .ok()
        .map(|parsed| parsed.as_hyphenated().to_string())
}
