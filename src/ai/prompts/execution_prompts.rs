use crate::ai::convo::AiPrompt;
use crate::models::commands::{CommandEvent, EventConversionFailures, ParsedCommand};
use crate::models::world::items::Item;
use crate::models::world::people::Person;
use crate::models::world::scenes::{Exit, Prop, Scene, Stage};
use crate::models::Insertable;
use itertools::Itertools;
use strum::VariantNames;
use tabled::settings::Style;
use tabled::{Table, Tabled};

const UNKNOWN: &'static str = "unknown";
const PERSON: &'static str = "person";
const ITEM: &'static str = "item";
const PROP: &'static str = "prop";
const NO_KEY: &'static str = "n/a";

#[derive(Tabled)]
struct EntityTableRow<'a> {
    name: &'a str,
    #[tabled(rename = "type")]
    entity_type: &'a str,
    key: &'a str,
}

impl<'a> From<&'a Person> for EntityTableRow<'a> {
    fn from(value: &'a Person) -> Self {
        EntityTableRow {
            name: &value.name,
            key: value.key().unwrap_or(UNKNOWN),
            entity_type: PERSON,
        }
    }
}

impl<'a> From<&'a Item> for EntityTableRow<'a> {
    fn from(value: &'a Item) -> Self {
        EntityTableRow {
            name: &value.name,
            key: value.key().unwrap_or(UNKNOWN),
            entity_type: ITEM,
        }
    }
}

impl<'a> From<&'a Prop> for EntityTableRow<'a> {
    fn from(value: &'a Prop) -> Self {
        EntityTableRow {
            name: &value.name,
            entity_type: PROP,
            key: NO_KEY,
        }
    }
}

#[derive(Tabled)]
pub struct ExitTableRow<'a> {
    pub name: &'a str,
    pub direction: &'a str,
    pub scene_key: &'a str,
    pub region: &'a str,
}

impl<'a> From<&'a Exit> for ExitTableRow<'a> {
    fn from(value: &'a Exit) -> Self {
        ExitTableRow {
            name: &value.name,
            direction: &value.direction,
            scene_key: &value.scene_key,
            region: &value.region,
        }
    }
}

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
 - `look_at_entity`: The player is looking at an entity--a person, prop, or item in the scene.
   - `appliesTo` is the Scene Key of the current scene.
   - `parameter` is the Entity Key of the entity being looked at.
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

Check that the events make sense and are generated correctly, given the original command.

The original command is the raw text entered by the player.

**Original Command:** `{ORIGINAL_COMMAND}`

{SCENE_INFO}

**Player Command**:
 - Action: `{ACTION}`
 - Target: `{TARGET}`
 - Location: `{LOCATION}`
 - Using: `{USING}`
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

const SCENE_PERSON_INFO: &'static str = r#"
**Person:**:
 - Name: `{PERSON_NAME}`
 - Entity Key: `{PERSON_KEY}`
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
    let mut info = "# SCENE INFORMATION\n\n".to_string();

    info.push_str("## CURRENT SCENE INFORMATION\n\n");
    info.push_str(" - Key: ");
    info.push_str(&format!("`{}`", stage.key));
    info.push_str("\n");

    info.push_str(" - Name: ");
    info.push_str(&stage.scene.name);
    info.push_str("\n");

    info.push_str(" - Location: ");
    info.push_str(&stage.scene.region);
    info.push_str("\n\n");

    let people = stage.people.iter().map_into::<EntityTableRow>();
    let items = stage.items.iter().map_into::<EntityTableRow>();
    let props = stage.scene.props.iter().map_into::<EntityTableRow>();
    let entities = people.chain(items).chain(props);

    let mut entities_table = Table::new(entities);
    entities_table.with(Style::markdown());

    info.push_str("## ENTITIES\n\n");
    info.push_str(&entities_table.to_string());
    info.push_str("\n\n");

    let mut exits = Table::new(stage.scene.exits.iter().map_into::<ExitTableRow>());
    exits.with(Style::markdown());
    info.push_str("## EXITS\n\n");
    info.push_str(&exits.to_string());

    info
}

pub fn execution_prompt(original_cmd: &str, stage: &Stage, cmd: &ParsedCommand) -> AiPrompt {
    let scene_info = stage_info(&stage);

    let prompt = COMMAND_EXECUTION_PROMPT
        .replacen("{SCENE_INFO}", &scene_info, 1)
        .replacen("{ORIGINAL_COMMAND}", &original_cmd, 1)
        .replacen("{ACTION}", &cmd.verb, 1)
        .replacen("{TARGET}", &cmd.target, 1)
        .replacen("{LOCATION}", &cmd.location, 1)
        .replacen("{USING}", &cmd.using, 1);

    AiPrompt::new_with_grammar_and_size(&prompt, COMMAND_EXECUTION_BNF, 512)
}

pub fn fix_prompt(scene: &Scene, failures: &EventConversionFailures) -> AiPrompt {
    AiPrompt::new("")
}
