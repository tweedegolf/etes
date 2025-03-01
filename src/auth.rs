/// This module contains the request handlers for the GitHub OAuth flow.
/// It includes functions for login, logout, and authorization.
use anyhow::{Context, anyhow};
use axum::{
    extract::{FromRef, Query, State},
    http::HeaderValue,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use cookie::{Key, SameSite};
use hyper::header::{ACCEPT, USER_AGENT};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl, basic::BasicClient, reqwest::async_http_client,
};
use serde::Deserialize;
use std::fmt::Debug;

use crate::{config::Config, error::AppError, user::GitHubUser, util::sha512};

pub static COOKIE_NAME: &str = "SESSION";
static CSRF_COOKIE_NAME: &str = "CSRF";
static USER_AGENT_VALUE: &str = "etes";

static GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
static GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
static GITHUB_USER_URL: &str = "https://api.github.com/user";
static GITHUB_ACCEPT_TYPE: &str = "application/vnd.github+json";

#[derive(Clone)]
pub struct GithubOauthService {
    oauth_client: BasicClient,
    session_key: Key,
}

impl FromRef<GithubOauthService> for Key {
    fn from_ref(state: &GithubOauthService) -> Key {
        state.session_key.clone()
    }
}

impl GithubOauthService {
    /// Creates a new instance of `GithubOauthService`.
    /// Returns a `Result` containing the `GithubOauthService` instance or an `Error` if there was an error creating the service.
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let oauth_client = BasicClient::new(
            ClientId::new(config.github_client_id.clone()),
            Some(ClientSecret::new(config.github_client_secret.clone())),
            AuthUrl::from_url(GITHUB_AUTH_URL.parse()?),
            Some(TokenUrl::from_url(GITHUB_TOKEN_URL.parse()?)),
        )
        .set_redirect_uri(RedirectUrl::from_url(config.authorize_url.parse()?));

        let session_key: Key = Key::from(&sha512(&config.session_key));

        Ok(Self {
            oauth_client,
            session_key,
        })
    }

    /// Returns the session key for the service.
    pub fn session_key(&self) -> Key {
        self.session_key.clone()
    }
}

/// Handles the login request.
/// Generates the authorization URL and CSRF token, sets the CSRF token as a cookie,
/// and redirects the user to the authorization URL.
///
/// # Parameters
///
/// - `service`: The `GithubOauthService` instance.
/// - `jar`: The private cookie jar to store the CSRF token cookie.
///
/// # Returns
///
/// Returns a `Result` containing the updated cookie jar and a `Redirect` response.
/// If successful, the user will be redirected to the authorization URL.
///
/// # Errors
///
/// Returns an `Error` if there is an issue generating the CSRF token or setting the cookie.
#[axum::debug_handler]
pub(super) async fn login(
    State(service): State<GithubOauthService>,
    jar: PrivateCookieJar,
) -> Result<impl IntoResponse, AppError> {
    // Generate the authorization URL and CSRF token
    let (auth_url, csrf_token) = service
        .oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .url();

    // Serialize the CSRF token as a string
    let csrf_cookie_value = serde_json::to_string(&csrf_token)?;

    // Create a new CSRF token cookie
    let mut csrf_cookie = Cookie::new(CSRF_COOKIE_NAME, csrf_cookie_value);

    // Set cookie attributes
    csrf_cookie.set_http_only(true);
    csrf_cookie.set_secure(true);
    csrf_cookie.set_same_site(SameSite::Lax);
    csrf_cookie.set_max_age(cookie::time::Duration::minutes(60));
    csrf_cookie.set_path("/");

    // Add the CSRF token cookie to the cookie jar
    let updated_jar = jar.add(csrf_cookie);

    // Return the updated cookie jar and a redirect response to the authorization URL
    Ok((updated_jar, Redirect::to(auth_url.to_string().as_str())))
}

/// Handles the logout request.
/// Removes the session cookie from the cookie jar and returns a simple message indicating successful logout.
///
/// # Parameters
///
/// - `jar`: The private cookie jar containing the session cookie.
///
/// # Returns
///
/// Returns a tuple containing the updated cookie jar and a simple logout message.
pub(super) async fn logout(mut jar: PrivateCookieJar) -> impl IntoResponse {
    // Remove the session cookie from the cookie jar
    if let Some(mut cookie) = jar.get(COOKIE_NAME) {
        cookie.set_same_site(SameSite::Lax);
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_path("/");
        jar = jar.remove(cookie);
    }

    // Return the updated cookie jar and a logout message
    (jar, Redirect::to("/"))
}

/// Represents the request parameters for the authorization request.
#[derive(Debug, Deserialize)]
pub(super) struct AuthRequest {
    code: String,
    state: String,
}

/// Handles the authorization request.
/// Exchanges the authorization code for an access token,
/// validates the CSRF token, fetches user data,
/// and sets the session cookie if the user is authorized.
///
/// # Parameters
///
/// - `service`: The `GithubOauthService` instance.
/// - `Query(query)`: The query parameters containing the authorization code and CSRF token.
/// - `jar`: The private cookie jar containing the CSRF token cookie.
///
/// # Returns
///
/// Returns a `Result` containing the updated cookie jar and a redirect response to the home page.
/// If successful, the user will be redirected to the home page with the session cookie set.
///
/// # Errors
///
/// Returns an `Error` if there is an issue exchanging the authorization code for an access token,
/// validating the CSRF token, or setting the session cookie.
pub(super) async fn authorize(
    State(service): State<GithubOauthService>,
    Query(query): Query<AuthRequest>,
    jar: PrivateCookieJar,
) -> Result<Response, AppError> {
    // Exchange the authorization code for an access token
    let token = service
        .oauth_client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(async_http_client)
        .await
        .context("Invalid token provided")?;

    // Get the CSRF token cookie from the cookie jar
    let mut csrf_cookie = jar
        .get(CSRF_COOKIE_NAME)
        .context("Missing CSRF cookie")
        .map_err(AppError::Client)?;

    // Set cookie attributes
    csrf_cookie.set_same_site(SameSite::Lax);
    csrf_cookie.set_http_only(true);
    csrf_cookie.set_secure(true);
    csrf_cookie.set_path("/");

    // Deserialize the CSRF token from the cookie value
    let csrf_token: CsrfToken = serde_json::from_str(csrf_cookie.value())?;

    // Validate the CSRF token
    if query.state != *csrf_token.secret() {
        return Err(AppError::Client(anyhow!("Invalid CSRF token")));
    }

    // Create a new HTTP client
    let client = reqwest::Client::new();

    // Fetch user data from the GitHub API
    let user: GitHubUser = client
        .get(GITHUB_USER_URL)
        .header(ACCEPT, HeaderValue::from_static(GITHUB_ACCEPT_TYPE))
        .header(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE))
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .context("Failed to fetch user data")?
        .json()
        .await
        .context("Failed te deserialize GitHub user data")?;

    // Serialize the user data as a string
    let session_cookie_value = serde_json::to_string(&user)?;

    // Create a new session cookie
    let mut session_cookie = Cookie::new(COOKIE_NAME, session_cookie_value);
    session_cookie.set_http_only(true);
    session_cookie.set_secure(true);
    session_cookie.set_same_site(cookie::SameSite::Lax);
    session_cookie.set_max_age(cookie::time::Duration::days(30));
    session_cookie.set_path("/");

    // Remove the CSRF token cookie and add the session cookie to the cookie jar
    let updated_jar = jar.remove(csrf_cookie).add(session_cookie);

    // Return the updated cookie jar and a redirect response to the home page
    Ok((updated_jar, Redirect::to("/")).into_response())
}
