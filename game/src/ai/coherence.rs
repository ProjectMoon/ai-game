use anyhow::{anyhow, Result};
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use itertools::Itertools;

use crate::models::{
    coherence::{CoherenceFailure, SceneFix},
    world::scenes::{root_scene_id, Exit, Scene, SceneStub},
    Content, ContentContainer, ContentRelation,
};

use super::generator::AiGenerator;

const DIRECTIONS: [&str; 15] = [
    "north",
    "south",
    "east",
    "west",
    "northeast",
    "northwest",
    "southeast",
    "southwest",
    "up",
    "down",
    "in",
    "out",
    "to",
    "from",
    "back",
];

fn is_direction(value: &str) -> bool {
    DIRECTIONS.contains(&value.to_lowercase().as_ref())
}

pub fn reverse_direction(direction: &str) -> String {
    match direction.to_lowercase().as_ref() {
        // compass directions
        "north" => "south".to_string(),
        "south" => "north".to_string(),
        "east" => "west".to_string(),
        "west" => "east".to_string(),

        // more compass directions
        "northwest" => "southeast".to_string(),
        "northeast" => "southwest".to_string(),
        "southeast" => "northwest".to_string(),
        "southwest" => "northeast".to_string(),

        // abstract directions
        "up" => "down".to_string(),
        "down" => "up".to_string(),
        "in" => "out".to_string(),
        "out" => "in".to_string(),
        _ => "back".to_string(),
    }
}

/// If LLM generates something odd, reject it.
fn is_weird_exit_name(value: &str) -> bool {
    value.to_lowercase().contains("connected scene")
        || value.to_lowercase() == root_scene_id().as_ref()
}

fn is_duplicate_recorded(failures: &[CoherenceFailure], exit: &Exit) -> bool {
    for failure in failures {
        match failure {
            CoherenceFailure::DuplicateExits(exits) => {
                if exits.iter().find(|e| e.name == exit.name).is_some() {
                    return true;
                }
            }
            _ => (),
        }
    }

    false
}

/// This is currently for handling coherence when CREATING stuff in
/// the world. It's not doing coherence to fix things like command
/// execution.
pub(super) struct AiCoherence {
    generator: Rc<AiGenerator>,
}

impl AiCoherence {
    pub fn new(generator: Rc<AiGenerator>) -> AiCoherence {
        AiCoherence { generator }
    }

    fn check_scene_coherence<'a>(&self, scene: &'a Scene) -> Vec<CoherenceFailure<'a>> {
        let mut failures: Vec<CoherenceFailure> = vec![];

        for exit in scene.exits.as_slice() {
            // Exit names cannot be directions, "weird", or the name of
            // the current scene itself.
            if is_direction(&exit.name) || is_weird_exit_name(&exit.name) || exit.name == scene.name
            {
                failures.push(CoherenceFailure::InvalidExitName(exit));
            }

            // Also need to detect duplicate exits by direction. Stub
            // creation can have two exits that lead the same way.
            let duplicate_exits: Vec<_> =
                scene.exits.iter().filter(|e| e.name == exit.name).collect();

            if duplicate_exits.len() > 1 && !is_duplicate_recorded(&failures, exit) {
                failures.push(CoherenceFailure::DuplicateExits(duplicate_exits));
            }
        }

        failures
    }

    /// Attempt to reconnect back to the connected scene. The model is not
    /// always good at this. Here, we correct it by attempting to find the
    /// exit and making sure the direction is coherently reversed. A
    /// linkback exit is created from scratch if one cannot be found.
    pub fn make_scene_from_stub_coherent(
        &self,
        content: &mut ContentContainer,
        connected_scene: &Scene,
    ) {
        let new_scene = content.owner.as_scene_mut();
        let connected_key = connected_scene._key.as_deref().unwrap();
        let connected_id = connected_scene._id.as_deref().unwrap();

        let direction_from = connected_scene
            .exits
            .iter()
            .find(|exit| &exit.scene_key == new_scene._key.as_ref().unwrap())
            .map(|exit| exit.direction.as_ref())
            .unwrap_or("from");

        let reversed_direction = reverse_direction(direction_from);

        // 1. Delete any exit that is from the reversed direction, or
        // has the name/ID/key of the connected scene.
        let mut stubs_to_delete = vec![];
        let keep_exit = |exit: &Exit| {
            !(exit.direction == reversed_direction
                || Some(exit.scene_key.as_ref()) == connected_scene._key.as_deref()
                || exit.scene_id.as_deref() == connected_scene._id.as_deref()
                || exit.name.to_lowercase() == connected_scene.name.to_lowercase()
                || exit.name == connected_key
                || exit.name == connected_id)
        };

        new_scene.exits.retain_mut(|exit| {
            let keep = keep_exit(exit);
            if !keep {
                stubs_to_delete.push(mem::take(&mut exit.scene_key));
            }
            keep
        });

        // 2. Delete corresponding scene stubs
        content.contained.retain(|c| match &c.content {
            Content::SceneStub(stub) => match stub._key.as_ref() {
                Some(key) => !stubs_to_delete.contains(key),
                _ => true,
            },
            _ => true,
        });

        // 3. Add new linkback exit
        let exit = Exit::from_connected_scene(connected_scene, &reversed_direction);
        new_scene.exits.push(exit);
    }

    pub async fn make_scene_coherent(&self, content: &mut ContentContainer) -> Result<()> {
        let scene = content.owner.as_scene_mut();
        let failures = self.check_scene_coherence(&scene);
        let fixes = self.generator.fix_scene(&scene, failures).await?;
        let mut deletes = vec![]; // Needed for call to Vec::retain after the fact

        for fix in fixes {
            match fix {
                SceneFix::FixedExit {
                    index,
                    new: fixed_exit,
                } => {
                    let old_exit_key = scene.exits[index].scene_key.as_str();

                    content.contained.retain(|c| match &c.content {
                        Content::SceneStub(stub) => stub._key.as_deref() != Some(old_exit_key),
                        _ => true,
                    });

                    scene.exits[index] = fixed_exit.into();
                    let fixed_exit = &scene.exits[index];

                    content
                        .contained
                        .push(ContentRelation::scene_stub(SceneStub::from(fixed_exit)));
                }
                SceneFix::DeleteExit(index) => {
                    deletes.push(index);
                }
            };
        }

        // Deletes
        let mut index: usize = 0;
        scene.exits.retain(|_| {
            let keep_it = !deletes.contains(&index);
            index += 1;
            keep_it
        });

        Ok(())
    }
}
