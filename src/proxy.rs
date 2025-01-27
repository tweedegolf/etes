use anyhow::{anyhow, Context};
use axum::{
    extract::{Request, State},
    response::{Html, IntoResponse, Redirect, Response},
    RequestExt,
};
use axum_extra::extract::Host;
use hyper::{StatusCode, Uri};

use crate::{
    error::AppError,
    user::{GitHubUser, User},
    util::{get_random_name, is_valid_hash, random_string},
    AppState,
};

fn not_found(domain: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Html(format!("<h1>No service found on this domain.</h1><h2>Visit <a href=\"https://{domain}\">{domain}</a> to view a list of running instances.</h2>")),
    )
        .into_response()
}

async fn redirect_to_service(
    state: AppState,
    domain: &str,
    user: User,
    commit_hash: &str,
) -> Result<Response, AppError> {
    // find exsisting service
    if let Some(name) = state.services.get_name_by_commit(commit_hash) {
        // redirect to service
        return Ok(Redirect::temporary(&format!("https://{name}.{domain}")).into_response());
    }

    // start up new service
    let name = get_random_name(&state.config.words);
    state
        .services
        .start_service(&name, &commit_hash.into(), user, state.clone())
        .await;

    // redirect to service
    Ok(Redirect::temporary(&format!("https://{name}.{domain}")).into_response())
}

pub async fn handler(
    State(state): State<AppState>,
    user: Option<GitHubUser>,
    mut req: Request,
) -> Result<Response, AppError> {
    let host = req
        .extract_parts::<Host>()
        .await
        .context("No request host found")?;

    let subdomain = host
        .0
        .split('.')
        .next()
        .context("Could not determine subdomain")?;

    let domain = host.0.split('.').skip(1).collect::<Vec<&str>>().join(".");

    if is_valid_hash(subdomain) {
        let user = User::from_request(random_string(), user)?;

        return redirect_to_service(state, &domain, user, subdomain).await;
    }

    // Check if the subdomain is a valid service
    let Some(port) = state.services.get_port(subdomain) else {
        // Return a 404 response, with a link to the homepage
        return Ok(not_found(&domain));
    };

    // Update the request URI to point to the service
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = format!("http://127.0.0.1:{}{path_query}", port);
    *req.uri_mut() = Uri::try_from(uri).unwrap();

    // Forward the request to the service
    Ok(state
        .client
        .request(req)
        .await
        .map_err(|_| anyhow!("Upstream error"))?
        .into_response())
}
