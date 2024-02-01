use super::world::{scenes::Exit, raw::ExitSeed};

/// Categorize various coherence issues to be re-submitted to the
/// LLM.
pub enum CoherenceFailure<'a> {
    /// Exit name is invalid, a direction or something else weird.
    InvalidExitName(&'a Exit),
    /// Two or more exits share the same name or direction.
    DuplicateExits(Vec<&'a Exit>),
}

pub enum SceneFix {
    FixedExit {
        index: usize,
        new: ExitSeed
    },
    DeleteExit(usize),
}
