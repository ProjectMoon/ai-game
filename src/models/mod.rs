use self::world::items::Item;
use self::world::people::Person;
use self::world::scenes::{Scene, SceneStub};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Has to come before any module declarations!
macro_rules! impl_insertable {
    ($structname: ident) => {
        impl Insertable for $structname {
            fn id(&self) -> Option<&str> {
                self._id.as_deref()
            }

            fn key(&self) -> Option<&str> {
                self._key.as_deref()
            }

            fn set_id(&mut self, id: String) -> Option<String> {
                let old_id = self.take_id();
                self._id = Some(id);
                old_id
            }

            fn set_key(&mut self, key: String) -> Option<String> {
                let old_key = self.take_key();
                self._key = Some(key);
                old_key
            }

            fn take_id(&mut self) -> Option<String> {
                self._id.take()
            }

            fn take_key(&mut self) -> Option<String> {
                self._key.take()
            }
        }
    };
}

pub mod coherence;
pub mod commands;
pub mod world;

pub fn new_uuid_string() -> String {
    let uuid = Uuid::now_v7();
    let mut uuid_str = Uuid::encode_buffer();
    let uuid_str = uuid.hyphenated().encode_lower(&mut uuid_str);
    uuid_str.to_owned()
}

/// This enables arbitrary outbound relations between game content
/// entities. Usually is something like scene -> person, or person ->
/// item. Can also be like scene -> item, scene -> prop, etc.
#[derive(Debug)]
pub struct ContentContainer {
    pub owner: Content,
    pub contained: Vec<ContentRelation>,
}

#[derive(Debug)]
pub struct ContentRelation {
    pub content: Content,
    pub outbound: String,
    pub inbound: String,
}

impl ContentRelation {
    pub fn person(person: Person) -> ContentRelation {
        ContentRelation {
            content: Content::Person(person),
            outbound: "scene-has-person".to_string(),
            inbound: "person-at-scene".to_string(),
        }
    }

    pub fn item(item: Item) -> ContentRelation {
        ContentRelation {
            content: Content::Item(item),
            outbound: "item-located-at".to_string(),
            inbound: "item-possessed-by".to_string(),
        }
    }

    pub fn scene_stub(stub: SceneStub) -> ContentRelation {
        ContentRelation {
            content: Content::SceneStub(stub),
            outbound: "connects-to".to_string(),
            inbound: "connects-to".to_string(),
        }
    }
}

pub trait Insertable {
    fn id(&self) -> Option<&str>;
    fn take_id(&mut self) -> Option<String>;
    fn set_id(&mut self, id: String) -> Option<String>;

    fn key(&self) -> Option<&str>;
    fn take_key(&mut self) -> Option<String>;
    fn set_key(&mut self, key: String) -> Option<String>;
}

/// Anything that can be considered unique game content. This is
/// something recorded as a separate entity in the database, rather
/// than being embedded as part of another entity.
#[derive(Debug)]
pub enum Content {
    Person(world::people::Person),
    Scene(world::scenes::Scene),
    SceneStub(world::scenes::SceneStub),
    Item(world::items::Item),
}

impl Content {
    pub fn as_scene(&self) -> &Scene {
        match self {
            Self::Scene(ref scene) => scene,
            _ => panic!("not a scene"),
        }
    }

    pub fn as_scene_mut(&mut self) -> &mut Scene {
        match self {
            Self::Scene(ref mut scene) => scene,
            _ => panic!("not a scene"),
        }
    }
}

impl Insertable for Content {
    fn id(&self) -> Option<&str> {
        match self {
            Content::Scene(scene) => scene._id.as_deref(),
            Content::SceneStub(stub) => stub._id.as_deref(),
            Content::Person(person) => person._id.as_deref(),
            Content::Item(item) => item._id.as_deref(),
        }
    }

    fn take_id(&mut self) -> Option<String> {
        match self {
            Content::Scene(ref mut scene) => scene._id.take(),
            Content::SceneStub(ref mut stub) => stub._id.take(),
            Content::Person(ref mut person) => person._id.take(),
            Content::Item(ref mut item) => item._id.take(),
        }
    }

    fn set_id(&mut self, id: String) -> Option<String> {
        let old_id = self.take_id();

        match self {
            Content::Scene(ref mut scene) => scene._id = Some(id),
            Content::SceneStub(ref mut stub) => stub._id = Some(id),
            Content::Person(ref mut person) => person._id = Some(id),
            Content::Item(ref mut item) => item._id = Some(id),
        }

        old_id
    }

    fn key(&self) -> Option<&str> {
        match self {
            Content::Scene(scene) => scene._key.as_deref(),
            Content::SceneStub(stub) => stub._key.as_deref(),
            Content::Person(person) => person._key.as_deref(),
            Content::Item(item) => item._key.as_deref(),
        }
    }

    fn take_key(&mut self) -> Option<String> {
        match self {
            Content::Scene(ref mut scene) => scene._key.take(),
            Content::SceneStub(ref mut stub) => stub._key.take(),
            Content::Person(ref mut person) => person._key.take(),
            Content::Item(ref mut item) => item._key.take(),
        }
    }

    fn set_key(&mut self, key: String) -> Option<String> {
        let old_key = self.take_key();

        match self {
            Content::Scene(ref mut scene) => scene._key = Some(key),
            Content::SceneStub(ref mut stub) => stub._key = Some(key),
            Content::Person(ref mut person) => person._key = Some(key),
            Content::Item(ref mut item) => item._key = Some(key),
        }

        old_key
    }
}

/// An entity in a scene that can be loaded from the database. Similar
/// to but different from Content/ContentRelation.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Entity {
    Person(world::people::Person),
    Item(world::items::Item),
}

impl Insertable for Entity {
    fn id(&self) -> Option<&str> {
        match self {
            Entity::Person(person) => person.id(),
            Entity::Item(item) => item.id(),
        }
    }

    fn key(&self) -> Option<&str> {
        match self {
            Entity::Person(person) => person.key(),
            Entity::Item(item) => item.key(),
        }
    }

    fn set_id(&mut self, id: String) -> Option<String> {
        match self {
            Entity::Person(person) => person.set_id(id),
            Entity::Item(item) => item.set_id(id),
        }
    }

    fn set_key(&mut self, key: String) -> Option<String> {
        match self {
            Entity::Person(person) => person.set_key(key),
            Entity::Item(item) => item.set_key(key),
        }
    }

    fn take_id(&mut self) -> Option<String> {
        match self {
            Entity::Person(person) => person.take_id(),
            Entity::Item(item) => item.take_id(),
        }
    }

    fn take_key(&mut self) -> Option<String> {
        match self {
            Entity::Person(person) => person.take_key(),
            Entity::Item(item) => item.take_key(),
        }
    }
}
