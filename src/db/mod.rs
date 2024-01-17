use crate::models::commands::{CachedParsedCommand, ParsedCommand, ParsedCommands};
use crate::models::world::scenes::{Scene, Stage, StageOrStub};
use crate::models::{Content, ContentContainer, Insertable};
use anyhow::Result;
use arangors::document::options::InsertOptions;
use arangors::graph::{EdgeDefinition, Graph};
use arangors::transaction::{TransactionCollections, TransactionSettings};
use arangors::uclient::reqwest::ReqwestClient;
use arangors::{
    AqlQuery, ClientError, Collection, Database as ArangoDatabase, Document, GenericConnection,
};
use serde::{Deserialize, Serialize};
use serde_json::value::to_value as to_json_value;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

mod queries;

/// Type alias for how we're storing IDs in DB. WOuld prefer to have
/// strong UUIDs in code and strings in DB.
pub type Key = String;

enum CollectionType {
    Document,
    Edge,
}

// Document Collections
const CMD_COLLECTION: &'static str = "command_cache";
const SCENE_COLLECTION: &'static str = "scenes";
const REGION_COLLECTION: &'static str = "regions";
const PEOPLE_COLLECTION: &'static str = "people";
const ITEMS_COLLECTION: &'static str = "items";
const PROPS_COLLECTION: &'static str = "props";
const RACES_COLLECTION: &'static str = "races";
const OCCUPATIONS_COLLECTION: &'static str = "occupations";

// Edge collections
const GAME_WORLD_EDGES: &'static str = "game_world";
const PERSON_ATTRS: &'static str = "person_attributes";

// Graphs
const GAME_WORLD_GRAPH: &'static str = "world";

const DOC_COLLECTIONS: &'static [&str] = &[
    CMD_COLLECTION,
    SCENE_COLLECTION,
    REGION_COLLECTION,
    PEOPLE_COLLECTION,
    ITEMS_COLLECTION,
    PROPS_COLLECTION,
    RACES_COLLECTION,
    OCCUPATIONS_COLLECTION,
];

const EDGE_COLLECTIONS: &'static [&str] = &[GAME_WORLD_EDGES, PERSON_ATTRS];

// Change if we decide to use a different HTTP client.
type ArangoHttp = ReqwestClient;
type ActiveDatabase = ArangoDatabase<ArangoHttp>;
type ArangoResult<T> = std::result::Result<T, ClientError>;

/// Generic edge that relates things back and forth, where the
/// relation property determines what kind of relation we actually
/// have.
#[derive(Serialize, Deserialize, Debug)]
struct Edge {
    _from: String,
    _to: String,
    relation: String,
}

/// Convert an Arango response for a single document, which may be
/// missing, into an Option type. Bubble up any other errors.
fn extract_document<T>(document: ArangoResult<Document<T>>) -> Result<Option<T>> {
    match document {
        Ok(doc) => Ok(Some(doc.document)),
        Err(db_err) => match db_err {
            ClientError::Arango(ref arr_err) => {
                if arr_err.error_num() == 1202 {
                    Ok(None)
                } else {
                    Err(db_err.into())
                }
            }
            _ => Err(db_err.into()),
        },
    }
}

fn take_first<T>(mut vec: Vec<T>) -> Option<T> {
    if vec.get(0).is_none() {
        None
    } else {
        Some(vec.swap_remove(0))
    }
}

fn insert_opts() -> InsertOptions {
    InsertOptions::builder()
        .silent(false)
        .return_new(true)
        .build()
}

fn is_scene_stub(value: &JsonValue) -> bool {
    value
        .as_object()
        .and_then(|v| v.get("scene"))
        .and_then(|scene| scene.get("isStub"))
        .and_then(|is_stub| is_stub.as_bool())
        .unwrap_or(false)
}

async fn insert_single<T>(collection: &Collection<ArangoHttp>, value: &mut T) -> Result<()>
where
    T: Insertable + Clone + Serialize,
{
    let doc = to_json_value(&value)?;
    let resp = collection.create_document(doc, insert_opts()).await?;

    let header = resp.header().unwrap();

    value.set_key(header._key.clone());
    value.set_id(header._id.clone());

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct UpsertResponse {
    pub _id: String,
    pub _key: String,
}

async fn upsert_scene(db: &ActiveDatabase, scene: &mut Scene) -> Result<()> {
    let scene_json = serde_json::to_string(&scene)?;
    let query = queries::UPSERT_SCENE.replace("<SCENE_JSON>", &scene_json);

    let aql = AqlQuery::builder()
        .query(&query)
        .bind_var("@scene_collection", SCENE_COLLECTION)
        .bind_var("scene_key", to_json_value(&scene._key).unwrap())
        .build();

    //db.aql_bind_vars::<JsonValue>(&query, vars).await?;
    let resp = take_first(db.aql_query::<UpsertResponse>(aql).await?)
        .expect("did not get upsert response");

    scene._id = Some(resp._id);
    scene._key = Some(resp._key);

    Ok(())
}

fn content_collection(content: &Content) -> &'static str {
    match content {
        Content::Scene(_) => SCENE_COLLECTION,
        Content::SceneStub(_) => SCENE_COLLECTION,
        Content::Person(_) => PEOPLE_COLLECTION,
        Content::Item(_) => ITEMS_COLLECTION,
    }
}

pub struct Database {
    conn: arangors::GenericConnection<ArangoHttp>,
    world_name: String,
}

impl Database {
    pub async fn new(conn: GenericConnection<ArangoHttp>, world_name: &str) -> Result<Database> {
        let db = Database {
            conn,
            world_name: world_name.to_string(),
        };

        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<()> {
        let dbs = self.conn.accessible_databases().await?;

        if !dbs.contains_key(&self.world_name) {
            self.conn.create_database(&self.world_name).await?;
        }

        self.create_collections(CollectionType::Document, DOC_COLLECTIONS)
            .await?;
        self.create_collections(CollectionType::Edge, EDGE_COLLECTIONS)
            .await?;

        self.create_graphs().await?;

        Ok(())
    }

    async fn create_collections(&self, coll_type: CollectionType, names: &[&str]) -> Result<()> {
        let db = self.db().await?;
        let in_db = db.accessible_collections().await?;

        for name in names {
            if in_db.iter().find(|info| info.name == *name).is_none() {
                match coll_type {
                    CollectionType::Document => db.create_collection(&name).await?,
                    CollectionType::Edge => db.create_edge_collection(&name).await?,
                };
            }
        }

        Ok(())
    }

    async fn create_graphs(&self) -> Result<()> {
        let db = self.db().await?;

        let in_db = db.graphs().await?.graphs;

        if in_db
            .iter()
            .find(|graph| graph.name == GAME_WORLD_GRAPH)
            .is_none()
        {
            let edge_def = EdgeDefinition {
                collection: GAME_WORLD_EDGES.to_string(),
                from: vec![SCENE_COLLECTION.to_string()],
                to: vec![
                    ITEMS_COLLECTION.to_string(),
                    REGION_COLLECTION.to_string(),
                    OCCUPATIONS_COLLECTION.to_string(),
                    PEOPLE_COLLECTION.to_string(),
                    PROPS_COLLECTION.to_string(),
                    RACES_COLLECTION.to_string(),
                ],
            };

            let world_graph = Graph::builder()
                .edge_definitions(vec![edge_def])
                .name(GAME_WORLD_GRAPH.to_string())
                .build();

            db.create_graph(world_graph, false).await?;
        }

        Ok(())
    }

    async fn db(&self) -> Result<ArangoDatabase<ArangoHttp>> {
        let db = self.conn.db(&self.world_name).await?;
        Ok(db)
    }

    async fn collection(&self, name: &str) -> Result<Collection<ArangoHttp>> {
        let coll = self.db().await?.collection(name).await?;
        Ok(coll)
    }

    pub async fn store_content(&self, container: &mut ContentContainer) -> Result<()> {
        let txn_settings = TransactionSettings::builder()
            .collections(
                TransactionCollections::builder()
                    .write(vec![
                        SCENE_COLLECTION.to_string(),
                        PEOPLE_COLLECTION.to_string(),
                        ITEMS_COLLECTION.to_string(),
                        GAME_WORLD_EDGES.to_string(),
                    ])
                    .build(),
            )
            .build();

        let txn = self.db().await?.begin_transaction(txn_settings).await?;

        // First, all contained content must be inserted.
        for relation in container.contained.as_mut_slice() {
            let collection = content_collection(&relation.content);
            self.store_single_content(collection, &mut relation.content)
                .await?;
        }

        // Now insert the container/owner content + relations
        let collection = content_collection(&container.owner);
        self.store_single_content(collection, &mut container.owner)
            .await?;
        self.relate_content(&container).await?;

        txn.commit_transaction().await?;

        Ok(())
    }

    async fn relate_content(&self, container: &ContentContainer) -> Result<()> {
        let game_world = self.collection(GAME_WORLD_EDGES).await?;

        let owner_id = container
            .owner
            .id()
            .expect("Did not get an ID from inserted object!");

        for relation in container.contained.as_slice() {
            let content_id = relation
                .content
                .id()
                .expect("Did not get ID from inserted contained object!");

            let outbound = Edge {
                _from: owner_id.to_string(),
                _to: content_id.to_string(),
                relation: relation.outbound.clone(),
            };

            let inbound = Edge {
                _from: content_id.to_string(),
                _to: owner_id.to_string(),
                relation: relation.inbound.clone(),
            };

            game_world
                .create_document(outbound, InsertOptions::default())
                .await?;
            game_world
                .create_document(inbound, InsertOptions::default())
                .await?;
        }

        Ok(())
    }

    pub async fn store_single_content(&self, coll_name: &str, content: &mut Content) -> Result<()> {
        let collection = self.collection(coll_name).await?;

        match content {
            //Content::Scene(ref mut scene) => insert_single(&collection, scene).await?,
            Content::Scene(ref mut scene) => upsert_scene(&self.db().await?, scene).await?,
            Content::SceneStub(ref mut stub) => insert_single(&collection, stub).await?,
            Content::Person(ref mut person) => insert_single(&collection, person).await?,
            Content::Item(ref mut item) => insert_single(&collection, item).await?,
        };

        Ok(())
    }

    pub async fn load_stage(&self, scene_key: &str) -> Result<Option<StageOrStub>> {
        let mut vars = HashMap::new();
        vars.insert("scene_key", to_json_value(&scene_key).unwrap());
        vars.insert("@scene_collection", SCENE_COLLECTION.into());

        let db = self.db().await?;

        let res = db
            .aql_bind_vars::<JsonValue>(queries::LOAD_STAGE, vars)
            .await?;

        let maybe_stage = take_first(res);

        if let Some(stage) = maybe_stage {
            let stage_or_stub = if is_scene_stub(&stage) {
                // The stub is embedded in the scene field of the result.
                StageOrStub::Stub(serde_json::from_value(
                    stage.get("scene").cloned().unwrap(),
                )?)
            } else {
                StageOrStub::Stage(serde_json::from_value(stage)?)
            };

            Ok(Some(stage_or_stub))
        } else {
            Ok(None)
        }
    }

    pub async fn stage_exists(&self, scene_key: &str) -> Result<bool> {
        let mut vars = HashMap::new();

        vars.insert("scene_key", to_json_value(&scene_key).unwrap());
        vars.insert("@scene_collection", SCENE_COLLECTION.into());

        let db = self.db().await?;
        let stage_count = db
            .aql_bind_vars::<JsonValue>(queries::LOAD_STAGE, vars)
            .await?
            .len();

        Ok(stage_count > 0)
    }

    pub async fn cache_command(
        &self,
        raw_cmd: &str,
        scene: &Scene,
        parsed_cmds: &ParsedCommands,
    ) -> Result<()> {
        let collection = self.collection(CMD_COLLECTION).await?;
        let doc = CachedParsedCommand {
            raw: raw_cmd.to_string(),
            scene_key: scene._key.as_ref().cloned().expect("scene is missing key"),
            commands: parsed_cmds.clone(),
        };

        collection.create_document(doc, insert_opts()).await?;
        Ok(())
    }

    pub async fn load_cached_command(
        &self,
        raw_cmd: &str,
        scene: &Scene,
    ) -> Result<Option<CachedParsedCommand>> {
        let scene_key = scene._key.as_deref();
        let aql = AqlQuery::builder()
            .query(queries::LOAD_CACHED_COMMAND)
            .bind_var("@cache_collection", CMD_COLLECTION)
            .bind_var("raw_cmd", to_json_value(raw_cmd)?)
            .bind_var("scene_key", to_json_value(scene_key)?)
            .build();

        let results = self.db().await?.aql_query(aql).await?;
        Ok(take_first(results))
    }
}
