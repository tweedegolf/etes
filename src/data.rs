use anyhow::Result;
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;

use crate::{
    AppState, GITHUB_BASE_URL,
    error::AppError,
    executable::ExecutableData,
    github::GitHubState,
    monitor::MemoryState,
    service::ServiceData,
    user::{GitHubUser, User},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    is_admin: bool,
    user: User,
    title: String,
    base_url: String,
    github: GitHubState,
    memory: MemoryState,
    executables: Vec<ExecutableData>,
    services: Vec<ServiceData>,
    words: Vec<String>,
}

// Initial data fetch
pub async fn data_handler(
    State(state): State<AppState>,
    Path(caller): Path<String>,
    github_user: Option<GitHubUser>,
) -> Result<Json<InitialState>, AppError> {
    let user = User::from_request(caller, github_user)?;

    let github = state.github.get_state();
    let services = state.services.get_state();
    let executables = state.services.get_executables();

    Ok(Json(InitialState {
        is_admin: user.is_admin(state.config),
        user: user.hash_anonymous(),
        base_url: format!(
            "{GITHUB_BASE_URL}/{}/{}",
            state.config.github_owner, state.config.github_repo
        ),
        title: state.config.title.clone(),
        memory: state.monitor.get_state(),
        executables,
        github,
        services,
        words: state.config.words.clone(),
    }))
}
