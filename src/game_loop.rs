use crate::io::display;
use crate::models::commands::{AiCommand, BuiltinCommand, CommandExecution};
use crate::state::GameState;
use crate::{commands::CommandExecutor, db::Database};
use anyhow::Result;
use reedline::{DefaultPrompt, Reedline, Signal};
use std::rc::Rc;

pub struct GameLoop {
    executor: CommandExecutor,
    state: GameState,
    db: Rc<Database>,
    editor: Reedline,
    prompt: DefaultPrompt,
}

impl GameLoop {
    pub fn new(state: GameState, db: &Rc<Database>) -> GameLoop {
        let executor_db = db.clone();
        let loop_db = db.clone();
        let executor_logic = state.logic.clone();

        GameLoop {
            state,
            db: loop_db,
            executor: CommandExecutor::new(executor_logic, executor_db),
            editor: Reedline::create(),
            prompt: DefaultPrompt::default(),
        }
    }

    async fn handle_ai_command(&mut self, execution: AiCommand) -> Result<()> {
        if !execution.valid {
            display!(
                "You can't do that: {}",
                execution.reason.unwrap_or("for some reason...".to_string())
            );

            return Ok(());
        }

        display!("\n\n{}\n\n", execution.narration);

        for event in execution.events {
            self.state.update(event).await?;
        }

        Ok(())
    }

    // TODO this will probably eventually be moved to its own file.
    async fn handle_builtin(&mut self, builtin: BuiltinCommand) -> Result<()> {
        match builtin {
            BuiltinCommand::Look => display!("{}", self.state.current_scene),
        };

        Ok(())
    }

    async fn handle_execution(&mut self, execution: CommandExecution) -> Result<()> {
        match execution {
            CommandExecution::Builtin(builtin) => self.handle_builtin(builtin).await?,
            CommandExecution::AiCommand(exec) => self.handle_ai_command(exec).await?,
        };

        Ok(())
    }

    async fn handle_input(&mut self, cmd: &str) -> Result<()> {
        if !cmd.is_empty() {
            //let execution = self.execute_command(cmd).await?;
            let mut stage = &self.state.current_scene;
            let execution = self.executor.execute(&mut stage, cmd).await?;
            self.handle_execution(execution).await?;
        }

        Ok(())
    }

    pub async fn run_loop(&mut self) -> Result<()> {
        loop {
            display!("{}", self.state.current_scene);
            let sig = self.editor.read_line(&self.prompt);

            match sig {
                Ok(Signal::Success(buffer)) => {
                    display!("We processed: {}", buffer);
                    self.handle_input(&buffer).await?;
                }
                Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                    display!("\nAborted!");
                    break;
                }
                x => {
                    display!("Event: {:?}", x);
                }
            }
        }

        Ok(())
    }
}
