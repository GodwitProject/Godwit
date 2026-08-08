use crate::metrics::get_metric_snapshot;
use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use std::time::Duration;

/// `/api/v1/ws/metrics` (merged under the admin `/api/v1` nest so it is
/// authenticated by `jwt_auth`). Matches the FE `websocket.ts` protocol:
/// client sends `{type:'subscribe',channel:'metrics'}` on open; the server
/// pushes unsolicited `{type:'metrics:update',data:{...camelCase...}}` frames.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/ws/metrics", get(ws_handler))
}

async fn ws_handler(ws: WebSocketUpgrade, State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

/// Build one metrics frame as a JSON string, matching the FE `MetricsUpdate` shape.
fn build_metrics_frame() -> String {
    let snap = get_metric_snapshot();
    serde_json::json!({
        "type": "metrics:update",
        "data": {
            "requestsTotal": snap.requestsTotal,
            "tokensTotal": snap.tokensTotal,
            "costUsdTotal": snap.costUsdTotal,
            "activeRequests": snap.activeRequests,
            "timestamp": snap.timestamp,
        }
    })
    .to_string()
}

async fn handle_socket(mut socket: WebSocket) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            maybe_msg = socket.recv() => {
                match maybe_msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => { /* subscribe/other messages ignored; we push unsolicited */ }
                    Some(Err(_)) => break,
                }
            }
            _ = interval.tick() => {
                let frame = build_metrics_frame();
                if socket.send(Message::Text(frame)).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_shape_matches_frontend_protocol() {
        let frame = build_metrics_frame();
        let parsed: serde_json::Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(parsed["type"], "metrics:update");
        let data = parsed.get("data").expect("frame must carry a data object");
        assert!(data.get("requestsTotal").is_some());
        assert!(data.get("tokensTotal").is_some());
        assert!(data.get("costUsdTotal").is_some());
        assert!(data.get("activeRequests").is_some());
        assert!(data.get("timestamp").is_some());
        assert!(data["timestamp"].as_str().map_or(false, |s| !s.is_empty()));
    }
}
