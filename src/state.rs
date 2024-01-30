use crate::io::display;
use crate::models::{Entity, Insertable};
use crate::{
    ai::logic::AiLogic,
    db::Database,
    models::{
        commands::CommandEvent,
        world::scenes::{SceneStub, Stage, StageOrStub},
        ContentContainer,
    },
};
use anyhow::Result;
use std::rc::Rc;

pub struct GameState {
    pub start_prompt: String,
    pub logic: Rc<AiLogic>,
    pub db: Rc<Database>,
    pub current_scene: Stage,
}

impl GameState {
    pub async fn update(&mut self, event: CommandEvent) -> Result<()> {
        println!("handling event: {:?}", event);
        match event {
            CommandEvent::ChangeScene { scene_key } => self.change_scene(&scene_key).await?,
            CommandEvent::Narration(narration) => println!("\n\n{}\n\n", narration),
            CommandEvent::LookAtEntity { ref entity_key, .. } => self.look_at(entity_key).await?,
            _ => (),
        }

        Ok(())
    }

    async fn create_from_stub(&mut self, stub: SceneStub) -> Result<Stage> {
        let mut created_scene: ContentContainer = self
            .logic
            .create_scene_from_stub(stub, &self.current_scene.scene)
            .await?;

        self.db.store_content(&mut created_scene).await?;
        let key = created_scene.owner.key().unwrap();

        let stage = self
            .db
            .load_stage(key)
            .await?
            .expect("could not find just-created scene")
            .stage();

        Ok(stage)
    }

    async fn change_scene(&mut self, scene_key: &str) -> Result<()> {
        match self.db.load_stage(scene_key).await? {
            Some(stage_or_stub) => match stage_or_stub {
                StageOrStub::Stage(stage) => self.current_scene = stage,
                StageOrStub::Stub(stub) => self.current_scene = self.create_from_stub(stub).await?,
            },
            _ => (),
        }

        Ok(())
    }

    async fn look_at(&mut self, entity_key: &str) -> Result<()> {
        let maybe_entity = self
            .db
            .load_entity(&self.current_scene.key, entity_key)
            .await?;

        if let Some(entity) = maybe_entity {
            match entity {
                Entity::Item(item) => display!(item.description),
                Entity::Person(person) => display!(person.description),
            }

            display!("\n");
        }

        Ok(())
    }
}
