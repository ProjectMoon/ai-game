use crate::models::Insertable;
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
    pub logic: AiLogic,
    pub db: Rc<Database>,
    pub current_scene: Stage,
}

impl GameState {
    pub async fn update(&mut self, event: CommandEvent) -> Result<()> {
        println!("handling event: {:?}", event);
        match event {
            CommandEvent::ChangeScene { scene_key } => self.change_scene(&scene_key).await?,
            CommandEvent::Narration(narration) => println!("\n\n{}\n\n", narration),
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
}
