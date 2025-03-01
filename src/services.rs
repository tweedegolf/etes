use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{error, info};

use crate::{
    AppState, Config,
    events::{Event, ServiceState},
    executable::{Executable, ExecutableData, get_executables},
    github::CommitHash,
    service::{Service, ServiceData},
    user::User,
    util::is_valid_name,
};

pub struct ServiceManager {
    services: Arc<RwLock<HashMap<String, Service>>>,
    executables: Arc<RwLock<Vec<Executable>>>,
}

impl ServiceManager {
    // Construct initial state, list exsisting executables
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            executables: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Get the state of all services
    pub fn get_state(&self) -> Vec<ServiceData> {
        let services = self.services.read();

        let mut services = services
            .iter()
            .map(|(_, service)| service.into())
            .collect::<Vec<ServiceData>>();

        services.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        services
    }

    // Add a new service, check if the service already exists, get the executable for the commit
    async fn add_service(
        &self,
        name: &str,
        executable: &Executable,
        creator: User,
        config: &Config,
    ) -> anyhow::Result<String> {
        if self.services.read().contains_key(name) {
            return Err(anyhow::anyhow!("Service {} already exists!", name));
        }

        info!("Starting service {name}");
        let Some(mut service) = Service::new(name, executable, creator).await else {
            return Err(anyhow::anyhow!("Failed to start service: no free port"));
        };

        // Start and add the service
        service.start(config);
        let error = service.error();
        self.services.write().insert(name.to_string(), service);

        match error {
            Some(e) => Err(anyhow::anyhow!(e)),
            None => Ok(name.to_string()),
        }
    }

    // Wait for the service to start, check if the service is running
    pub async fn wait_for_startup(&self, name: &str) -> Result<()> {
        let port = match self.services.read().get(name) {
            Some(service) => service.port(),
            None => return Err(anyhow::anyhow!("Service {} not found", name)),
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()?;

        for i in 0..10 {
            info!("Checking ({i}) service on port {}", port);

            if let Ok(response) = client
                .get(format!("http://127.0.0.1:{}/", port))
                .send()
                .await
            {
                if response.status().is_success() {
                    self.set_service_state(name, ServiceState::Running, None);

                    return Ok(());
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        self.set_service_state(
            name,
            ServiceState::Error,
            Some("Service did not start".to_owned()),
        );

        Err(anyhow!("Service did not start"))
    }

    // Set the state of the service with a possible error message
    fn set_service_state(&self, name: &str, state: ServiceState, error: Option<String>) {
        if let Some(service) = self.services.write().get_mut(name) {
            service.set_state(state, error);
        }
    }

    // Remove a service from the list
    fn remove_service(&self, name: &str) -> Option<Service> {
        self.services.write().remove(name)
    }

    // Get the list of executable commit hashes
    pub fn get_executables(&self) -> Vec<ExecutableData> {
        self.executables
            .read()
            .iter()
            .map(|executable| executable.into())
            .collect()
    }

    // Update the list of executables
    pub async fn update_executables(&self) {
        let executables = get_executables().await;

        *self.executables.write() = executables;
    }

    // Get the port of a service by a name
    pub fn get_port(&self, name: &str) -> Option<u16> {
        if let Some(service) = self.services.read().get(name) {
            return Some(service.port());
        }

        None
    }

    // Check if the caller is the owner of the service, or is the admin
    pub fn is_owner(&self, name: &str, user: &User, config: &Config) -> bool {
        if let Some(service) = self.services.read().get(name) {
            return service.user() == user || user.is_admin(config);
        }

        false
    }

    // Get the executable by the commit hash
    pub fn get_executable_by_commit(&self, commit: &CommitHash) -> Option<Executable> {
        self.executables
            .read()
            .iter()
            .find(|executable| executable.hash() == commit || executable.trigger_hash() == commit)
            .cloned()
    }

    // Get the name of a service by the commit hash
    pub fn get_name_by_commit(&self, commit: &str) -> Option<String> {
        let services = self.services.read();

        for (name, service) in services.iter() {
            if service.hash() == commit || service.trigger_hash() == commit {
                return Some(name.clone());
            }
        }

        None
    }

    // Stop a service, check if the caller is the owner
    async fn stop_service(&self, name: &str, user: User, state: AppState) {
        if !self.is_owner(name, &user, state.config) {
            state.channel.send(Event::Error {
                message: "You are not the owner of this service".to_owned(),
                user,
            });

            return;
        }

        if let Some(service) = self.remove_service(name) {
            if let Err(e) = service.stop() {
                error!("Failed to stop service {}: {:?}", name, e);
            }
        }

        state.channel.send(Event::ServiceState {
            services: self.get_state(),
        });
    }

    // Start a service, check if the commit exists, check if the name is alphanumeric
    pub async fn start_service(
        &self,
        name: &str,
        commit_hash: &CommitHash,
        user: User,
        state: AppState,
    ) {
        // Check if the commit exists
        let executable = match self.get_executable_by_commit(commit_hash) {
            Some(executable) => executable,
            None => {
                state.channel.send(Event::Error {
                    message: "Executable not found".to_owned(),
                    user,
                });

                return;
            }
        };

        // check name is alphanumeric
        if !is_valid_name(name) {
            state.channel.send(Event::Error {
                message: "Service name must be alphanumeric".to_owned(),
                user,
            });
            return;
        }

        // Add and start the service
        match self
            .add_service(name, &executable, user.clone(), state.config)
            .await
        {
            Ok(_) => {
                state.channel.send(Event::ServiceState {
                    services: self.get_state(),
                });

                if let Err(e) = self.wait_for_startup(name).await {
                    error!("Failed to start service {}: {:?}", name, e);
                    state.channel.send(Event::Error {
                        message: format!("Failed to start service: {}", e),
                        user,
                    });
                } else {
                    info!("Started service {}", name);
                    state.channel.send(Event::ServiceState {
                        services: self.get_state(),
                    });
                }
            }
            Err(e) => {
                error!("Failed to start service: {}", e);
                state.channel.send(Event::Error {
                    message: format!("Failed to start service: {}", e),
                    user,
                });

                state.channel.send(Event::ServiceState {
                    services: self.get_state(),
                });
            }
        }
    }
}

pub async fn start_and_stop_services(state: AppState) -> Result<()> {
    let mut receiver = state.channel.get_receiver();

    while let Ok(event) = receiver.recv().await {
        match event {
            Event::StopService { name, user } => {
                let state = state.clone();
                tokio::task::spawn(async move {
                    state
                        .services
                        .stop_service(&name, user, state.clone())
                        .await;
                });
            }
            Event::StartService {
                executable,
                name,
                user,
            } => {
                let state = state.clone();
                tokio::task::spawn(async move {
                    state
                        .services
                        .start_service(&name, executable.hash(), user, state.clone())
                        .await;
                });
            }
            _ => {
                // Debug print log all incoming events
                if !event.is_memory_state() {
                    match event.caller() {
                        Some(caller) => info!("Received event {} from {}", event.name(), caller),
                        None => info!("Received event: {}", event.name()),
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{
        AppState, AppStateContainer,
        events::{Event, ServiceState},
        executable::Executable,
        services::start_and_stop_services,
        user::User,
    };

    #[tokio::test]
    async fn test_service_manager() {
        let state: AppState = AppStateContainer::new().unwrap().into();

        assert_eq!(state.config.command_args[0], "{port}");

        let hash = "ffffffffffffffffffffffffffffffffffffffff".to_string();
        let executable = Executable::from_commit(hash.clone(), hash.clone());

        let _ = tokio::fs::remove_file(executable.path()).await;
        tokio::fs::copy("test/hello-world", executable.path())
            .await
            .unwrap();

        let mut receiver = state.channel.get_receiver();

        state.services.update_executables().await;

        let job = tokio::task::spawn(start_and_stop_services(state.clone()));

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        state.channel.send(Event::StartService {
            executable: (&executable).into(),
            name: "foobar".to_string(),
            user: User::Anonymous("frank".to_string()),
        });

        let event = receiver.recv().await.unwrap();

        let Event::StartService { .. } = event else {
            panic!("Expected StartService event, got {event:?}");
        };

        let event = receiver.recv().await.unwrap();

        let Event::ServiceState { services } = event else {
            panic!("Expected ServiceData event, got {event:?}");
        };

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "foobar");
        assert_eq!(services[0].state, ServiceState::Pending);

        let event = receiver.recv().await.unwrap();

        let Event::ServiceState { services } = event else {
            panic!("Expected ServiceData event, got {event:?}");
        };

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "foobar");
        assert_eq!(services[0].state, ServiceState::Running);

        state.channel.send(Event::StopService {
            name: "foobar".to_string(),
            user: User::Anonymous("frank".to_string()),
        });

        let event = receiver.recv().await.unwrap();

        let Event::StopService { .. } = event else {
            panic!("Expected StopService event, got {event:?}");
        };

        let event = receiver.recv().await.unwrap();

        let Event::ServiceState { services } = event else {
            panic!("Expected ServiceData event, got {event:?}");
        };

        assert_eq!(services.len(), 0);

        job.abort();
        let _ = job.await;

        tokio::fs::remove_file(executable.path()).await.unwrap();
    }
}
