use crate::{
    ai::logic::AiLogic,
    db::Database,
    models::{
        commands::{
            AiCommand, BuiltinCommand, CommandExecution, ExecutionConversionResult, ParsedCommand,
            ParsedCommands, RawCommandExecution,
        },
        world::scenes::Stage,
    },
};
use anyhow::{anyhow, Result};
use std::rc::Rc;

pub mod builtins;
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

    cmd.map(|c| ParsedCommands {
        commands: vec![c],
        count: 1,
    })
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
        CommandExecution::AiCommand(AiCommand::empty());

        if let Some(builtin) = builtins::check_builtin_command(stage, cmd) {
            return Ok(CommandExecution::Builtin(builtin));
        }

        let pre_parsed = self.check_translation_and_cache(stage, cmd).await?;
        let raw_exec: RawCommandExecution = if let Some(pre_parsed_cmds) = pre_parsed {
            self.logic.execute_parsed(stage, &pre_parsed_cmds).await?
        } else {
            let (cmds_to_cache, execution) = self.logic.execute(stage, cmd).await?;

            self.db
                .cache_command(cmd, &stage.scene, &cmds_to_cache)
                .await?;

            execution
        };

        let converted = converter::convert_raw_execution(raw_exec, &self.db).await;

        //TODO handle the errored events aside from getting rid of them
        let execution: AiCommand = match converted {
            ExecutionConversionResult::Success(execution) => Ok(execution),
            ExecutionConversionResult::PartialSuccess(execution, _) => Ok(execution),
            ExecutionConversionResult::Failure(failures) => Err(anyhow!(
                "unhandled command execution failure: {:?}",
                failures
            )),
        }?;

        Ok(CommandExecution::AiCommand(execution))
    }
}
