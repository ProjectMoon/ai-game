use crate::ai::convo::AiPrompt;
use crate::models::commands::{Command, CommandEvent, EventConversionFailures};
use crate::models::world::scenes::{Scene, Stage};
use strum::VariantNames;

const COMMAND_EXECUTION_BNF: &'static str = r#"
root ::= CommandExecution
CommandEvent ::= "{"   ws   "\"eventName\":"   ws   string   ","   ws   "\"appliesTo\":"   ws   string   ","   ws   "\"parameter\":"   ws   string   "}"
CommandEventlist ::= "[]" | "["   ws   CommandEvent   (","   ws   CommandEvent)*   "]"
CommandExecution ::= "{"   ws   "\"valid\":"   ws   boolean   ","   ws   "\"reason\":"   ws   string   ","   ws   "\"narration\":"   ws   string   ","   ws   "\"events\":"   ws   CommandEventlist   "}"
CommandExecutionlist ::= "[]" | "["   ws   CommandExecution   (","   ws   CommandExecution)*   "]"
string ::= "\""   ([^"]*)   "\""
boolean ::= "true" | "false"
ws ::= [ \t\n]*
number ::= [0-9]+   "."?   [0-9]*
stringlist ::= "["   ws   "]" | "["   ws   string   (","   ws   string)*   ws   "]"
numberlist ::= "["   ws   "]" | "["   ws   string   (","   ws   number)*   ws   "]"
"#;

const COMMAND_EXECUTION_PROMPT: &'static str = r#"
[INST]
You are running a text-based adventure game. You have been given a command to execute. Your response must be in JSON.

You can only execute the command if it is valid. A command is invalid if:
 - It is physically impossible to perform the action.
   - Example: climbing a flat, vertical wall without equipment.
   - Example: carrying more weight than physically possible.
 - The action does not make sense.
   - Example: trying to kill something that is already dead.
   - Example: grabbing an item not present in the scene.
   - Example: targeting something or someone not present in the scene.
 - The action is not legal, moral, or ethical, according to the cultural norms or laws of the player's current location.
   - Exception: If the player is evil, they might proceed with an illegal action anyway.

A command is valid if it does not fail one or more of the tests above that would make it invalid.

Return structured JSON data consisting of:
 - `valid`: This field is `true` if the command is judged to be valid and possible. It is `false` if the command is not valid.
 - `reason`: This field contains the reason a command is considered invalid. This value should be `null` if the command is valid.
 - `narration`: The narrative text that the player will see. A descriptive result of their action.
 - `events`: A field that contains the results of executing the commands - a series of events that must happen to the player, the scene, and entities in the scene, in order for the command to be considered executed.

The `events` field must be filled with entries if the command is valid. It is a series of events that must happen. An event has `name`, `appliesTo`, and `parameter` fields:
 - `name`: The name of the event, which can be one of the ones detailed below.
 - `appliesTo`: The player, item, NPC, or other entity in the scene.
   - The event applies only to one target.
   - The `appliesTo` field should be the `key` of the target. If no key was provided, use the target's name instead.
 - `parameter`: Optional parameter with a string value that will be parsed. Parameters allowed depend on the type of event, and are detailed below.

The following events can be generated:
 - `change_scene`: The player's current scene is changed.
   - `appliesTo` must be set to `player`.
   - `parameter` must be the Scene Key of the new scene.
 - `take_damage`: The target of the event takes an amount of damage.
   - `appliesTo` must be the target taking damage (player, NPC, item, prop, or other thing in the scene)
   - `parameter` must be the amount of damage taken. This value must be a positive integer.
 - `narration`: Additional narrative information for the player that summarizes something not covered in the main narration.
   - `appliesTo` is irrelevant for this event.
   - `parameter` is irrelevant for this event.
 - `stand`: The target of the event stands up.
   - `appliesTo` must be the person standing up.
   - `parameter` is irrelevant for this event.
 - `sit`: The target of the event sits down.
   - `appliesTo` must be the person sitting down.
   - `parameter` is irrelevant for this event.
 - `prone`: The target of the event lies prone.
   - `appliesTo` must be the person lying prone.
   - `parameter` is irrelevant for this event.
 - `crouch`: The target of the event crouches.
   - `appliesTo` must be the person crouching.
   - `parameter` is irrelevant for this event.
 - `unrecognized`: For any event that is not in the list above, and is thus considered invalid. This event will be recorded for analysis.
   - `appliesTo` must be the target in the scene that the event would apply to, if it was a valid event.
   - `parameter` should be a value that theoretically makes sense, if this event was a valid event.

{SCENE_INFO}

**Player Command**:
 - Action: `{}`
 - Target: `{}`
 - Location: `{}`
 - Using: `{}`
[/INST]
"#;

pub const FIX_PROMPT: &'static str = r#"
The following command enxecution events are invalid or unrecognized.
"#;

const INVALID_NUMBER: &'static str = r#"
The number was invalid. It must be a positive integer. Make sure it is a positive integer.
"#;

const UNRECOGNIZED_EVENT: &'static str = r#"
The event {event_name} is not a recognized event. The event must be one of these events:

{event_name_list}

Change it so that the event is one of the valid events in the list, but only if the event
would make sense. If the event still cannot be recognized, set the event name to `unrecognized`.

Your reponse must be in JSON.
"#;

const SCENE_EXIT_INFO: &'static str = r#"
**Exit:**:
 - Name: `{EXIT_NAME}`
 - Direction: `{DIRECTION}`
 - Scene Key: `{SCENE_KEY}`
 - Scene Location: `{EXIT_LOCATION}`
"#;

fn unrecognized_event_solution(event_name: &str) -> String {
    let valid_events = CommandEvent::VARIANTS
        .iter()
        .map(|name| format!(" - {}", name))
        .collect::<Vec<_>>()
        .join("\n");

    UNRECOGNIZED_EVENT
        .replacen("{event_name}", event_name, 1)
        .replacen("{valid_event_names}", &valid_events, 1)
}

fn stage_info(stage: &Stage) -> String {
    let scene_description = "**Scene Description:** ".to_string() + &stage.scene.description;

    let mut info = "**Scene Information:**\n".to_string();

    info.push_str(" - Key: ");
    info.push_str(&format!("`{}`", stage.key));
    info.push_str("\n");

    info.push_str(" - Name: ");
    info.push_str(&stage.scene.name);
    info.push_str("\n");

    info.push_str(" - Location: ");
    info.push_str(&stage.scene.region);
    info.push_str("\n");

    let people: String = stage
        .people
        .iter()
        .map(|p| format!(" - {}", p.name))
        .collect::<Vec<_>>()
        .join("\n");

    info.push_str("**People:**\n");
    info.push_str(&people);
    info.push_str("\n");

    let items: String = stage
        .items
        .iter()
        .map(|i| format!(" - {} ({})", i.name, i.category))
        .collect::<Vec<_>>()
        .join("\n");

    info.push_str("**Items:**\n");
    info.push_str(&items);
    info.push_str("\n");

    let props: String = stage
        .scene
        .props
        .iter()
        .map(|p| format!(" - {}", p.name))
        .collect::<Vec<_>>()
        .join("\n");

    info.push_str("**Props:**\n");
    info.push_str(&props);
    info.push_str("\n\n");

    let exits: String = stage
        .scene
        .exits
        .iter()
        .map(|e| {
            SCENE_EXIT_INFO
                .replacen("{EXIT_NAME}", &e.name, 1)
                .replacen("{DIRECTION}", &e.direction, 1)
                .replacen("{SCENE_KEY}", &e.scene_key, 1)
                .replacen("{EXIT_LOCATION}", &e.region, 1)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    info.push_str(&exits);

    info.push_str(&scene_description);

    info
}

pub fn execution_prompt(stage: &Stage, cmd: &Command) -> AiPrompt {
    let scene_info = stage_info(&stage);

    let prompt = COMMAND_EXECUTION_PROMPT
        .replacen("{SCENE_INFO}", &scene_info, 1)
        .replacen("{}", &cmd.verb, 1)
        .replacen("{}", &cmd.target, 1)
        .replacen("{}", &cmd.location, 1)
        .replacen("{}", &cmd.using, 1);

    AiPrompt::new_with_grammar_and_size(&prompt, COMMAND_EXECUTION_BNF, 512)
}

pub fn fix_prompt(scene: &Scene, failures: &EventConversionFailures) -> AiPrompt {
    AiPrompt::new("")
}
