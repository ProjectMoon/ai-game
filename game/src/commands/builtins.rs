use crate::models::commands::{BuiltinCommand, AiCommand};
use crate::models::world::scenes::Stage;

pub fn check_builtin_command(stage: &Stage, cmd: &str) -> Option<BuiltinCommand> {
    match cmd {
        "look" => look_command(stage),
        _ => None,
    }
}

fn look_command(_stage: &Stage) -> Option<BuiltinCommand> {
    Some(BuiltinCommand::LookAtScene)
}
