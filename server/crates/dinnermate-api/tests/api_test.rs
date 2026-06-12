//! HTTP integration tests against the real router with Postgres-backed
//! services and the seed provider. Run via `server/scripts/test-db.sh
//! "cargo test -p dinnermate-api"`, which provides `TEST_DATABASE_URL`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, Response, StatusCode};
use axum::Router;
use dinnermate_api::server::{build_router, AppState};
use dinnermate_core::{ListService, RoomService, SeedProvider};
use dinnermate_db::{connect_and_migrate, PgListRepo, PgRoomRepo};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

async fn router() -> Router {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL is not set — run these tests via server/scripts/test-db.sh \
         \"cargo test -p dinnermate-api\", which starts a disposable postgres",
    );
    let pool = connect_and_migrate(&url)
        .await
        .expect("connect and migrate test database");
    let state = AppState {
        rooms: Arc::new(RoomService::new(
            Arc::new(PgRoomRepo::new(pool.clone())),
            Arc::new(SeedProvider::new()),
            // Task 4 replaces with PgDetailsCacheRepo (migration 0002, Task 3).
            Arc::new(dinnermate_core::testing::NoopDetailsCache),
        )),
        lists: Arc::new(ListService::new(Arc::new(PgListRepo::new(pool)))),
    };
    build_router(state, CorsLayer::permissive())
}

fn req(method: Method, uri: &str, user: Option<Uuid>, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(user) = user {
        builder = builder.header("X-Dinnermate-User", user.to_string());
    }
    match body {
        Some(value) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

async fn json_body(response: Response<Body>) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|err| panic!("body is not JSON ({err}): {:?}", String::from_utf8_lossy(&bytes)))
}

fn create_room_body() -> Value {
    json!({
        "location_label": "Salt Lake City",
        "lat": 40.760,
        "lng": -111.890,
        "radius_m": 40_000,
        "cuisines": [],
        "price_min": 1,
        "price_max": 4,
        "min_rating": 0.0,
    })
}

/// Creates a room as a fresh user and returns `(code, deck)`.
async fn create_room(app: &Router) -> (String, Vec<Value>) {
    let response = app
        .clone()
        .oneshot(req(Method::POST, "/api/v1/rooms", Some(Uuid::new_v4()), Some(create_room_body())))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = json_body(response).await;
    let code = body["room"]["code"].as_str().unwrap().to_string();
    let deck = body["deck"].as_array().unwrap().clone();
    (code, deck)
}

async fn join(app: &Router, code: &str, user: Uuid, name: &str) {
    let response = app
        .clone()
        .oneshot(req(
            Method::POST,
            &format!("/api/v1/rooms/{code}/join"),
            Some(user),
            Some(json!({"display_name": name})),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "{name} join failed");
}

async fn swipe(app: &Router, code: &str, user: Uuid, restaurant_id: &str, liked: bool) {
    let response = app
        .clone()
        .oneshot(req(
            Method::POST,
            &format!("/api/v1/rooms/{code}/swipes"),
            Some(user),
            Some(json!({"restaurant_id": restaurant_id, "liked": liked})),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(json_body(response).await, json!({}));
}

#[tokio::test]
async fn full_room_flow() {
    let app = router().await;
    let (code, deck) = create_room(&app).await;
    assert_eq!(code.len(), 6);
    assert!(!deck.is_empty() && deck.len() <= 60, "deck size {}", deck.len());

    let (user_a, user_b) = (Uuid::new_v4(), Uuid::new_v4());
    join(&app, &code, user_a, "Alice").await;
    join(&app, &code, user_b, "Bob").await;

    let first = deck[0]["id"].as_str().unwrap();
    let second = deck[1]["id"].as_str().unwrap();
    swipe(&app, &code, user_a, first, true).await;
    swipe(&app, &code, user_a, second, true).await;
    swipe(&app, &code, user_b, first, true).await;

    let response = app
        .clone()
        .oneshot(req(Method::GET, &format!("/api/v1/rooms/{code}/matches"), Some(user_a), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let matches = body["matches"].as_array().unwrap();
    let got: Vec<(&str, i64)> = matches
        .iter()
        .map(|m| (m["restaurant"]["id"].as_str().unwrap(), m["like_count"].as_i64().unwrap()))
        .collect();
    assert_eq!(got, [(first, 2), (second, 1)]);
    assert_eq!(body["participant_count"], json!(2));
}

#[tokio::test]
async fn status_code_table() {
    let app = router().await;
    let (code, deck) = create_room(&app).await;
    let joined = Uuid::new_v4();
    join(&app, &code, joined, "Alice").await;
    let first = deck[0]["id"].as_str().unwrap();
    swipe(&app, &code, joined, first, true).await;

    let cases: Vec<(&str, Request<Body>, StatusCode, &str)> = vec![
        (
            "missing user header",
            req(Method::GET, &format!("/api/v1/rooms/{code}"), None, None),
            StatusCode::BAD_REQUEST,
            "MISSING_USER",
        ),
        (
            "unknown room code",
            req(Method::GET, "/api/v1/rooms/ZZZZZZ", Some(joined), None),
            StatusCode::NOT_FOUND,
            "ROOM_NOT_FOUND",
        ),
        (
            "duplicate swipe",
            req(
                Method::POST,
                &format!("/api/v1/rooms/{code}/swipes"),
                Some(joined),
                Some(json!({"restaurant_id": first, "liked": false})),
            ),
            StatusCode::CONFLICT,
            "ALREADY_SWIPED",
        ),
        (
            "swipe without join",
            req(
                Method::POST,
                &format!("/api/v1/rooms/{code}/swipes"),
                Some(Uuid::new_v4()),
                Some(json!({"restaurant_id": first, "liked": true})),
            ),
            StatusCode::FORBIDDEN,
            "NOT_IN_ROOM",
        ),
        (
            "create room with inverted price window",
            req(
                Method::POST,
                "/api/v1/rooms",
                Some(joined),
                Some(json!({
                    "location_label": "Salt Lake City",
                    "lat": 40.760, "lng": -111.890, "radius_m": 40_000,
                    "cuisines": [], "price_min": 3, "price_max": 1, "min_rating": 0.0,
                })),
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_PARAMS",
        ),
        (
            "swipe on restaurant outside deck",
            req(
                Method::POST,
                &format!("/api/v1/rooms/{code}/swipes"),
                Some(joined),
                Some(json!({"restaurant_id": "no-such-restaurant", "liked": true})),
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
            "UNKNOWN_RESTAURANT",
        ),
        (
            "join with blank display name",
            req(
                Method::POST,
                &format!("/api/v1/rooms/{code}/join"),
                Some(Uuid::new_v4()),
                Some(json!({"display_name": "   "})),
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_PARAMS",
        ),
        (
            "malformed user header",
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/rooms/{code}"))
                .header("X-Dinnermate-User", "not-a-uuid")
                .body(Body::empty())
                .unwrap(),
            StatusCode::BAD_REQUEST,
            "MISSING_USER",
        ),
    ];

    for (name, request, want_status, want_code) in cases {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), want_status, "{name}: wrong status");
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], json!(want_code), "{name}: wrong error code, body {body}");
        assert!(
            body["error"]["message"].as_str().map(|m| !m.is_empty()).unwrap_or(false),
            "{name}: error message must be a non-empty string, body {body}"
        );
    }
}

#[tokio::test]
async fn list_flow() {
    let app = router().await;
    let (owner, other) = (Uuid::new_v4(), Uuid::new_v4());

    let response = app
        .clone()
        .oneshot(req(Method::POST, "/api/v1/lists", Some(owner), Some(json!({"name": "Date night"}))))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = json_body(response).await;
    let code = created["list"]["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 6);

    let response = app
        .clone()
        .oneshot(req(
            Method::POST,
            &format!("/api/v1/lists/{code}/items"),
            Some(other),
            Some(json!({"name": "Thai Garden", "cuisine": "thai", "source_restaurant_id": "seed-001"})),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let added = json_body(response).await;
    assert_eq!(added["item"]["name"], json!("Thai Garden"));
    assert_eq!(added["item"]["added_by_user_id"], json!(other.to_string()));

    let response = app
        .clone()
        .oneshot(req(Method::GET, &format!("/api/v1/lists/{code}"), Some(owner), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let detail = json_body(response).await;
    let items = detail["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], json!("Thai Garden"));

    let response = app
        .clone()
        .oneshot(req(Method::GET, "/api/v1/lists", Some(owner), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mine = json_body(response).await;
    let codes: Vec<&str> = mine["lists"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&code.as_str()), "owner's lists {codes:?} must contain {code}");

    let response = app
        .clone()
        .oneshot(req(Method::GET, "/api/v1/lists", Some(other), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let theirs = json_body(response).await;
    assert_eq!(theirs["lists"], json!([]), "non-owner must see an empty list array");
}

#[tokio::test]
async fn healthz_no_auth() {
    let app = router().await;
    let response = app
        .oneshot(req(Method::GET, "/healthz", None, None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"ok");
}

#[tokio::test]
async fn cors_preflight() {
    let app = router().await;
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/v1/rooms")
        .header(header::ORIGIN, "https://dinnermate.example")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert!(response.status().is_success(), "preflight status {}", response.status());
    assert!(
        response.headers().contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        "missing access-control-allow-origin, headers: {:?}",
        response.headers()
    );
}
