use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

use crate::{AppState, github::CommitHash, util::is_valid_hash};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableData {
    hash: CommitHash,
    trigger_hash: CommitHash,
}

impl ExecutableData {
    pub fn hash(&self) -> &CommitHash {
        &self.hash
    }
}

impl From<&Executable> for ExecutableData {
    fn from(executable: &Executable) -> Self {
        Self {
            hash: executable.hash.clone(),
            trigger_hash: executable.trigger_hash.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Executable {
    path: PathBuf,
    hash: CommitHash,
    trigger_hash: CommitHash,
}

impl Executable {
    pub fn from_commit(commit_hash: CommitHash, trigger_hash: CommitHash) -> Self {
        let path = if commit_hash == trigger_hash {
            format!("./bin/{commit_hash}.bin")
        } else {
            format!("./bin/{trigger_hash}_{commit_hash}.bin")
        };

        Self {
            path: PathBuf::from(path),
            hash: commit_hash,
            trigger_hash,
        }
    }

    pub fn hash(&self) -> &CommitHash {
        &self.hash
    }

    pub fn trigger_hash(&self) -> &CommitHash {
        &self.trigger_hash
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

// Loop over all files in the bin directory and create a new Executable for each file with valid git commit hash name
pub async fn get_executables() -> Vec<Executable> {
    let mut executables = Vec::new();

    if let Ok(mut dir) = tokio::fs::read_dir("./bin").await {
        while let Ok(Some(file)) = dir.next_entry().await {
            let path = &file.path();

            let Some(file_name) = path.file_name() else {
                continue;
            };

            if let Some(hash_or_hashes) = file_name.to_string_lossy().strip_suffix(".bin") {
                let executable = match hash_or_hashes.split_once('_') {
                    Some((trigger_hash, hash)) => {
                        // Check for valid git commit hashes
                        if !is_valid_hash(hash) || !is_valid_hash(trigger_hash) {
                            continue;
                        }

                        Executable {
                            path: path.clone(),
                            hash: hash.into(),
                            trigger_hash: trigger_hash.into(),
                        }
                    }
                    None => {
                        // Check for valid git commit hash
                        if !is_valid_hash(hash_or_hashes) {
                            continue;
                        }

                        Executable {
                            path: path.clone(),
                            hash: hash_or_hashes.into(),
                            trigger_hash: hash_or_hashes.into(),
                        }
                    }
                };

                executables.push(executable);
            }
        }
    }

    executables
}

pub async fn remove_unused_executables(state: AppState) -> anyhow::Result<()> {
    // create bin directory if it does not exist
    tokio::fs::create_dir_all("./bin").await?;

    let executables = get_executables().await;
    let commit_hashes = state.github.get_commit_hashes();

    // check if any of the executables are not in the commit hashes
    for executable in executables {
        if !commit_hashes
            .iter()
            .any(|hash| hash == executable.hash() || hash == executable.trigger_hash())
        {
            // remove old executables (after 30 days)
            if let Ok(metadata) = tokio::fs::metadata(executable.path()).await {
                let Ok(created) = metadata.created() else {
                    continue;
                };

                let Ok(duration) = created.elapsed() else {
                    continue;
                };

                if duration.as_secs() > 30 * 24 * 60 * 60 {
                    tokio::fs::remove_file(executable.path()).await?;
                    info!("Removing old executable: {:?}", executable.path());
                } else {
                    info!(
                        "Keeping recent executable: {:?} {} days",
                        executable.path(),
                        duration.as_secs() / (24 * 60 * 60)
                    );
                }
            } else {
                info!("Keeping used executable: {:?}", executable.path());
            }
        }
    }

    state.services.update_executables().await;

    Ok(())
}
