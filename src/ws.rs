use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use tracing::{error, info, warn};

use crate::{
    error::AppError,
    events::Event,
    user::{GitHubUser, User},
    AppState,
};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(caller): Path<String>,
    State(state): State<AppState>,
    user: Option<GitHubUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::from_request(caller, user)?;

    Ok(ws.on_upgrade(|socket| handle_socket(socket, user, state)))
}

// Route messags between the internal bus and the websocket
async fn handle_socket(mut socket: WebSocket, user: User, state: AppState) {
    let mut receiver = state.channel.get_receiver();

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Text(msg)) => {
                        let Ok(event) = serde_json::from_str::<Event>(&msg) else {
                            error!("Invalid event: {msg}");
                            continue;
                        };

                        if event.is_client_event() {
                            state.channel.send(event.update_user(user.clone()));
                        } else {
                            error!("Invalid client event: {msg}");
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Connection closed by client, user {user}");
                        break;
                    }
                    Err(e) => {
                        warn!("Error, closing connection: {e:?}, user {user}");
                        break;
                    }
                    _ => {
                        error!("Invalid message: {msg:?}, user {user}");
                    }
                }
            }
            Ok(event) = receiver.recv() => {
                if event.should_forward(&user) {
                    if let Ok(msg) = serde_json::to_string(&event) {
                        if let Err(e) = socket.send(Message::Text(msg.into())).await {
                            warn!("Socket error {e}, user {user}");
                            break;
                        }
                    }
                }
            }
            else => break,
        }
    }
}
