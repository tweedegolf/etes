use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::error;

use crate::{executable::ExecutableData, github::GitHubState, service::ServiceData, user::User};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Pending,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    // Client requests
    GithubRefresh {
        user: User,
    },
    StartService {
        executable: ExecutableData,
        name: String,
        user: User,
    },
    StopService {
        name: String,
        user: User,
    },
    // Server responses
    Error {
        message: String,
        user: User,
    },
    GithubState {
        payload: GitHubState,
    },
    ServiceState {
        services: Vec<ServiceData>,
    },
    ExecutablesState {
        executables: Vec<ExecutableData>,
    },
    MemoryState {
        used: u64,
        total: u64,
    },
}

impl Event {
    pub fn caller(&self) -> Option<&User> {
        match self {
            Event::GithubRefresh { user, .. } => Some(user),
            Event::StartService { user, .. } => Some(user),
            Event::StopService { user, .. } => Some(user),
            Event::Error { user, .. } => Some(user),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Event::ExecutablesState { .. } => "executables_state",
            Event::GithubRefresh { .. } => "github_refresh",
            Event::GithubState { .. } => "github_state",
            Event::StartService { .. } => "run",
            Event::ServiceState { .. } => "service_state",
            Event::StopService { .. } => "stop_service",
            Event::Error { .. } => "error",
            Event::MemoryState { .. } => "memory_state",
        }
    }

    pub fn is_memory_state(&self) -> bool {
        matches!(self, Event::MemoryState { .. })
    }

    pub fn should_forward(&self, user: &User) -> bool {
        match self {
            Event::Error {
                user: event_user, ..
            } => user == event_user,
            e if e.is_client_event() => false,
            _ => true,
        }
    }

    pub fn is_client_event(&self) -> bool {
        matches!(
            self,
            Event::GithubRefresh { .. } | Event::StartService { .. } | Event::StopService { .. }
        )
    }

    pub fn update_user(self, user: User) -> Self {
        match self {
            Event::GithubRefresh { .. } => Event::GithubRefresh { user },
            Event::StartService {
                executable, name, ..
            } => Event::StartService {
                executable,
                name,
                user,
            },
            Event::StopService { name, .. } => Event::StopService { name, user },
            Event::Error { message, .. } => Event::Error { message, user },
            event => event,
        }
    }
}

pub struct EventManager {
    sender: broadcast::Sender<Event>,
}

impl EventManager {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(512);

        Self { sender }
    }

    pub fn send(&self, event: Event) {
        if let Err(e) = self.sender.send(event) {
            error!("Failed to send event: {e:?}");
        }
    }

    pub fn get_receiver(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}
