use crate::{
    ai::AiPrompt,
    models::world::{
        raw::{PersonSeed, SceneSeed},
        scenes::{Exit, Scene, SceneStub},
    },
};

const SCENE_BNF: &'static str = r#"
root ::= Scene
Prop ::= "{"   ws   "\"name\":"   ws   string   ","   ws   "\"description\":"   ws   string   ","   ws   "\"features\":"   ws   stringlist   ","   ws   "\"possible_interactions\":"   ws   stringlist   "}"
Proplist ::= "[]" | "["   ws   Prop   (","   ws   Prop)*   "]"
Exit ::= "{"   ws   "\"name\":"   ws   string   ","   ws   "\"direction\":"   ws   string   ","   "\"region\":"   ws   string   "}"
Exitlist ::= "[]" | "["   ws   Exit   (","   ws   Exit)*   "]"
Item ::= "{"   ws   "\"name\":"   ws   string   ","   ws   "\"category\":"   ws   string   ","   ws   "\"rarity\":"   ws   string   "}"
Itemlist ::= "[]" | "["   ws   Item   (","   ws   Item)*   "]"
Person ::= "{"   ws   "\"name\":"   ws   string   ","   ws   "\"occupation\":"   ws   string   ","   ws   "\"race\":"   ws   string   "}"
Personlist ::= "[]" | "["   ws   Person   (","   ws   Person)*   "]"
Scene ::= "{"   ws "\"name\":"   ws   string   ","   ws   "\"region\":"   ws   string   ","   ws   "\"description\":"   ws   string   ","   ws   "\"people\":"   ws   Personlist   ","   ws   "\"items\":"   ws   Itemlist   ","   ws   "\"props\":"   ws   Proplist   "," "\"exits\":"    ws  Exitlist   "}"
Scenelist ::= "[]" | "["   ws   Scene   (","   ws   Scene)*   "]"
string ::= "\""   ([^"]*)   "\""
boolean ::= "true" | "false"
ws ::= [ \t\n]*
number ::= [0-9]+   "."?   [0-9]*
stringlist ::= "["   ws   "]" | "["   ws   string   (","   ws   string)*   ws   "]"
numberlist ::= "["   ws   "]" | "["   ws   string   (","   ws   number)*   ws   "]"
"#;

const EXIT_SEED_BNF: &'static str = r#"
root ::= ExitSeed
ExitSeed ::= "{"   ws   "\"name\":"   ws   string   ","   ws   "\"direction\":"   ws   string   ","   ws   "\"region\":"   ws   string   "}"
ExitSeedlist ::= "[]" | "["   ws   ExitSeed   (","   ws   ExitSeed)*   "]"
string ::= "\""   ([^"]*)   "\""
boolean ::= "true" | "false"
ws ::= [ \t\n]*
number ::= [0-9]+   "."?   [0-9]*
stringlist ::= "["   ws   "]" | "["   ws   string   (","   ws   string)*   ws   "]"
numberlist ::= "["   ws   "]" | "["   ws   string   (","   ws   number)*   ws   "]"
"#;

const PERSON_DETAILS_BNF: &'static str = r#"
root ::= PersonDetails
Item ::= "{"   ws   "\"name\":"   ws   string   ","   ws   "\"category\":"   ws   string   ","   ws   "\"rarity\":"   ws   string   "}"
Itemlist ::= "[]" | "["   ws   Item   (","   ws   Item)*   "]"
PersonDetails ::= "{"   ws   "\"description\":"   ws   string   ","   ws   "\"sex\":"   ws   string   ","   ws   "\"gender\":"   ws   string   ","   ws   "\"age\":"   ws   number   ","   ws   "\"residence\":"   ws   string   ","   ws   "\"items\":"   ws   Itemlist   ","   ws   "\"currentActivity\":"   ws   string   "}"
PersonDetailslist ::= "[]" | "["   ws   PersonDetails   (","   ws   PersonDetails)*   "]"
string ::= "\""   ([^"]*)   "\""
boolean ::= "true" | "false"
ws ::= [ \t\n]*
number ::= [0-9]+   "."?   [0-9]*
stringlist ::= "["   ws   "]" | "["   ws   string   (","   ws   string)*   ws   "]"
numberlist ::= "["   ws   "]" | "["   ws   string   (","   ws   number)*   ws   "]"
"#;

const SCENE_INSTRUCTIONS: &'static str = r#"
You are running a text-based adventure game. You must design a scene for the text-based adventure game that the user is playing. Your response must be in JSON.

A scene is a room, city, natural landmark, or another specific location in the game world.

The scene must be created with a certain level of fantasticalness:
 - `low`: Completely mundane scene, with little to no magical elements. No powerful items or artifacts. No powerful people are present, only common, mundane people.
 - `medium`: Magical elements might be present in the scene, along with some notable items or people.
 - `high`: High fantasy, a place of great power, where important people congregate, and powerful artifacts are found.

The scene has the following information:
 - `name`: The name of the scene, or location where the scene takes place.
 - `region`: The greater enclosing region of the scene.
   - The region should be specific, like the name of the city, state/province, kingdom, or geographical area.
   - The are should not be a description of where the scene is located. It must be a specifically named place.
 - `description`: A description of the scene, directed at the player.
 - `exits`: A handful of cardinal directions or new scenes to which the player can use to move to a new scene, either in the same region, or a completely different region. Exits have their own fields.
   - `direction`: This must be cardinal or relative direction of the exit. Examples: `north`, `south`, `east`, `west`, `up`, `down`, `nearby`, `in`, `out`.
   - `name`: This should be the name name of the new scene that the exit leads to. This must NOT be a direction (like `north`, `south`, `up`, `down`, `in`, `out`, etc).
   - `region`: This should be the greater enclosing region of the scene that this exit leads to.

More instructions for the `exits` field of a scene:
 - The name of an exit must be thematically appropriate.
 - All exit directions must be unique. Do not include the same direction twice.
 - Make sure the `name` field does not have the direction in it, as that is already in the `direction` field.
 - The `region` field for an exit should be same the `region` as the scene itself, if the exit leads somewhere else in the same general area.
 - IF the exit leads to a different region, the `region` should be a different value, leading the player to a new region of the world.

The scene should also be populated with the following entities:
 - People: Interesting people (not including the player themselves)
 - Items: Weapons, trinkets, currency, utensils, and other equipment.
 - Props: Various features in the scene which may or may not have a purpose.

A scene is NOT required to have these entities. A scene can have 0 people, items, or props. It should generally have at least one entity.

Do not generate more than 10 entities.

Generate this data as a structured response.
"#;

const SCENE_CREATION_PROMPT: &'static str = r#"
[INST]
{SCENE_INSTRUCTIONS}

The requested type of scene is: `{}`

The requested amount of fantasticalness is: `{}`
[/INST]
"#;

const SCENE_FROM_STUB_PROMPT: &'static str = r#"
[INST]
{SCENE_INSTRUCTIONS}

## Creation of THIS scene

Create the scene and determine its fantasticalness (`low`, `medium`, or `high`) from the provided name and region.
 - The scene connected to this one is provided for contextual information.
 - The player arrived from the Connected Scene.
 - The newly created scene MUST include the Connected Scene as an exit, in the opposite direction the player entered this new scene.
   - Example: If player went `east` to arrive here, the Connected Scene should be `west`.
   - Example: If player went `in` to arrive here, the Connected Scene should be `out`.
   - The `scene_key` field of this exit MUST be the `key` of the Connected Scene.
   - The `scene_id` field of this exit MUST be the `id` of the Connected Scene.

## Scene to Create

Name of scene to create: `{SCENE_NAME}`

Region of scene to create: `{SCENE_REGION}`

## Connected Scene Information

The Connected Scene is the scene that the player just arrived from. Use the connected scene as context for building the new scene.

Basic Connected Scene Information:
 - Connected Scene ID: `{CONNECTED_SCENE_ID}`
 - Connected Scene Key: `{CONNECTED_SCENE_KEY}`
 - Connected Scene Name: `{CONNECTED_SCENE_NAME}`
 - Connected Scene Region: `{CONNECTED_SCENE_REGION}`
 - Connected Scene Direction: `{CONNECTED_SCENE_DIRECTION}`

### Connected Scene Description

{CONNECTED_SCENE_DESCRIPTION}
[/INST]
"#;

const PERSON_CREATION_PROMPT: &'static str = r#"
[INST]
You are running a text-based adventure game. Your response must be in JSON.

Fill in the details of the person below. This person is a character in a text-based adventure game. Use the person's basic information (name, race, occupation), along with information about the scene, to fill in details about this character. The character is in this scene. The following information needs to be generated:

 - `age`: How old the person is, in years. This age should be appropriate for the person's race.
 - `sex`: The physical sex of the character. This must always be `male` or `female`.
 - `gender`: The self-identified gender of the character.
  - This is usually the same value as `sex`, but not always, as characters are, very rarely, trans.
  - Valid values for `gender` are `male`, `female`, and `nonbinary`.
 - `description`: A long, detailed physical description of the character.
  - What they look like, the color of their hair, skin, eyes.
  - What clothes they are wearing.
  - Their facial expression.
  - Details about how they move and act. How they sound when they talk.
 - `residence`: Where the person lives. This place does not need to be located in the current scene.
  - A mundane person, like a peasant, worker, or merchant, would likely have a home in the current scene.
  - People that are more fantastical in nature, or more powerful, might have a residence outside the current scene.
 - `items`: Any items or equipment that the person currently has in their possession.
  - The items and equipment should be relevant to what they are currently doing.
 - `currentActivity`: What the person is currently doing in the scene.
  - This is narrative text, that has no effect on the state of the  player or the person.

## Person Information

- Name: `{NAME}`
- Race: `{RACE}`
- Occupation: `{OCCUPATION}`

## Scene Information

{SCENE_INFO}
[/INST]
"#;

const SCENE_INFO_FOR_PERSON: &'static str = r#"
Basic scene information:
 - Scene Name: {NAME}
 - Scene REGION: {REGION}

Extended scene description:

{DESCRIPTION}
"#;

const FIX_EXIT_PROMPT: &'static str = r#"
This is an exit in a scene that was determined to be invalid. Fix the exit by giving it a better name, and making sure the direction makes sense. The scene's name and description is provided below for reference.
 - The `name` field should be the name of the place that the player would go from this scene.
 - The `name` field must not be the name of the scene below.
 - The `name` field must not be a cardinal or relative direction.
   - Example: `north`, `south`, `east`, `west` are NOT valid exit names.
   - Example: `up`, `down`, `in`, `out` are NOT valid exit names.
 - Keep the same `direction` field, if the direction is already a proper direction word.
   - The `direction` field should be `north`, `south`, `west`, `east`, `in`, `out`, etc.
   - The `direction` field is the direction the player goes to get to the new place.

Do NOT use any of the directions below for the fixed exit. Prefer using the original `direction`, if possible.

**Do not use these directions in the fixed exit:**
{OTHER_DIRECTIONS}

## Invalid Exit Information

**Invalid Exit Name:** `{INVALID_EXIT_NAME}`

**Invalid Exit Direction:** `{INVALID_EXIT_DIRECTION}`
 - Keep this `direction`, if the direction is a valid direction word.
 - If it is not valid, change it to something else.
 - If the direction is changed to something else, it must NOT be one of the directions you cannot use (see above).

## Scene Information

**Scene Name:** `{SCENE_NAME}`

Do **NOT** use this Scene Name as a `name` for the new exit.

**Scene Description**

{SCENE_DESCRIPTION}
"#;

fn scene_info_for_person(scene: &SceneSeed) -> String {
    SCENE_INFO_FOR_PERSON
        .replacen("{NAME}", &scene.name, 1)
        .replacen("{REGION}", &scene.region, 1)
        .replacen("{DESCRIPTION}", &scene.description, 1)
}

pub fn scene_creation_prompt(scene_type: &str, fantasticalness: &str) -> AiPrompt {
    AiPrompt::creative_with_grammar_and_size(
        &SCENE_CREATION_PROMPT
            .replacen("{SCENE_INSTRUCTIONS}", SCENE_INSTRUCTIONS, 1)
            .replacen("{}", scene_type, 1)
            .replacen("{}", fantasticalness, 1),
        SCENE_BNF,
        1024,
    )
}

pub fn fix_exit_prompt(scene: &Scene, invalid_exit: &Exit) -> AiPrompt {
    let other_directions: String = scene
        .exits
        .iter()
        .map(|exit| format!("- `{}`", exit.direction))
        .collect::<Vec<_>>()
        .join("\n");

    AiPrompt::new_with_grammar_and_size(
        &FIX_EXIT_PROMPT
            .replacen("{INVALID_EXIT_NAME}", &invalid_exit.name, 1)
            .replacen("{INVALID_EXIT_DIRECTION}", &invalid_exit.direction, 1)
            .replacen("{SCENE_NAME}", &scene.name, 1)
            .replacen("{SCENE_DESCRIPTION}", &scene.description, 1)
            .replacen("{OTHER_DIRECTIONS}", &other_directions, 1),
        EXIT_SEED_BNF,
        1024,
    )
}

pub fn scene_from_stub_prompt(connected_scene: &Scene, stub: &SceneStub) -> AiPrompt {
    let connected_scene_id = connected_scene._id.as_deref().unwrap_or("");
    let connected_scene_key = connected_scene._key.as_deref().unwrap_or("");
    let connected_direction = connected_scene
        .exits
        .iter()
        .find(|exit| Some(&exit.scene_key) == stub._key.as_ref())
        .map(|exit| exit.direction.as_ref())
        .unwrap_or("back");

    AiPrompt::creative_with_grammar_and_size(
        &SCENE_FROM_STUB_PROMPT
            .replacen("{SCENE_INSTRUCTIONS}", SCENE_INSTRUCTIONS, 1)
            .replacen("{CONNECTED_SCENE_NAME}", &connected_scene.name, 1)
            .replacen("{CONNECTED_SCENE_REGION}", &connected_scene.region, 1)
            .replacen("{CONNECTED_SCENE_DIRECTION}", &connected_direction, 1)
            .replacen("{CONNECTED_SCENE_KEY}", connected_scene_key, 1)
            .replacen("{CONNECTED_SCENE_ID}", connected_scene_id, 1)
            .replacen(
                "{CONNECTED_SCENE_DESCRIPTION}",
                &connected_scene.description,
                1,
            )
            .replacen("{SCENE_NAME}", &stub.name, 1)
            .replacen("{SCENE_REGION}", &stub.region, 1),
        SCENE_BNF,
        1024,
    )
}

pub fn person_creation_prompt(scene: &SceneSeed, person: &PersonSeed) -> AiPrompt {
    AiPrompt::creative_with_grammar_and_size(
        &PERSON_CREATION_PROMPT
            .replacen("{NAME}", &person.name, 1)
            .replacen("{RACE}", &person.race, 1)
            .replacen("{OCCUPATION}", &person.occupation, 1)
            .replacen("{SCENE_INFO}", &scene_info_for_person(scene), 1),
        PERSON_DETAILS_BNF,
        1024,
    )
}
