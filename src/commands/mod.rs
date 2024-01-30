use crate::{
    ai::logic::AiLogic,
    db::Database,
    models::{
        commands::{
            AiCommand, CommandEvent, CommandExecution, EventCoherenceFailure,
            ExecutionConversionResult, ParsedCommand, ParsedCommands, RawCommandExecution,
        },
        world::scenes::Stage,
    },
};
use anyhow::Result;
use std::rc::Rc;

/// Splits up a stream of results into successes and failures.
macro_rules! partition {
    ($stream: expr) => {
        $stream
            .fold(
                (vec![], vec![]),
                |(mut successes, mut failures), res| async {
                    match res {
                        Ok(event) => successes.push(event),
                        Err(err) => failures.push(err),
                    };

                    (successes, failures)
                },
            )
            .await
    };
}

pub(self) use partition;

pub mod builtins;
pub mod coherence;
pub mod converter;

fn directional_command(direction: &str) -> ParsedCommand {
    ParsedCommand {
        verb: "go".to_string(),
        target: direction.to_string(),
        location: "direction".to_string(),
        using: "".to_string(),
    }
}

/// Translate certain common commands to commands better understood by
/// the LLM.
fn translate(cmd: &str) -> Option<ParsedCommands> {
    let cmd = match cmd {
        "n" => Some(directional_command("north")),
        "s" => Some(directional_command("south")),
        "e" => Some(directional_command("east")),
        "w" => Some(directional_command("west")),
        "nw" => Some(directional_command("northwest")),
        "ne" => Some(directional_command("northeast")),
        "sw" => Some(directional_command("southwest")),
        "se" => Some(directional_command("southeast")),
        "up" => Some(directional_command("up")),
        "down" => Some(directional_command("down")),
        "in" => Some(directional_command("in")),
        "out" => Some(directional_command("out")),
        "back" => Some(directional_command("back")),
        "from" => Some(directional_command("from")),
        _ => None,
    };

    cmd.map(|c| ParsedCommands::single(&format!("go {}", c.target), c))
}

pub struct CommandExecutor {
    logic: Rc<AiLogic>,
    db: Rc<Database>,
}

impl CommandExecutor {
    pub fn new(logic: Rc<AiLogic>, db: Rc<Database>) -> CommandExecutor {
        CommandExecutor { logic, db }
    }

    async fn check_translation_and_cache(
        &self,
        stage: &Stage,
        cmd: &str,
    ) -> Result<Option<ParsedCommands>> {
        let maybe_commands = match translate(cmd) {
            Some(translated_cmds) => Some(translated_cmds),
            None => self
                .db
                .load_cached_command(cmd, &stage.scene)
                .await?
                .map(|c| c.commands),
        };

        Ok(maybe_commands)
    }

    pub async fn execute(&self, stage: &Stage, cmd: &str) -> Result<CommandExecution> {
        if let Some(builtin) = builtins::check_builtin_command(stage, cmd) {
            return Ok(CommandExecution::Builtin(builtin));
        }

        let pre_parsed = self.check_translation_and_cache(stage, cmd).await?;
        let raw_exec: RawCommandExecution = if let Some(pre_parsed_cmds) = pre_parsed {
            self.logic.execute_parsed(stage, &pre_parsed_cmds).await?
        } else {
            let (cmds_to_cache, execution) = self.logic.execute(stage, cmd).await?;

            if execution.valid && cmds_to_cache.commands.len() > 0 {
                self.db
                    .cache_command(cmd, &stage.scene, &cmds_to_cache)
                    .await?;
            }

            execution
        };

        let converted = converter::convert_raw_execution(raw_exec, &self.db).await;

        let execution: Result<AiCommand> = match converted {
            ExecutionConversionResult::Success(execution) => Ok(execution),
            ExecutionConversionResult::PartialSuccess(mut execution, failures) => {
                // TODO also deal with conversion failures
                // TODO deal with failures to fix incoherent events.
                // right now we just drop them.
                let mut fixed_events = self
                    .fix_incoherence(stage, failures.coherence_failures)
                    .await
                    .ok()
                    .unwrap_or(vec![]);

                execution.events.append(&mut fixed_events);
                Ok(execution)
            }
            ExecutionConversionResult::Failure(failures) => {
                // TODO also deal with conversion failures
                // For a complete failure, we want to make sure all
                // events become coherent.
                Ok(AiCommand::from_events(
                    self.fix_incoherence(stage, failures.coherence_failures)
                        .await?,
                ))
            }
        };

        let execution = execution?;

        Ok(CommandExecution::AiCommand(execution))
    }

    async fn fix_incoherence(
        &self,
        stage: &Stage,
        failures: Vec<EventCoherenceFailure>,
    ) -> Result<Vec<CommandEvent>> {
        println!("Attempting to fix {} incoherent events", failures.len());
        let fixer = coherence::CommandCoherence::new(&self.logic, &self.db, stage);

        // TODO should do something w/ partial failures.
        let events = match fixer.fix_incoherent_events(failures).await {
            ExecutionConversionResult::Success(AiCommand { events, .. }) => Ok(events),
            ExecutionConversionResult::PartialSuccess(AiCommand { events, .. }, _) => Ok(events),
            ExecutionConversionResult::Failure(errs) => Err(errs),
        }?;

        Ok(events)
    }
}
