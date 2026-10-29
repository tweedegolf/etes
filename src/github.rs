use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

use crate::{AppState, Config, events::Event};

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
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
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
pub struct Branch {
    name: String,
    commit: Commit,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Assignee {
    avatar_url: String,
    login: String,
    name: Option<String>,
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
    commits: Vec<Commit>,
    branches: Vec<Branch>,
    releases: Vec<Release>,
    pulls: Vec<Pull>,
}

impl GitHubState {
    // Fetch GitHub data using the GitHub GraphQL API
    pub async fn fetch(config: &Config) -> anyhow::Result<Self> {
        let request_body = include_str!("query.graphql")
            .replace("$owner", &config.github_owner)
            .replace("$name", &config.github_repo)
            .replace("$branches", &branch_query(&config.branches));

        let response = reqwest::Client::new()
            .post("https://api.github.com/graphql")
            .json(&json!({ "query": request_body }))
            .header("User-Agent", "etes")
            .header("Authorization", format!("Bearer {}", config.github_token))
            .send()
            .await?;

        let root: GraphRoot = response.json().await?;

        GitHubState::from_graphql(root, &config.branches)
    }

    // Commit hashes that should keep their executable: the latest default branch
    // commit, the latest commit of each configured branch, releases and pull
    // requests with check status success
    pub fn get_commit_hashes(&self) -> Vec<String> {
        self.commits
            .iter()
            .map(|commit| commit.hash.clone())
            .chain(
                self.branches
                    .iter()
                    .map(|branch| branch.commit.hash.clone()),
            )
            .chain(
                self.releases
                    .iter()
                    .map(|release| release.commit.hash.clone()),
            )
            .chain(
                self.pulls
                    .iter()
                    .filter(|pull| pull.status == WorkflowStatus::Success)
                    .map(|pull| pull.commit.hash.clone()),
            )
            .collect()
    }

    // Convert data returned from graphql to GitHubState
    fn from_graphql(root: GraphRoot, branch_names: &[String]) -> anyhow::Result<Self> {
        let mut pulls = Vec::new();
        let mut releases = Vec::new();
        let mut commits = Vec::new();
        let mut branches = Vec::new();

        for edge in root.data.repository.default_branch_ref.target.history.edges {
            let node = edge.node;

            let commit = Commit {
                date: node.committed_date,
                hash: node.oid,
                message: Some(node.message_headline),
                url: Some(node.url),
            };

            commits.push(commit);
        }

        // Branch refs are requested as aliased fields `branch_{index}`, in the
        // order of the configured branch names; a missing branch resolves to null
        let mut refs = root.data.repository.branch_refs;

        for (index, name) in branch_names.iter().enumerate() {
            let Some(Some(branch_ref)) = refs.remove(&branch_alias(index)) else {
                tracing::warn!("Configured branch not found on GitHub: {name}");
                continue;
            };

            let node = branch_ref.target;

            branches.push(Branch {
                name: branch_ref.name,
                commit: Commit {
                    date: node.committed_date,
                    hash: node.oid,
                    message: Some(node.message_headline),
                    url: Some(node.url),
                },
            });
        }

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
                    message: None,
                    url: None,
                },
            };

            releases.push(release);
        }

        for edge in root.data.repository.pull_requests.edges {
            let node = edge.node;

            if let Some(status_check_rollup) = node.status_check_rollup {
                let commit = status_check_rollup.commit;

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
                    status: status_check_rollup.state,
                    assignees,
                    commit: Commit {
                        date: commit.authored_date,
                        hash: commit.oid,
                        message: None,
                        url: None,
                    },
                };

                pulls.push(pull);
            }
        }

        Ok(GitHubState {
            commits,
            branches,
            releases,
            pulls,
        })
    }
}

fn branch_alias(index: usize) -> String {
    format!("branch_{index}")
}

// Build aliased `ref` fields for every configured branch, for inclusion in the
// repository selection of the GraphQL query
fn branch_query(branch_names: &[String]) -> String {
    branch_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            // serde_json produces a quoted and escaped string literal, which is
            // also valid GraphQL string syntax
            let qualified_name = serde_json::to_string(&format!("refs/heads/{name}"))
                .expect("string serialization cannot fail");

            format!(
                "{alias}: ref(qualifiedName: {qualified_name}) {{ name target {{ ... on Commit {{ oid url committedDate messageHeadline }} }} }}",
                alias = branch_alias(index)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Returned data structure from the graphql query
structstruck::strike! {
    #[structstruck::each[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]]
    #[structstruck::each[serde(rename_all = "camelCase")]]
    struct GraphRoot {
        data: struct Data {
            repository: struct Repository {
                default_branch_ref: struct DefaultBranchRefs {
                    target: struct DefaultBranchTarget {
                        history: struct DefaultBranchHistory {
                            edges: Vec<struct BranchEdge {
                                node: struct BranchNode {
                                    oid: CommitHash,
                                    committed_date: DateTime<Utc>,
                                    url: String,
                                    message_headline: String,
                                },
                            }>,
                        },
                    },
                },

                // Aliased `branch_{index}` ref fields, see `branch_query`
                #[serde(flatten)]
                branch_refs: HashMap<String, Option<struct BranchRef {
                    name: String,
                    target: BranchNode,
                }>>,

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
                            status_check_rollup: Option<pub struct StatusCheckRollup {
                                pub commit: struct CheckCommit {
                                    pub authored_date: DateTime<Utc>,
                                    pub oid: CommitHash,
                                },
                                pub state: WorkflowStatus,
                            }>,
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
                    message: format!("Failed to fetch GitHub data: {e}"),
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

    #[test]
    fn test_branch_query() {
        assert_eq!(branch_query(&[]), "");

        let query = branch_query(&["develop".to_string(), "release/1.0".to_string()]);

        assert!(query.contains("branch_0: ref(qualifiedName: \"refs/heads/develop\")"));
        assert!(query.contains("branch_1: ref(qualifiedName: \"refs/heads/release/1.0\")"));
    }

    #[test]
    fn test_from_graphql_branches() {
        let root: GraphRoot = serde_json::from_value(serde_json::json!({
            "data": {
                "repository": {
                    "defaultBranchRef": { "target": { "history": { "edges": [] } } },
                    "branch_0": {
                        "name": "develop",
                        "target": {
                            "oid": "a".repeat(40),
                            "committedDate": "2026-01-01T00:00:00Z",
                            "url": "https://github.com/x/y/commit/a",
                            "messageHeadline": "Develop commit"
                        }
                    },
                    "branch_1": null,
                    "releases": { "edges": [] },
                    "pullRequests": { "edges": [] }
                }
            }
        }))
        .unwrap();

        let state =
            GitHubState::from_graphql(root, &["develop".to_string(), "missing".to_string()])
                .unwrap();

        assert_eq!(state.branches.len(), 1);
        assert_eq!(state.branches[0].name, "develop");
        assert_eq!(state.branches[0].commit.hash, "a".repeat(40));
        assert_eq!(state.get_commit_hashes(), vec!["a".repeat(40)]);
    }
}
