use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::{process::Command, sync::oneshot, task::JoinHandle};
use tracing::{error, info};

use crate::{
    config::Config,
    events::ServiceState,
    executable::{Executable, ExecutableData},
    user::User,
    util::get_free_port,
};

/// Service data structure for the client
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceData {
    pub name: String,
    pub port: u16,
    pub executable: ExecutableData,
    pub state: ServiceState,
    pub creator: User,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&Service> for ServiceData {
    fn from(service: &Service) -> Self {
        Self {
            name: service.name.to_string(),
            port: service.port,
            executable: (&service.executable).into(),
            created_at: service.created_at,
            creator: service.creator.hash_anonymous(),
            error: service.error.clone(),
            state: service.state.clone(),
        }
    }
}

/// Internal service data structure
#[derive(Debug)]
pub struct Service {
    name: String,
    executable: Executable,
    port: u16,
    creator: User,
    created_at: DateTime<Utc>,
    state: ServiceState,
    error: Option<String>,
    kill: Option<oneshot::Sender<()>>,
    child: Option<JoinHandle<()>>,
}

impl Service {
    pub async fn new(name: &str, executable: &Executable, creator: User) -> Option<Self> {
        let port = get_free_port().await?;

        Some(Self {
            name: name.to_string(),
            port,
            executable: executable.clone(),
            creator: creator.clone(),
            created_at: Utc::now(),
            state: ServiceState::Pending,
            error: None,
            kill: None,
            child: None,
        })
    }

    pub fn set_state(&mut self, state: ServiceState, error: Option<String>) {
        self.state = state;
        self.error = error;
    }

    pub fn user(&self) -> &User {
        &self.creator
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn hash(&self) -> &str {
        self.executable.hash()
    }

    pub fn trigger_hash(&self) -> &str {
        self.executable.trigger_hash()
    }

    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    pub fn start(&mut self, config: &Config) {
        // collect command args and replace port number
        let args = config
            .command_args
            .iter()
            .map(|arg| arg.replace("{port}", &self.port.to_string()))
            .collect::<Vec<_>>();

        // start the service / run the command
        let mut child = match Command::new(self.executable.path())
            .args(args)
            .stderr(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                self.state = ServiceState::Error;
                self.error = Some(format!("Failed to start service: {:?}", e));
                return;
            }
        };

        // Create a oneshot channel to kill the child process
        let (kill, recv_kill) = oneshot::channel::<()>();

        // Store the child process and the kill channel
        let port = self.port;
        self.kill = Some(kill);
        self.child = Some(tokio::task::spawn(async move {
            tokio::select! {
                result = child.wait() => {
                    if let Err(e) = result {
                        error!("Child error: {:?}", e);
                    }
                }
                _ = recv_kill => {
                    info!("Killing child on port {}", port);
                    if let Err(e) = child.kill().await {
                        error!("Child kill error: {:?}", e);
                    }
                }
            }

            info!("Finished child on port {}", port);
        }));
    }

    // Stop the service by sending a signal to the kill channel
    pub fn stop(self) -> Result<()> {
        match (self.kill, self.child) {
            (Some(kill), Some(_)) => {
                let _ = kill.send(());
            }
            _ => {
                error!("Service is not running");
            }
        }

        Ok(())
    }
}
