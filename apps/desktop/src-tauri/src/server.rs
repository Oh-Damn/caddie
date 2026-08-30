use crate::protocol::{
    AckPayload, AgentsPayload, AppsPayload, ClipboardPayload, CommandPayload, EnvelopeIn,
    EnvelopeOut, ErrorPayload, HealthPayload, HelloOkPayload, HelloPayload, InfoPayload,
    PairDenyPayload, PairOkPayload, PairPayload, PresencePayload, QueryPayload, PROTOCOL_VERSION,
};
use crate::state::{pwa_dist, AppState, SnapshotDelta};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, Path, Query, State, WebSocketUpgrade};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;
use axum::Router;
use std::net::SocketAddr;
use tokio::time::{interval, Duration};
use tower_http::services::{ServeDir, ServeFile};

#[derive(serde::Deserialize)]
struct ArtworkQuery {
    url: String,
}

#[derive(serde::Deserialize)]
struct ClipQuery {
    #[serde(default)]
    t: String,
    #[serde(default)]
    size: String,
}

#[derive(serde::Deserialize)]
struct IconQuery {
    #[serde(default)]
    t: String,
}

pub async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(guard) = crate::mdns::advertise(state.port()) {
        state.set_mdns(guard);
    } else {
        tracing::warn!("mDNS unavailable");
    }

    #[cfg(target_os = "macos")]
    state.start_app_watch();
    state.start_agent_watch();

    let watcher = state.clone();
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(1200));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut idle = interval(Duration::from_secs(5));
        idle.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            if !watcher.wants_updates() {
                idle.tick().await;
                continue;
            }
            tick.tick().await;
            if !watcher.wants_updates() {
                continue;
            }
            watcher.set_lan_ip(crate::state::lan_ip());
            let w = watcher.clone();
            let delta = tokio::task::spawn_blocking(move || w.try_refresh_snapshot())
                .await
                .ok()
                .flatten();
            match delta {
                None | Some(SnapshotDelta::None) => {}
                Some(SnapshotDelta::Position) => watcher.notify_state(),
                Some(SnapshotDelta::Full) => watcher.notify(),
            }
        }
    });

    let rotator = state.clone();
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(120));
        tick.tick().await;
        loop {
            tick.tick().await;
            rotator.rotate_secret();
        }
    });

    let dist = pwa_dist();
    let mut app = Router::new()
        .route("/ws", get(ws_upgrade))
        .route("/api/health", get(health))
        .route("/api/info", get(info))
        .route("/api/icon/{bundle}", get(icon))
        .route("/api/artwork", get(artwork))
        .route("/api/clip/{id}", get(clip_image))
        .route("/manifest.json", get(manifest))
        .with_state(state.clone());

    if dist.join("index.html").exists() {
        let index = ServeFile::new(dist.join("index.html"));
        app = app.fallback_service(ServeDir::new(&dist).not_found_service(index));
    }
    app = app.layer(axum::middleware::from_fn(cache_policy));

    let addr = SocketAddr::from(([0, 0, 0, 0], state.port()));
    tracing::info!("listening on {addr} (http)");
    axum_server::bind(addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;
    Ok(())
}

async fn cache_policy(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let path = req.uri().path().to_string();
    let mut res = next.run(req).await;
    if let Some(value) = cache_value(&path) {
        res.headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    }
    res
}

fn cache_value(path: &str) -> Option<&'static str> {
    if path.starts_with("/api/") || path.starts_with("/ws") {
        return None;
    }
    if path.starts_with("/assets/") {
        return Some("public, max-age=31536000, immutable");
    }
    Some("no-cache")
}

async fn health() -> Json<HealthPayload> {
    Json(HealthPayload { ok: true })
}

async fn info() -> Json<InfoPayload> {
    Json(InfoPayload {
        name: crate::protocol::PRODUCT_NAME.into(),
        proto: PROTOCOL_VERSION,
    })
}

async fn manifest() -> impl IntoResponse {
    let path = pwa_dist().join("manifest.json");
    match tokio::fs::read(&path).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "application/manifest+json")], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn icon(
    Path(bundle): Path<String>,
    Query(query): Query<IconQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if query.t.is_empty() || !state.is_trusted(&query.t) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match state.app_icon(&bundle) {
        Some(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "public, max-age=86400"),
            ],
            bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn artwork_url_allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    host == "i.scdn.co" || host.ends_with(".scdn.co") || host.ends_with(".mzstatic.com")
}

async fn artwork(Query(query): Query<ArtworkQuery>) -> impl IntoResponse {
    if !artwork_url_allowed(&query.url) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let output = tokio::process::Command::new("curl")
        .args(["-sL", "--max-time", "10", &query.url])
        .output()
        .await;
    let Ok(output) = output else {
        return StatusCode::BAD_GATEWAY.into_response();
    };
    if !output.status.success() || output.stdout.is_empty() {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let content_type = if output.stdout.starts_with(b"\x89PNG") {
        "image/png"
    } else {
        "image/jpeg"
    };
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        output.stdout,
    )
        .into_response()
}

async fn clip_image(
    Path(id): Path<String>,
    Query(query): Query<ClipQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if query.t.is_empty() || !state.is_trusted(&query.t) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(path) = state.clipboard_image_file(&id, query.size != "full") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "private, max-age=60"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, peer.ip()))
}

enum SocketJob {
    Command {
        id: String,
        result: Result<(), String>,
    },
    Apps {
        id: String,
        payload: AppsPayload,
    },
    Clipboard {
        id: String,
        payload: ClipboardPayload,
    },
    Agents {
        id: String,
        payload: AgentsPayload,
    },
}

async fn handle_socket(mut socket: WebSocket, state: AppState, peer: std::net::IpAddr) {
    let mut trusted = false;
    let mut token: Option<u64> = None;
    let (evict_tx, mut evict_rx) = tokio::sync::mpsc::channel::<()>(1);
    let mut events = state.subscribe();
    let mut ping = interval(Duration::from_secs(20));
    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let (jobs_tx, mut jobs_rx) = tokio::sync::mpsc::channel::<SocketJob>(8);

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else { break; };
                let Message::Text(text) = msg else { continue; };
                let parsed: EnvelopeIn = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if parsed.v != PROTOCOL_VERSION {
                    let _ = send_json(
                        &mut socket,
                        EnvelopeOut::new(
                            parsed.id,
                            "error",
                            ErrorPayload {
                                code: "version".into(),
                                message: "unsupported protocol".into(),
                            },
                        ),
                    )
                    .await;
                    continue;
                }
                match parsed.kind.as_str() {
                    "hello" => {
                        let payload: HelloPayload = match serde_json::from_value(parsed.payload) {
                            Ok(v) => v,
                            Err(err) => {
                                tracing::warn!("hello payload: {err}");
                                continue;
                            }
                        };
                        tracing::debug!("hello from {}", payload.device_name);
                        let device_id = payload.device_id.clone().filter(|id| state.is_trusted(id));
                        trusted = device_id.is_some();
                        if let Some(id) = device_id.as_ref() {
                            state.mark_seen(
                                id,
                                &payload.device_name,
                                &payload.details.unwrap_or_default(),
                                Some(peer),
                            );
                        }
                        let _ = send_json(
                            &mut socket,
                            EnvelopeOut::new(
                                parsed.id,
                                "hello_ok",
                                HelloOkPayload {
                                    server_name: crate::protocol::PRODUCT_NAME.into(),
                                    trusted,
                                    device_id,
                                },
                            ),
                        )
                        .await;
                        if trusted {
                            if token.is_none() {
                                token = Some(state.claim_client(evict_tx.clone()));
                                wake_snapshot(&state);
                            }
                            let _ = push_layout(&mut socket, &state).await;
                        }
                    }
                    "pair" => {
                        let payload: PairPayload = match serde_json::from_value(parsed.payload) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        if payload.secret != state.pairing_secret() {
                            let _ = send_json(
                                &mut socket,
                                EnvelopeOut::new(
                                    parsed.id,
                                    "pair_deny",
                                    PairDenyPayload {
                                        reason: "invalid_secret".into(),
                                    },
                                ),
                            )
                            .await;
                            continue;
                        }
                        let device_id = uuid::Uuid::new_v4().to_string();
                        let details = payload.details.unwrap_or_default();
                        if let Err(err) = state.trust(
                            device_id.clone(),
                            payload.device_name.clone(),
                            details.clone(),
                        ) {
                            tracing::warn!("persist trust: {err}");
                        }
                        state.mark_seen(&device_id, &payload.device_name, &details, Some(peer));
                        trusted = true;
                        if token.is_none() {
                            token = Some(state.claim_client(evict_tx.clone()));
                            wake_snapshot(&state);
                        }
                        let _ = send_json(
                            &mut socket,
                            EnvelopeOut::new(
                                parsed.id,
                                "pair_ok",
                                PairOkPayload { device_id },
                            ),
                        )
                        .await;
                        let _ = push_layout(&mut socket, &state).await;
                    }
                    "command" => {
                        if !trusted {
                            let _ = send_json(
                                &mut socket,
                                EnvelopeOut::new(
                                    parsed.id,
                                    "error",
                                    ErrorPayload {
                                        code: "unpaired".into(),
                                        message: "pair required".into(),
                                    },
                                ),
                            )
                            .await;
                            continue;
                        }
                        let payload: CommandPayload = match serde_json::from_value(parsed.payload) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let job_state = state.clone();
                        let job_tx = jobs_tx.clone();
                        let id = parsed.id.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = job_state.run_command(
                                &payload.action,
                                payload.value,
                                payload.target,
                            );
                            let _ = job_tx.blocking_send(SocketJob::Command { id, result });
                        });
                    }
                    "presence" => {
                        let payload: PresencePayload =
                            match serde_json::from_value(parsed.payload) {
                                Ok(v) => v,
                                Err(_) => continue,
                            };
                        let Some(token) = token else { continue };
                        if state.set_client_visible(token, payload.visible) {

                            let refresh = state.clone();
                            tokio::task::spawn_blocking(move || {
                                match refresh.try_refresh_snapshot() {
                                    Some(SnapshotDelta::Position) => refresh.notify_state(),
                                    Some(SnapshotDelta::Full) => refresh.notify(),
                                    _ => {}
                                }
                            });
                        }
                    }
                    "query" => {
                        if !trusted {
                            let _ = send_json(
                                &mut socket,
                                EnvelopeOut::new(
                                    parsed.id,
                                    "error",
                                    ErrorPayload {
                                        code: "unpaired".into(),
                                        message: "pair required".into(),
                                    },
                                ),
                            )
                            .await;
                            continue;
                        }
                        let payload: QueryPayload = match serde_json::from_value(parsed.payload) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        match payload.kind.as_str() {
                            "apps" => {
                                let job_state = state.clone();
                                let job_tx = jobs_tx.clone();
                                let id = parsed.id.clone();
                                tokio::task::spawn_blocking(move || {
                                    let payload = job_state.list_apps();
                                    let _ = job_tx.blocking_send(SocketJob::Apps { id, payload });
                                });
                            }
                            "agents" => {
                                let job_state = state.clone();
                                let job_tx = jobs_tx.clone();
                                let id = parsed.id.clone();
                                tokio::task::spawn_blocking(move || {
                                    let payload = job_state.agents_payload();
                                    let _ = job_tx.blocking_send(SocketJob::Agents { id, payload });
                                });
                            }
                            "clipboard" => {
                                let job_state = state.clone();
                                let job_tx = jobs_tx.clone();
                                let id = parsed.id.clone();
                                tokio::task::spawn_blocking(move || {
                                    let payload = job_state.clipboard_payload();
                                    let _ = job_tx
                                        .blocking_send(SocketJob::Clipboard { id, payload });
                                });
                            }
                            other => {
                                let _ = send_json(
                                    &mut socket,
                                    EnvelopeOut::new(
                                        parsed.id,
                                        "error",
                                        ErrorPayload {
                                            code: "query".into(),
                                            message: format!("unknown query {other}"),
                                        },
                                    ),
                                )
                                .await;
                            }
                        }
                    }
                    _ => {}
                }
            }
            job = jobs_rx.recv() => {
                let Some(job) = job else { continue };
                let result = match job {
                    SocketJob::Command { id, result: Ok(()) } => {
                        send_json(&mut socket, EnvelopeOut::new(id, "ack", AckPayload { ok: true })).await
                    }
                    SocketJob::Command { id, result: Err(message) } => {
                        send_json(
                            &mut socket,
                            EnvelopeOut::new(
                                id,
                                "error",
                                ErrorPayload {
                                    code: "command".into(),
                                    message,
                                },
                            ),
                        )
                        .await
                    }
                    SocketJob::Apps { id, payload } => {
                        send_json(&mut socket, EnvelopeOut::new(id, "apps", payload)).await
                    }
                    SocketJob::Clipboard { id, payload } => {
                        send_json(&mut socket, EnvelopeOut::new(id, "clipboard", payload)).await
                    }
                    SocketJob::Agents { id, payload } => {
                        send_json(&mut socket, EnvelopeOut::new(id, "agents", payload)).await
                    }
                };
                if result.is_err() {
                    break;
                }
            }
            ev = events.recv() => {
                if !trusted {
                    continue;
                }
                let result = match ev {
                    Ok(SnapshotDelta::Position) => push_state(&mut socket, &state).await,
                    Ok(SnapshotDelta::Full) => push_layout(&mut socket, &state).await,
                    Ok(SnapshotDelta::None) => Ok(()),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        push_layout(&mut socket, &state).await
                    }
                    Err(_) => continue,
                };
                if result.is_err() {
                    break;
                }
            }
            _ = evict_rx.recv() => {
                let _ = send_json(
                    &mut socket,
                    EnvelopeOut::new(
                        uuid::Uuid::new_v4().to_string(),
                        "error",
                        ErrorPayload {
                            code: "replaced".into(),
                            message: "another device connected".into(),
                        },
                    ),
                )
                .await;
                break;
            }
            _ = ping.tick() => {
                if socket.send(Message::Ping(Default::default())).await.is_err() {
                    break;
                }
                if send_json(
                    &mut socket,
                    EnvelopeOut::new(
                        uuid::Uuid::new_v4().to_string(),
                        "ping",
                        AckPayload { ok: true },
                    ),
                )
                .await
                .is_err()
                {
                    break;
                }
            }
        }
    }

    if let Some(token) = token {
        state.release_client(token);
    }
}

fn wake_snapshot(state: &AppState) {
    let refresh = state.clone();
    tokio::task::spawn_blocking(move || match refresh.try_refresh_snapshot() {
        Some(SnapshotDelta::Position) => refresh.notify_state(),
        Some(SnapshotDelta::Full) => refresh.notify(),
        _ => {}
    });
}

async fn push_layout(socket: &mut WebSocket, state: &AppState) -> Result<(), axum::Error> {
    send_json(
        socket,
        EnvelopeOut::new(uuid::Uuid::new_v4().to_string(), "layout", state.layout()),
    )
    .await?;
    push_state(socket, state).await
}

async fn push_state(socket: &mut WebSocket, state: &AppState) -> Result<(), axum::Error> {
    send_json(
        socket,
        EnvelopeOut::new(
            uuid::Uuid::new_v4().to_string(),
            "state",
            state.state_payload(),
        ),
    )
    .await
}

async fn send_json<T: serde::Serialize>(
    socket: &mut WebSocket,
    env: EnvelopeOut<T>,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(&env).unwrap_or_else(|_| "{}".into());
    socket.send(Message::Text(text.into())).await
}

#[cfg(test)]
mod tests {
    use super::cache_value;

    #[test]
    fn fingerprinted_assets_are_kept_forever() {
        let forever = Some("public, max-age=31536000, immutable");
        assert_eq!(cache_value("/assets/index-O4ho6hHU.js"), forever);
        assert_eq!(cache_value("/assets/index-DUXBBm4D.css"), forever);
    }

    #[test]
    fn the_shell_and_worker_always_revalidate() {

        for path in ["/", "/index.html", "/sw.js", "/manifest.json", "/ai"] {
            assert_eq!(cache_value(path), Some("no-cache"), "{path}");
        }
    }

    #[test]
    fn live_endpoints_keep_their_own_headers() {
        assert_eq!(cache_value("/api/artwork"), None);
        assert_eq!(cache_value("/api/icon/com.apple.Music"), None);
        assert_eq!(cache_value("/ws"), None);
    }
}
