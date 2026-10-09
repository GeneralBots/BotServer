pub(crate) use reqwest::Client;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use super::types::{DeploymentEnvironment, DeploymentError, DeployTarget, GeneratedApp, GeneratedFile, ProjectType};

pub struct ForgejoClient {
    base_url: String,
    token: String,
    client: Client,
}

    /// Push a small app (CI/CD workflow) onto `main` without replacing
    /// existing history: clone the remote, write the files, commit on top of
    /// the current HEAD, and push non-force. Works for both an empty remote
    /// (creates the initial commit) and a repo with existing history.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgejoRepo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub html_url: String,
}

#[derive(Debug, Serialize)]
struct CreateRepoRequest {
    name: String,
    description: String,
    private: bool,
    auto_init: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    gitignores: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    readme: Option<String>,
}

#[derive(Debug)]
pub enum ForgejoError {
    HttpError(String),
    JsonError(String),
    ApiError(String),
    GitError(String),
}

mod part1;
mod part2;
mod part3;
