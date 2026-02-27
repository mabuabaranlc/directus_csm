use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Create,
    Update,
    Delete,
    Revert,
    VersionSave,
    Comment,
    Upload,
    Login,
    Logout,
    Run,
    Install,
}
