use crate::db::Database;
use crate::kobold_api::Client as KoboldClient;
use crate::models::commands::{
    AiCommand, ParsedCommands, ExecutionConversionResult, RawCommandExecution,
};
use crate::models::world::items::{Category, Item, Rarity};
use crate::models::world::people::{Gender, Person, Sex};
use crate::models::world::raw::{ItemSeed, PersonSeed, SceneSeed};
use crate::models::world::scenes::{Exit, Scene, SceneStub, Stage};
use crate::models::{new_uuid_string, Content, ContentContainer, ContentRelation};
use crate::commands::converter as command_converter;
use anyhow::{bail, Result};
use itertools::Itertools;
use std::rc::Rc;

use super::coherence::AiCoherence;
use super::generator::AiGenerator;

/// Highest-level AI/LLM construct, which returns fully converted game
/// objects to us. Basically, call the mid-level `client` to create
/// seed objects, then call the mid level client again to detail the
/// entities from their seeds. Then, stick a DB ID on them and put
/// them in the database(?).
pub struct AiLogic {
    generator: Rc<AiGenerator>,
    coherence: AiCoherence,
    db: Rc<Database>,
}

impl AiLogic {
    pub fn new(api_client: Rc<KoboldClient>, db: &Rc<Database>) -> AiLogic {
        let generator = Rc::new(AiGenerator::new(api_client));
        let coherence = AiCoherence::new(generator.clone());

        AiLogic {
            generator,
            coherence,
            db: db.clone(),
        }
    }

    pub async fn execute(
        &self,
        stage: &Stage,
        cmd: &str,
    ) -> Result<(ParsedCommands, RawCommandExecution)> {
        let parsed_cmd = self.generator.parse(cmd).await?;
        let execution = self.execute_parsed(stage, &parsed_cmd).await?;
        Ok((parsed_cmd, execution))
    }

    pub async fn execute_parsed(
        &self,
        stage: &Stage,
        parsed_cmd: &ParsedCommands,
    ) -> Result<RawCommandExecution> {
        //TODO handle multiple commands in list
        if parsed_cmd.commands.is_empty() {
            return Ok(RawCommandExecution::empty());
        }

        let cmd = &parsed_cmd.commands[0];
        let raw_exec: RawCommandExecution = self.generator.execute_raw(stage, cmd).await?;

        // Coherence check:
        // Set aside any events that are not in the enum
        // Set aside anything with correct event, but wrong parameters.
        // Ask LLM to fix them, if possible
        //TODO make a aiclient::fix_execution

        self.generator.reset_commands();
        Ok(raw_exec)
    }

    pub async fn create_person(&self, scene: &SceneSeed, seed: &PersonSeed) -> Result<Person> {
        self.generator.reset_person_creation();
        let details = self.generator.create_person_details(scene, seed).await?;

        let gender = match details.gender.to_lowercase().as_ref() {
            "male" | "man" | "boy" | "transmasc" => Gender::Male,
            "female" | "woman" | "girl" | "transfem" => Gender::Female,
            "nonbinary" => Gender::NonBinary,
            // fall back to using sex
            _ => match details.sex.to_lowercase().as_ref() {
                "male" | "man" | "boy" | "transmasc" => Gender::Male,
                "female" | "woman" | "girl" | "transfem" => Gender::Female,
                _ => Gender::NonBinary, // TODO 1/3 chance!
            },
        };

        let sex = match details.sex.to_lowercase().as_ref() {
            "male" | "man" | "boy" | "transfem" => Sex::Male,
            "female" | "woman" | "girl" | "transmasc" => Sex::Female,
            _ => match gender {
                Gender::Male => Sex::Male,
                Gender::Female => Sex::Male,
                _ => Sex::Male, // TODO 50/50 chance!
            },
        };

        self.generator.reset_person_creation();

        Ok(Person {
            _key: Some(new_uuid_string()),
            name: seed.name.to_string(),
            description: details.description,
            age: details.age,
            residence: details.residence,
            current_activity: details.current_activity,
            occupation: seed.occupation.to_string(),
            race: seed.race.clone(),
            sex,
            ..Default::default()
        })
    }

    pub async fn create_item(&self, scene: &SceneSeed, seed: &ItemSeed) -> Result<Item> {
        let details = self.generator.create_item_details(scene, seed).await?;

        // TODO these have to be sent to the AI
        let category = Category::Other;
        let rarity = Rarity::Common;

        Ok(Item {
            _key: Some(new_uuid_string()),
            name: seed.name.to_string(),
            description: details.description,
            attributes: details.attributes,
            secret_attributes: details.secret_attributes,
            category,
            rarity,
            ..Default::default()
        })
    }

    pub async fn create_scene_with_id(
        &self,
        scene_type: &str,
        fantasticalness: &str,
        scene_id: &str,
    ) -> Result<ContentContainer> {
        let mut content = self.create_scene(scene_type, fantasticalness).await?;
        let scene = content.owner.as_scene_mut();
        scene._key = Some(scene_id.to_string());

        Ok(content)
    }

    pub async fn create_scene_from_stub(
        &self,
        stub: SceneStub,
        connected_scene: &Scene,
    ) -> Result<ContentContainer> {
        self.generator.reset_world_creation();

        let seed = self
            .generator
            .create_scene_seed_from_stub(&stub, connected_scene)
            .await?;

        // There are two coherence steps: the first fixes up exit
        // directions and stuff, while the second is the normal scene
        // coherence (that can invoke the LLM).
        let mut content = self.fill_in_scene_from_stub(seed, stub).await?;
        self.coherence
            .make_scene_from_stub_coherent(&mut content, connected_scene);
        self.coherence.make_scene_coherent(&mut content).await?;

        self.generator.reset_world_creation();

        Ok(content)
    }

    pub async fn create_scene(
        &self,
        scene_type: &str,
        fantasticalness: &str,
    ) -> Result<ContentContainer> {
        self.generator.reset_world_creation();

        let scene_seed = self
            .generator
            .create_scene_seed(scene_type, fantasticalness)
            .await?;

        let mut content = self.fill_in_scene(scene_seed).await?;
        self.coherence.make_scene_coherent(&mut content).await?;

        self.generator.reset_world_creation();
        Ok(content)
    }

    async fn fill_in_scene_from_stub(
        &self,
        seed: SceneSeed,
        stub: SceneStub,
    ) -> Result<ContentContainer> {
        let mut content = self.fill_in_scene(seed).await?;
        let new_scene = content.owner.as_scene_mut();
        new_scene._id = stub._id;
        new_scene._key = stub._key;

        Ok(content)
    }

    async fn fill_in_scene(&self, mut scene_seed: SceneSeed) -> Result<ContentContainer> {
        let mut content_in_scene = vec![];

        // People in scene
        let mut people = vec![];
        for person_seed in scene_seed.people.as_slice() {
            let person = self.create_person(&scene_seed, person_seed).await?;
            people.push(ContentRelation::person(person));
        }

        // Items in scene
        let mut items = vec![];
        for item_seed in scene_seed.items.as_slice() {
            let item = self.create_item(&scene_seed, item_seed).await?;
            items.push(ContentRelation::item(item));
        }

        // TODO items on people, which will require 'recursive' ContentContainers.

        let exits: Vec<_> = scene_seed
            .exits
            .drain(0..)
            .map(|seed| Exit::from(seed))
            .collect();

        let mut stubs: Vec<_> = exits
            .iter()
            .map(|exit| ContentRelation::scene_stub(SceneStub::from(exit)))
            .collect();

        let mut scene = Scene {
            _key: Some(new_uuid_string()),
            name: scene_seed.name,
            region: scene_seed.region,
            description: scene_seed.description,
            props: scene_seed.props.drain(0..).map_into().collect(),
            is_stub: false,
            exits,
            ..Default::default()
        };

        content_in_scene.append(&mut people);
        content_in_scene.append(&mut items);
        content_in_scene.append(&mut stubs);

        Ok(ContentContainer {
            owner: Content::Scene(scene),
            contained: content_in_scene,
        })
    }
}
