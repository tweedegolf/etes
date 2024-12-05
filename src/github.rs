use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{events::Event, AppState, Config};

pub type CommitHash = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkflowStatus {
    #[default]
    Pending,
    Error,
    Expected,
    Failure,
    Success,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    date: DateTime<Utc>,
    hash: CommitHash,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    name: String,
    url: String,
    tag_name: String,
    created_at: DateTime<Utc>,
    commit: Commit,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Assignee {
    avatar_url: String,
    login: String,
    name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pull {
    number: i64,
    created_at: DateTime<Utc>,
    is_draft: bool,
    title: String,
    assignees: Vec<Assignee>,
    status: WorkflowStatus,
    commit: Commit,
}

pub struct GitHubStateManager {
    state: Arc<RwLock<GitHubState>>,
}

impl GitHubStateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(GitHubState::default())),
        }
    }

    pub async fn update(&self, config: &Config) -> Result<()> {
        let state = GitHubState::fetch(config).await?;
        self.set_state(state.clone());

        Ok(())
    }

    pub fn get_commit_hashes(&self) -> Vec<String> {
        self.state.read().get_commit_hashes()
    }

    pub fn get_state(&self) -> GitHubState {
        self.state.read().clone()
    }

    fn set_state(&self, state: GitHubState) {
        *self.state.write() = state;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubState {
    releases: Vec<Release>,
    pulls: Vec<Pull>,
}

impl GitHubState {
    // Fetch GitHub data using the GitHub GraphQL API
    pub async fn fetch(config: &Config) -> anyhow::Result<Self> {
        let request_body = include_str!("query.graphql")
            .replace("$owner", &config.github_owner)
            .replace("$name", &config.github_repo);

        let ressponse = reqwest::Client::new()
            .post("https://api.github.com/graphql")
            .json(&json!({ "query": request_body }))
            .header("User-Agent", "etes")
            .header("Authorization", format!("Bearer {}", config.github_token))
            .send()
            .await?;

        let root: GraphRoot = ressponse.json().await?;

        GitHubState::from_graphql(root).await
    }

    // Fetch commit hashes of releases and pull requests with check status success
    pub fn get_commit_hashes(&self) -> Vec<String> {
        self.releases
            .iter()
            .map(|release| release.commit.hash.clone())
            .chain(
                self.pulls
                    .iter()
                    .filter(|pull| pull.status == WorkflowStatus::Success)
                    .map(|pull| pull.commit.hash.clone()),
            )
            .collect()
    }

    // Convert data returned from graphql to GitHubState
    async fn from_graphql(root: GraphRoot) -> anyhow::Result<Self> {
        let mut pulls = Vec::new();
        let mut releases = Vec::new();

        for edge in root.data.repository.releases.edges {
            let node = edge.node;
            let commit = node.tag_commit;

            let release = Release {
                name: node.name,
                url: node.url,
                created_at: node.created_at,
                tag_name: node.tag_name,
                commit: Commit {
                    date: commit.authored_date,
                    hash: commit.oid,
                },
            };

            releases.push(release);
        }

        for edge in root.data.repository.pull_requests.edges {
            let node = edge.node;
            let commit = node.status_check_rollup.commit;

            let assignees = node
                .assignees
                .edges
                .into_iter()
                .map(|edge| edge.node)
                .collect();

            let pull = Pull {
                number: node.number,
                created_at: node.created_at,
                is_draft: node.is_draft,
                title: node.title,
                status: node.status_check_rollup.state,
                assignees,
                commit: Commit {
                    date: commit.authored_date,
                    hash: commit.oid,
                },
            };

            pulls.push(pull);
        }

        Ok(GitHubState { releases, pulls })
    }
}

// Returned data structure from the graphql query
structstruck::strike! {
    #[strikethrough[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]]
    #[strikethrough[serde(rename_all = "camelCase")]]
    struct GraphRoot {
        data: struct Data {
            repository: struct Repository {
                releases: struct Releases {
                    edges: Vec<struct ReleaseEdge {
                        node: struct ReleaseNode {
                            created_at: DateTime<Utc>,
                            name: String,
                            url: String,
                            tag_name: String,
                            tag_commit: struct TagCommit {
                                oid: CommitHash,
                                authored_date: DateTime<Utc>,
                            },
                        }
                    }>,
                },
                pull_requests: struct PullRequests {
                    edges: Vec<struct PullRequestsEdge {
                        node: struct PullRequestsNode {
                            created_at: DateTime<Utc>,
                            is_draft: bool,
                            number: i64,
                            title: String,
                            assignees: struct AssigneesEdges {
                                edges: Vec<struct AssigneesEdge {
                                    node: Assignee,
                                }>,
                            },
                            status_check_rollup: pub struct StatusCheckRollup {
                                pub commit: struct CheckCommit {
                                    pub authored_date: DateTime<Utc>,
                                    pub oid: CommitHash,
                                },
                                pub state: WorkflowStatus,
                            },
                        }
                    }>,
                }
            }
        }
    }
}

// Refresh GitHub data when requested
pub async fn refresh_github_data(state: AppState) -> Result<()> {
    let mut receiver = state.channel.get_receiver();

    while let Ok(event) = receiver.recv().await {
        let Event::GithubRefresh { user } = event else {
            continue;
        };

        match state.github.update(state.config).await {
            Ok(_) => {
                state.channel.send(Event::GithubState {
                    payload: state.github.get_state(),
                });
            }
            Err(e) => {
                state.channel.send(Event::Error {
                    user,
                    message: format!("Failed to fetch GitHub data: {}", e),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "do not call propduction API's in tests"]
    async fn test_get_state() {
        let config = Config::from_env().unwrap();
        let state = GitHubState::fetch(config).await.unwrap();

        assert!(!state.releases.is_empty());
        assert!(!state.pulls.is_empty());
    }
}