use crate::models::world::people::Person;
use crate::models::{new_uuid_string, Insertable};
use crate::{db::Key, models::world::items::Item};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use super::raw::{ExitSeed, PropSeed};

pub fn root_scene_id() -> &'static String {
    static ROOT_SCENE_ID: OnceLock<String> = OnceLock::new();
    ROOT_SCENE_ID.get_or_init(|| "__root_scene__".to_string())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "_key", default)]
    pub _key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "_id", default)]
    pub _id: Option<String>,

    pub name: String,
    pub region: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub is_stub: bool,

    #[serde(default)]
    pub props: Vec<Prop>,

    #[serde(default)]
    pub exits: Vec<Exit>,
}

impl_insertable!(Scene);

impl Default for Scene {
    fn default() -> Self {
        Self {
            _key: Some(new_uuid_string()),
            _id: None,
            name: "".to_string(),
            region: "".to_string(),
            description: "".to_string(),
            is_stub: false,
            props: vec![],
            exits: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SceneStub {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "_key", default)]
    pub _key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "_id", default)]
    pub _id: Option<String>,

    pub name: String,
    pub region: String,

    #[serde(default)]
    pub is_stub: bool,
}

impl_insertable!(SceneStub);

impl Default for SceneStub {
    fn default() -> Self {
        Self {
            _key: None,
            _id: None,
            name: "".to_string(),
            region: "".to_string(),
            is_stub: true,
        }
    }
}

impl From<&Exit> for SceneStub {
    fn from(exit: &Exit) -> Self {
        Self {
            _key: Some(exit.scene_key.clone()),
            name: exit.name.clone(),
            region: exit.region.clone(),
            is_stub: true,
            ..Default::default()
        }
    }
}

// The stage is everything: a scene, the people ("actors") in it, the
// props, etc.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stage {
    pub id: String,
    pub key: String,
    pub scene: Scene,
    pub people: Vec<Person>,
    pub items: Vec<Item>,
}

impl Default for Stage {
    fn default() -> Self {
        Self {
            id: String::default(),
            key: String::default(),
            scene: Scene::default(),
            people: Vec::default(),
            items: Vec::default(),
        }
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = self.scene.name.clone();
        output.push_str("\n\n");
        output.push_str(&self.scene.description);
        output.push_str("\n\n");

        let people = self
            .people
            .iter()
            .map(|p| format!("{} ({} {}) is here.", p.name, p.race, p.occupation))
            .collect::<Vec<_>>()
            .join("\n");

        let items = self
            .items
            .iter()
            .map(|i| format!("A {} is here.", i.name))
            .collect::<Vec<_>>()
            .join("\n");

        let props = self
            .scene
            .props
            .iter()
            .map(|p| format!("A {} is here.", p.name.to_ascii_lowercase()))
            .collect::<Vec<_>>()
            .join("\n");

        let exits = self
            .scene
            .exits
            .iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<_>>()
            .join("\n");

        if !people.is_empty() {
            output.push_str(&people);
            output.push_str("\n");
        }

        if !items.is_empty() {
            output.push_str(&items);
            output.push_str("\n");
        }

        if !props.is_empty() {
            output.push_str(&props);
            output.push_str("\n");
        }

        if !exits.is_empty() {
            output.push_str("\n\nExits:\n");
            output.push_str(&exits);
        } else {
            output.push_str("\n\nExits: seemingly none...");
        }

        write!(f, "{}", output)
    }
}

// key vs id: scene_key is the document id within the scenes
// collection. scene_id is the full collection + key of that scene.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Exit {
    pub name: String,
    pub region: String,
    pub direction: String,
    pub scene_key: String,

    // will be set when returned from DB, if not manually set.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub scene_id: Option<String>,
}

impl Exit {
    pub fn from_connected_scene(scene: &Scene, direction_from: &str) -> Exit {
        Exit {
            name: scene.name.clone(),
            region: scene.region.clone(),
            direction: direction_from.to_string(),
            scene_key: scene._key.as_ref().cloned().unwrap(),
            scene_id: scene._id.clone(),
        }
    }
}

impl From<ExitSeed> for Exit {
    fn from(seed: ExitSeed) -> Self {
        Self {
            direction: seed.direction,
            name: seed.name,
            region: seed.region,
            scene_key: new_uuid_string(),
            scene_id: None, // it will be set by the database.
        }
    }
}

impl From<&ExitSeed> for Exit {
    fn from(seed: &ExitSeed) -> Self {
        Self {
            direction: seed.direction.clone(),
            name: seed.name.clone(),
            region: seed.region.clone(),
            scene_key: new_uuid_string(),
            scene_id: None, // it will be set by the database.
        }
    }
}

impl From<&mut ExitSeed> for Exit {
    fn from(seed: &mut ExitSeed) -> Self {
        Self {
            direction: seed.direction.clone(),
            name: seed.name.clone(),
            region: seed.region.clone(),
            scene_key: new_uuid_string(),
            scene_id: None, // it will be set by the database.
        }
    }
}

impl std::fmt::Display for Exit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, " - {} ({})", self.name, self.direction)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Prop {
    pub name: String,
    pub description: String,
    pub features: Vec<String>,
    pub possible_interactions: Vec<String>,
}

// This is here because some day we might pormote props to first-calss
// entities in a Stage instance.
impl From<PropSeed> for Prop {
    fn from(value: PropSeed) -> Self {
        Prop {
            name: value.name,
            description: value.description,
            features: value.features,
            possible_interactions: value.possible_interactions,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StageOrStub {
    Stage(Stage),
    Stub(SceneStub),
}

impl StageOrStub {
    /// Consumes self into Stage type. Panics if not a Stage.
    pub fn stage(self) -> Stage {
        match self {
            Self::Stage(stage) => stage,
            _ => panic!("not a stage"),
        }
    }
}
