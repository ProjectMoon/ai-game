use super::tables::exit_table;
use crate::models::world::scenes::Scene;

pub(super) const CHANGE_SCENE: &'static str = r#"
The player is moving to a new scene. Pick the correct scene key from the exits table, based on the place the player wants to go.
"#;

pub(super) fn change_scene(scene: &Scene) -> String {
    // currently have exits table in beginning of prompt.
    CHANGE_SCENE.replacen("{EXIT_TABLE}", &exit_table(&scene.exits).to_string(), 1)
}
