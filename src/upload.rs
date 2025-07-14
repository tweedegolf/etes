use anyhow::{Result, anyhow};
use axum::{
    extract::{Path, Request, State},
    response::IntoResponse,
};
use constant_time_eq::constant_time_eq;
use futures::TryStreamExt;
use hyper::StatusCode;
use std::{fs::Permissions, io, os::unix::fs::PermissionsExt};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};
use tokio_util::io::StreamReader;
use tracing::{error, info};

use crate::{
    AppState, error::AppError, events::Event, executable::Executable, util::is_valid_hash,
};

pub async fn upload_handler(
    State(state): State<AppState>,
    Path((trigger_hash, build_hash)): Path<(String, String)>,
    request: Request,
) -> Result<impl IntoResponse, AppError> {
    if !is_valid_hash(&trigger_hash) || !is_valid_hash(&build_hash) {
        return Err(AppError::Client(anyhow!("Invalid commit hash")));
    }

    info!("Incoming upload for {trigger_hash} and {build_hash}");

    // get the authorization header
    let authorization = request
        .headers()
        .get("authorization")
        .ok_or_else(|| AppError::Client(anyhow!("No authorization header found")))?
        .to_str()
        .map_err(|_| AppError::Client(anyhow!("Invalid authorization header value")))?
        .strip_prefix("Bearer ")
        .ok_or_else(|| {
            AppError::Client(anyhow!("Missing 'Bearer' in authorization header value"))
        })?;

    // secure string compare
    if !constant_time_eq(authorization.as_bytes(), state.config.api_key.as_bytes()) {
        error!("Invalid API key for upload of {trigger_hash} and {build_hash}");

        return Err(AppError::Client(anyhow!("Invalid API key")));
    }

    // init new executable
    let executable = Executable::from_commit(build_hash.clone(), trigger_hash.clone());

    // delete the file if it already exists
    if executable.path().exists() {
        if let Err(err) = tokio::fs::remove_file(executable.path()).await {
            error!("Failed to remove existing file: {err}");
            return Err(AppError::Server(anyhow!("Failed to remove existing file")));
        }
    }

    // get data stream
    let body_reader = StreamReader::new(
        request
            .into_body()
            .into_data_stream()
            .map_err(io::Error::other),
    );

    futures::pin_mut!(body_reader);

    let mut file = BufWriter::new(File::create(executable.path()).await?);

    // copy the body into the file (streaming)
    tokio::io::copy(&mut body_reader, &mut file).await?;

    // close the file
    file.flush().await?;
    drop(file);

    // make file executable
    tokio::fs::set_permissions(executable.path(), Permissions::from_mode(0o755)).await?;

    info!("Uploaded {trigger_hash} and {build_hash}");

    // update state
    state.services.update_executables().await;

    // set updated state to all clients
    state.channel.send(Event::ExecutablesState {
        executables: state.services.get_executables(),
    });

    if state.github.update(state.config).await.is_ok() {
        state.channel.send(Event::GithubState {
            payload: state.github.get_state(),
        });
    }

    Ok((
        StatusCode::CREATED,
        format!("Upload of executable for {trigger_hash} and {build_hash} successful"),
    ))
}

#[cfg(test)]
mod test {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use hyper::Method;
    use tower::ServiceExt;

    use crate::{app, executable::Executable};

    #[tokio::test]
    async fn test_upload_handler() {
        let (state, app) = app(false).await.unwrap();

        let hash1 = "1111111111111111111111111111111111111111";
        let hash2 = "2222222222222222222222222222222222222222";

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/etes/api/v1/executable/{hash1}/{hash2}"))
                    .header("Authorization", format!("Bearer {}", state.config.api_key))
                    .body(Body::new("test".to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 201);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            String::from_utf8_lossy(&body),
            format!("Upload of executable for {hash1} and {hash2} successful")
        );

        let executable = Executable::from_commit(hash2.into(), hash1.into());
        tokio::fs::remove_file(executable.path()).await.unwrap();
    }
}
