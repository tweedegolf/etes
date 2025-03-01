use anyhow::Result;
use auth::GithubOauthService;
use axum::{
    body::Body,
    extract::FromRef,
    routing::{any, get, put},
    Router,
};
use cookie::Key;
use github::GitHubStateManager;
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use std::{ops::Deref, sync::Arc};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use ws::ws_handler;

use crate::{
    config::Config, data::data_handler, events::EventManager, monitor::SystemMonitor,
    services::ServiceManager, upload::upload_handler,
};

pub const GITHUB_BASE_URL: &str = "https://github.com";

mod auth;
mod config;
mod data;
mod error;
mod events;
mod executable;
mod github;
mod monitor;
mod proxy;
mod service;
mod services;
mod upload;
mod user;
mod util;
mod ws;

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

// Global application state
struct AppStateContainer {
    config: &'static Config,
    client: Client,
    oauth: GithubOauthService,
    github: GitHubStateManager,
    services: ServiceManager,
    channel: EventManager,
    monitor: SystemMonitor,
}

#[derive(Clone)]
struct AppState(Arc<AppStateContainer>);

impl Deref for AppState {
    type Target = AppStateContainer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRef<AppState> for GithubOauthService {
    fn from_ref(state: &AppState) -> GithubOauthService {
        state.oauth.clone()
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Key {
        state.oauth.session_key()
    }
}

impl From<AppStateContainer> for AppState {
    fn from(state: AppStateContainer) -> Self {
        Self(Arc::new(state))
    }
}

impl AppStateContainer {
    fn new() -> Result<Self> {
        let config = Config::from_env()?;

        let client: Client =
            hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
                .build(HttpConnector::new());

        let oauth = GithubOauthService::new(config)?;

        Ok(Self {
            config,
            oauth,
            client,
            github: GitHubStateManager::new(),
            services: ServiceManager::new(),
            channel: EventManager::new(),
            monitor: SystemMonitor::new(),
        })
    }

    async fn init(state: AppState) {
        if let Err(e) = state.github.update(state.config).await {
            error!("Failed to fetch GitHub data: {e:?}");
        }

        if let Err(e) = executable::remove_unused_executables(state.clone()).await {
            error!("Failed to remove unused executables: {e:?}");
        }
    }

    async fn spawn_workers(state: AppState) {
        tokio::spawn(monitor::send_updates(state.clone()));
        tokio::spawn(github::refresh_github_data(state.clone()));
        tokio::spawn(services::start_and_stop_services(state.clone()));
    }
}

async fn app() -> Result<(AppState, Router)> {
    let state: AppState = AppStateContainer::new()?.into();
    let index = include_str!("../frontend/index.html").replace("%FAVICON%", &state.config.favicon);

    let frontend = spaxum::load!(&state.config.title).set_html_template(index);

    let app = Router::new()
        .merge(frontend.router())
        .route("/etes/login", get(auth::login))
        .route("/etes/logout", get(auth::logout))
        .route("/etes/authorize", get(auth::authorize))
        .route("/etes/api/v1/ws/{caller}", get(ws_handler))
        .route(
            "/etes/api/v1/executable/{trigger_hash}/{build_hash}",
            put(upload_handler),
        )
        .route("/etes/api/v1/data/{caller}", get(data_handler))
        .with_state(state.clone());

    Ok((state, app))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .without_time(),
        )
        .init();

    let (state, app) = app().await?;

    AppStateContainer::init(state.clone()).await;
    AppStateContainer::spawn_workers(state.clone()).await;

    let proxy_app: Router = Router::new()
        .fallback(any(proxy::handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;

    info!(
        "Starting server on {} and {}",
        listener.local_addr()?,
        proxy_listener.local_addr()?
    );

    tokio::select! {
        _ = axum::serve(listener, app) => {}
        _ = axum::serve(proxy_listener, proxy_app) => {}
    }

    Ok(())
}
