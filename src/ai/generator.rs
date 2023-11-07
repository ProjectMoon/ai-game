use anyhow::{anyhow, Result};
use itertools::Itertools;

use super::convo::AiConversation;
use super::prompts::{execution_prompts, parsing_prompts, world_prompts};

use crate::kobold_api::Client as KoboldClient;
use crate::models::coherence::{CoherenceFailure, SceneFix};
use crate::models::commands::{Command, Commands, RawCommandExecution, VerbsResponse};
use crate::models::world::raw::{
    ExitSeed, ItemDetails, ItemSeed, PersonDetails, PersonSeed, SceneSeed,
};
use crate::models::world::scenes::{Exit, Scene, SceneStub, Stage};
use std::rc::Rc;

fn find_exit_position(exits: &[Exit], exit_to_find: &Exit) -> Result<usize> {
    let (pos, _) = exits
        .iter()
        .find_position(|&exit| exit == exit_to_find)
        .ok_or(anyhow!("cannot find exit"))?;

    Ok(pos)
}

/// Intermediate level struct that is charged with creating 'raw'
/// information via the LLM and doing basic coherence on it. Things
/// like ID creation, data management, and advanced coherence are done
/// at a higher level.
pub struct AiClient {
    parsing_convo: AiConversation,
    world_creation_convo: AiConversation,
    person_creation_convo: AiConversation,
    execution_convo: AiConversation,
}

impl AiClient {
    pub fn new(client: Rc<KoboldClient>) -> AiClient {
        AiClient {
            parsing_convo: AiConversation::new(client.clone()),
            world_creation_convo: AiConversation::new(client.clone()),
            person_creation_convo: AiConversation::new(client.clone()),
            execution_convo: AiConversation::new(client.clone()),
        }
    }

    pub fn reset_commands(&self) {
        self.parsing_convo.reset();
        self.execution_convo.reset();
    }

    pub fn reset_world_creation(&self) {
        self.world_creation_convo.reset();
    }

    pub fn reset_person_creation(&self) {
        self.person_creation_convo.reset();
    }

    pub async fn parse(&self, cmd: &str) -> Result<Commands> {
        // If convo so far is empty, add the instruction header,
        // otherwise only append to existing convo.
        let prompt = match self.parsing_convo.is_empty() {
            true => parsing_prompts::intro_prompt(&cmd),
            false => parsing_prompts::continuation_prompt(&cmd),
        };

        let mut cmds: Commands = self.parsing_convo.execute(&prompt).await?;
        let verbs = self.find_verbs(cmd).await?;
        self.check_coherence(&verbs, &mut cmds).await?;
        Ok(cmds)
    }

    async fn find_verbs(&self, cmd: &str) -> Result<Vec<String>> {
        let prompt = parsing_prompts::find_verbs_prompt(cmd);
        let verbs: VerbsResponse = self.parsing_convo.execute(&prompt).await?;

        // Basic coherence filtering to make sure the 'verb' is
        // actually in the text.
        Ok(verbs
            .verbs
            .into_iter()
            .filter(|verb| cmd.contains(verb))
            .collect())
    }

    async fn check_coherence(&self, verbs: &[String], commands: &mut Commands) -> Result<()> {
        // let coherence_prompt = parsing_prompts::coherence_prompt();
        // let mut commands: Commands = self.parsing_convo.execute(&coherence_prompt).await?;

        // Non-LLM coherence checks: remove empty commands, remove
        // non-verbs, etc.
        let filtered_commands: Vec<Command> = commands
            .clone()
            .commands
            .into_iter()
            .filter(|cmd| !cmd.verb.is_empty() && verbs.contains(&cmd.verb))
            .collect();

        commands.commands = filtered_commands;
        commands.count = commands.commands.len();

        Ok(())
    }

    pub async fn execute_raw(&self, stage: &Stage, cmd: &Command) -> Result<RawCommandExecution> {
        let prompt = execution_prompts::execution_prompt(stage, &cmd);
        let raw_exec: RawCommandExecution = self.execution_convo.execute(&prompt).await?;
        Ok(raw_exec)
    }

    pub async fn create_scene_seed(
        &self,
        scene_type: &str,
        fantasticalness: &str,
    ) -> Result<SceneSeed> {
        let prompt = world_prompts::scene_creation_prompt(scene_type, fantasticalness);
        let scene: SceneSeed = self.world_creation_convo.execute(&prompt).await?;
        Ok(scene)
    }

    pub async fn create_scene_seed_from_stub(
        &self,
        stub: &SceneStub,
        connected_scene: &Scene,
    ) -> Result<SceneSeed> {
        let prompt = world_prompts::scene_from_stub_prompt(connected_scene, stub);
        let scene: SceneSeed = self.world_creation_convo.execute(&prompt).await?;
        Ok(scene)
    }

    pub async fn create_person_details(
        &self,
        scene: &SceneSeed,
        seed: &PersonSeed,
    ) -> Result<PersonDetails> {
        let prompt = world_prompts::person_creation_prompt(scene, seed);
        let person: PersonDetails = self.person_creation_convo.execute(&prompt).await?;
        Ok(person)
    }

    pub async fn create_item_details(
        &self,
        scene: &SceneSeed,
        seed: &ItemSeed,
    ) -> Result<ItemDetails> {
        let item_details = ItemDetails {
            description: "fill me in--details prompt to AI not done yet".to_string(),
            attributes: vec![],
            secret_attributes: vec![],
        };

        Ok(item_details)
    }

    pub(super) async fn fix_scene<'a>(
        &self,
        scene: &Scene,
        failures: Vec<CoherenceFailure<'a>>,
    ) -> Result<Vec<SceneFix>> {
        let mut fixes = vec![];

        // We should always have exits here, and we should always find
        // them in the scene.
        for failure in failures {
            let fix = match failure {
                CoherenceFailure::InvalidExitName(original_exit) => {
                    println!("invalid exit name: {}", original_exit.name);
                    let prompt = world_prompts::fix_exit_prompt(scene, original_exit);
                    let fixed: ExitSeed = self.world_creation_convo.execute(&prompt).await?;
                    println!("fixed with: {:?}", fixed);
                    let position = find_exit_position(&scene.exits, original_exit)?;

                    SceneFix::FixedExit {
                        index: position,
                        new: fixed,
                    }
                }
                CoherenceFailure::DuplicateExits(bad_exits) => {
                    println!("found duplicate exits {:?}", bad_exits);
                    let position = find_exit_position(&scene.exits, bad_exits[0])?;
                    SceneFix::DeleteExit(position)
                }
            };

            fixes.push(fix);
        }

        Ok(fixes)
    }

    // async fn fix_events(&mut self, scene: &Scene, failures: &EventConversionFailures) {
    //     //
    // }
}
