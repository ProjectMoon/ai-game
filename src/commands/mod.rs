use crate::{
    ai::logic::AiLogic,
    db::Database,
    models::{
        commands::{
            BuiltinCommand, AiCommand, ExecutionConversionResult, RawCommandExecution, CommandExecution,
        },
        world::scenes::Stage,
    },
};
use anyhow::{anyhow, Result};
use std::rc::Rc;

pub mod builtins;
pub mod converter;

pub struct CommandExecutor {
    logic: Rc<AiLogic>,
    db: Rc<Database>,
}

impl CommandExecutor {
    pub fn new(logic: Rc<AiLogic>, db: Rc<Database>) -> CommandExecutor {
        CommandExecutor { logic, db }
    }

    pub async fn execute(&self, stage: &Stage, cmd: &str) -> Result<CommandExecution> {
        CommandExecution::AiCommand(AiCommand::empty());

        if let Some(builtin) = builtins::check_builtin_command(stage, cmd) {
            return Ok(CommandExecution::Builtin(builtin));
        }

        let cached_command = self.db.load_cached_command(cmd, &stage.scene).await?;

        let raw_exec: RawCommandExecution = if let Some(cached) = cached_command {
            self.logic.execute_parsed(stage, &cached.commands).await?
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
