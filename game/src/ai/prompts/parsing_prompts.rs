use crate::{
    ai::convo::AiPrompt,
    models::commands::{ParsedCommands, VerbsResponse},
};

pub const INTRO_PROMPT: &'static str = r#"
[INST]
You are running a text-based adventure game, and the player is providing you commands as input.
 - The commands must be parsed into structured data for command execution.
 - Every message provided after these instructions that starts with `Player Input` is considered Player Input.
 - Your response should be structured JSON data that contains a list of commands to execute.
 - The parsed structured commands must also be checked for coherence.

A command consists of:
 - `verb`: a verb, which is the action that the player wants to take. This must always be a verb.
 - `target`: the target of the action. This must always be a valid target.
 - `location`: the location of the target (example: player's inventory, in the room, towards the north)
 - `using`: the item or means by which the action will be accomplished. The item must be mentioned in the
    Player Input.

Steps for parsing the Player Input:
 1. Extract the verbs from the Player Input. These are the commands that will be executed.
 2. Match the extracted verbs with their targets.
 3. Extract the location of each target, acccording to the instructions below.
 4. The `using` field should be the item or means via which the command will be accomplished.
 5. Check the structured data for coherence. Remove any commands from the list that are not do not make snse.
 6. The `count` value should be the expected number of commands, given the original Player Input.

Instructions for extracting target locations:
 - The location is where the target of the command is located.
 - If the target is in the scene with the player, the location is `current_scene`.
 - If there is no obvious location of the target, check to see if there is a compass direction related to the target. If so, that is the location of the target.
 - If the target is located on the player's person, the value is `self`.
 - If the location is not known, the value should be `unknown`.
 - If the generated location is `other`, change the location to `unknown`.

Instructions for checking structured data for coherence and making sure it makes sense:
 - Remove any commands from the final list that are not verbs.
   - Words like `with`, `and`, `by` are not verbs. Remove them from the final command list.
 - Targets of commands in the structured data must be in the Player Input.
 - The action in the `verb` field must be present in the original Player Input. If not, remove
   the comand from the list.
 - If the original Player Input does not mention a target, remove that comand from the final list.
 - The location of the target should make sense. If the player is interacting with another character
   as a target, the location of the target is not `self`, but most likely `current_scene`.
 - The value in the `using` field must be mentioned in the original Player Input. If it is not,
   change the value of `using` to `unknown`.
 - If the command is not part of the expected output, given the Player Input, remove it from the list.
 - If the `verb` field is empty, remove the command from the list.

Final instructions:
 - If the `verb` field does not actually contain a verb, remove it from the list.
 - Make sure the `using` field makes sense.
 - Make sure the `target` field makes sense.
 - Make sure all commands that are coherent and make sense remain in the list.
 - Make sure commands that are not coherent or don't make sense are removed from the list.

Player Input: `{}`
[/INST]
"#;

pub const COHERENCE_PROMPT: &'static str = r#"
[INST]
Check the generated commands for coherence according to these instructions. Your response must be in JSON.
 - If the `verb` field does not actually contain a verb, remove it from the list.
 - The action in the `verb` field must be present in the original Player Input. If not, remove
   the comand from the list.
 - Make sure the `using` field makes sense.
 - Make sure the `target` field makes sense.
 - Make sure all commands that are coherent and make sense remain in the list.
 - Make sure commands that are not coherent or don't make sense are removed from the list.
[/INST]
"#;

pub const FIND_VERBS_PROMPT: &'static str = "
[INST]

Extract the verbs from from the text below, labeled `Text`. This text is a command entered by the user, playing a text-based aventure game. Return the verbs as a JSON array.

Text: `{}`
[/INST]";

pub fn intro_prompt(cmd: &str) -> AiPrompt {
    let prompt = INTRO_PROMPT.replace("{}", cmd);
    AiPrompt::new_with_grammar(&prompt, ParsedCommands::to_grammar())
}

pub fn continuation_prompt(cmd: &str) -> AiPrompt {
    let mut prompt = String::new();

    prompt.push_str("[INST]");
    prompt.push_str(&format!("Player Input: `{}`", cmd));

    prompt.push_str("[/INST]");

    AiPrompt::new_with_grammar(&prompt, ParsedCommands::to_grammar())
}

pub fn coherence_prompt() -> AiPrompt {
    AiPrompt::new_with_grammar(COHERENCE_PROMPT, ParsedCommands::to_grammar())
}

pub fn find_verbs_prompt(cmd: &str) -> AiPrompt {
    let prompt = FIND_VERBS_PROMPT.replace("{}", cmd);
    AiPrompt::new_with_grammar(&prompt, VerbsResponse::to_grammar())
}
