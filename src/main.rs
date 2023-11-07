use anyhow::Result;
use config::Config;
use game_loop::GameLoop;
use models::world::scenes::{root_scene_id, Stage};
use state::GameState;
use std::{io::stdout, rc::Rc, time::Duration};

use arangors::Connection;

mod ai;
mod commands;
mod db;
mod game_loop;
mod io;

#[allow(dead_code)]
mod kobold_api;

mod models;
mod state;

use crate::{db::Database, models::world::scenes::StageOrStub};
use kobold_api::Client;

struct GameConfig {
    pub kobold_endpoint: String,
    pub arangodb_endpoint: String,
}

// Needs to be moved somewhere else.
async fn store_root_scene(db: &Database, state: &mut GameState) -> Result<Stage> {
    let mut created_scene: crate::models::ContentContainer = state
        .logic
        .create_scene_with_id(&state.start_prompt, "mundane", root_scene_id())
        .await?;

    db.store_content(&mut created_scene).await?;

    let stage = db
        .load_stage(&root_scene_id())
        .await?
        .expect("could not find root scene")
        .stage();

    Ok(stage)
}

async fn load_root_scene(db: &Database, state: &mut GameState) -> Result<()> {
    let root_scene: Stage = if let Some(stage_or_stub) = db.load_stage(&root_scene_id()).await? {
        match stage_or_stub {
            StageOrStub::Stage(stage) => stage,
            _ => panic!("Root scene was not a Stage!"),
        }
    } else {
        store_root_scene(db, state).await?
    };

    state.current_scene = root_scene;

    Ok(())
}

fn load_config() -> Result<GameConfig> {
    let settings = Config::builder()
        .add_source(config::File::with_name("config.toml"))
        .add_source(config::Environment::with_prefix("AIGAME"))
        .build()
        .unwrap();


    let kobold_endpoint = settings
        .get::<Option<String>>("connection.kobold_endpoint")?
        .unwrap_or("http://127.0.0.1:5001/api".to_string());

    let arangodb_endpoint = settings
        .get::<Option<String>>("connection.arangodb_endpoint")?
        .unwrap_or("http://localhost:8529".to_string());

    Ok(GameConfig {
        arangodb_endpoint,
        kobold_endpoint,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config()?;
    println!("Kobold API: {}", config.kobold_endpoint);
    println!("ArangoDB: {}", config.arangodb_endpoint);
    println!();

    let base_client = reqwest::ClientBuilder::new()
        .connect_timeout(Duration::from_secs(180))
        .pool_idle_timeout(Duration::from_secs(180))
        .timeout(Duration::from_secs(180))
        .build()?;

    let conn = Connection::establish_without_auth(config.arangodb_endpoint).await?;
    let client = Rc::new(Client::new_with_client(
        &config.kobold_endpoint,
        base_client,
    ));
    let db = Rc::new(Database::new(conn, "test_world").await?);
    let logic = ai::AiLogic::new(client, &db);

    let mut state = GameState {
        logic,
        db: db.clone(),
        current_scene: Stage::default(),
        start_prompt: "simple medieval village surrounded by farmlands, with a forest nearby"
            .to_string(),
    };

    load_root_scene(&db, &mut state).await?;

    let mut game_loop = GameLoop::new(state, db.clone());
    game_loop.run_loop().await?;

    Ok(())
}
