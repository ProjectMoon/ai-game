use crate::db::Database;
use crate::io::display;
use crate::models::commands::CommandExecution;
use crate::state::GameState;
use anyhow::Result;
use reedline::{DefaultPrompt, Reedline, Signal};
use std::rc::Rc;

pub struct GameLoop {
    state: GameState,
    db: Rc<Database>,
    editor: Reedline,
    prompt: DefaultPrompt,
}

impl GameLoop {
    pub fn new(state: GameState, db: Rc<Database>) -> GameLoop {
        GameLoop {
            state,
            db,
            editor: Reedline::create(),
            prompt: DefaultPrompt::default(),
        }
    }

    async fn handle_execution(&mut self, execution: CommandExecution) -> Result<()> {
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

    async fn execute_command(&mut self, cmd: &str) -> Result<CommandExecution> {
        let stage = &self.state.current_scene;

        let cached_command = self.db.load_cached_command(cmd, &stage.scene).await?;

        let execution = if let Some(cached) = cached_command {
            self.state
                .logic
                .execute_parsed(stage, &cached.commands)
                .await?
        } else {
            let (cmds_to_cache, execution) = self.state.logic.execute(stage, cmd).await?;

            self.db
                .cache_command(cmd, &stage.scene, &cmds_to_cache)
                .await?;

            execution
        };

        Ok(execution)
    }

    async fn handle_input(&mut self, cmd: &str) -> Result<()> {
        if !cmd.is_empty() {
            let execution = self.execute_command(cmd).await?;
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
