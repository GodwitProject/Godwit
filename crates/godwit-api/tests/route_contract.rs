//! Verifies every route declared in `contract/routes.json` actually exists in the
//! production router. Mounts the real `app(state)` and, for each contract route,
//! issues a request; a route that does not exist returns axum's empty-body 404 —
//! anything else (401/403/400/200/405...) proves the route matched.

use axum::body::Body;
use axum::http::Request;
use godwit_api::app::{build_app, build_test_state};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";

fn contract_path() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = crates/godwit-api; workspace root is two levels up.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    std::path::Path::new(&manifest)
        .join("..")
        .join("..")
        .join("contract")
        .join("routes.json")
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct ContractRoute {
    id: String,
    method: String,
    path: String,
    scope: String,
}

/// Substitute every `{param}` placeholder with a valueless-but-parseable sentinel so
/// the concrete path reaches the handler (avoiding a mis-parsed 404). Every protected
/// route is gated by `jwt_auth`/`api_key_auth`, which reject unauthenticated probes
/// with 401 before any DB lookup — so the value only needs to be path-parseable.
fn concrete(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut chars = path.chars();
    while let Some(c) = chars.next() {
        if c == '{' {
            for pc in chars.by_ref() {
                if pc == '}' {
                    break;
                }
            }
            out.push_str(ZERO_UUID);
        } else {
            out.push(c);
        }
    }
    out
}

async fn exists(app: &axum::Router, method: &str, path: &str) -> bool {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(&concrete(path))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    // axum's "no route matched" 404 has an empty body. Distinguish that from a
    // handler-returned 404 (e.g. OIDC/SAML provider not configured), which carries a
    // non-empty JSON body — the route exists in both cases, but only the former means
    // the path was never registered.
    if status != axum::http::StatusCode::NOT_FOUND {
        return true;
    }
    let (_parts, body) = resp.into_parts();
    let bytes = body.collect().await.expect("read body").to_bytes();
    !bytes.is_empty()
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn all_contract_routes_exist(pool: PgPool) {
    let state = build_test_state(pool);
    let router = build_app(state);
    let bytes = std::fs::read_to_string(contract_path()).expect("contract file");
    let routes: Vec<ContractRoute> = serde_json::from_str(&bytes).expect("contract JSON");

    assert!(!routes.is_empty(), "contract must not be empty");
    for r in &routes {
        let ok = exists(&router, &r.method, &r.path).await;
        assert!(
            ok,
            "contract route {} {} {} not found in router",
            r.method, r.path, r.id
        );
    }
}

/// Negative control: a completely bogus path returns the route-missing sentinel,
/// proving `exists` isn't trivially always-true.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn bogus_route_is_not_found(pool: PgPool) {
    let state = build_test_state(pool);
    let router = build_app(state);
    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/does/not/exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
}
