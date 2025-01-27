use std::fmt::{self, Display, Formatter};

use anyhow::anyhow;
use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts, State},
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
    RequestPartsExt,
};
use axum_extra::extract::PrivateCookieJar;
use serde::{Deserialize, Serialize};

use crate::{
    auth::{GithubOauthService, COOKIE_NAME},
    config::Config,
    error::AppError,
    util::{is_valid_name, sha256},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum User {
    Anonymous(String),
    GitHub(GitHubUser),
}

impl User {
    pub fn from_request(caller: String, user: Option<GitHubUser>) -> Result<Self, AppError> {
        if let Some(user) = user {
            Ok(User::GitHub(user))
        } else {
            if !is_valid_name(&caller) {
                return Err(AppError::Client(anyhow!("Invalid caller name")));
            }

            Ok(User::Anonymous(caller))
        }
    }

    pub fn is_admin(&self, config: &Config) -> bool {
        match self {
            User::GitHub(user) => config.admins.contains(&user.login),
            _ => false,
        }
    }

    pub fn hash_anonymous(&self) -> User {
        match self {
            User::Anonymous(id) => User::Anonymous(sha256(id)),
            user => user.clone(),
        }
    }
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            User::Anonymous(id) => write!(f, "Anonymous({})", id),
            User::GitHub(user) => write!(f, "GitHub({})", user.login),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq)]
pub struct GitHubUser {
    pub login: String,
    pub name: String,
    pub avatar_url: String,
}

impl PartialEq for GitHubUser {
    fn eq(&self, other: &Self) -> bool {
        self.login == other.login
    }
}

/// Represents an action to perform after authentication.
pub enum AuthAction {
    /// Redirects to the specified path.
    Redirect(String),
    /// Represents an error that occurred during authentication.
    Error(AppError),
}

impl IntoResponse for AuthAction {
    fn into_response(self) -> Response {
        match self {
            Self::Redirect(path) => Redirect::temporary(&path).into_response(),
            Self::Error(e) => e.into_response(),
        }
    }
}

impl<S> OptionalFromRequestParts<S> for GitHubUser
where
    GithubOauthService: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthAction;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let service: State<GithubOauthService> =
            parts.extract_with_state(state).await.map_err(|_| {
                AuthAction::Error(AppError::Server(anyhow!("Authorization service not found")))
            })?;

        let jar: PrivateCookieJar =
            PrivateCookieJar::from_headers(&parts.headers, service.session_key());

        let Some(session_cookie) = jar.get(COOKIE_NAME) else {
            return Ok(None);
        };

        let Ok(user) = serde_json::from_str::<GitHubUser>(session_cookie.value()) else {
            return Ok(None);
        };

        Ok(Some(user))
    }
}

impl<S> FromRequestParts<S> for GitHubUser
where
    GithubOauthService: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthAction;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let service: State<GithubOauthService> =
            parts.extract_with_state(state).await.map_err(|_| {
                AuthAction::Error(AppError::Server(anyhow!("Authorization service not found")))
            })?;

        let jar: PrivateCookieJar =
            PrivateCookieJar::from_headers(&parts.headers, service.session_key());

        let session_cookie = jar
            .get(COOKIE_NAME)
            .ok_or(AuthAction::Redirect("/etes/login".into()))?;

        let user: GitHubUser = serde_json::from_str(session_cookie.value())
            .map_err(|_| AuthAction::Error(AppError::Client(anyhow!("Invalid user cookie"))))?;

        Ok(user)
    }
}
