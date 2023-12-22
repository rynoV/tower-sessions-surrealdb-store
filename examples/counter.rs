//! Example demonstrating use of `SurrealSessionStore` with an
//! in-memory backend and continuous expired session deletion with 10
//! second cookie lifetime. Go to the URL shown in the console in your
//! browser, the counter should increment on each page load, and after
//! 10 seconds of inactivity the counter should reset.
use std::net::SocketAddr;

use axum::http::StatusCode;
use axum::{
    error_handling::HandleErrorLayer, response::IntoResponse, routing::get, BoxError, Router,
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_sessions::{cookie::time::Duration, Expiry, Session, SessionManagerLayer};
use tower_sessions_core::ExpiredDeletion as _;
use tower_sessions_surrealdb_store::SurrealSessionStore;

const COUNTER_KEY: &str = "counter";

#[derive(Default, Deserialize, Serialize)]
struct Counter(usize);

#[tokio::main]
async fn main() {
    let db = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(())
        .await
        .expect("Surreal initialization failure");
    db.use_ns("testing")
        .await
        .expect("Surreal namespace initialization failure");
    db.use_db("testing")
        .await
        .expect("Surreal database initialization failure");

    // This sets up the store to use the `sessions` table.
    let session_store = SurrealSessionStore::new(db.clone(), "sessions".to_string());
    let expired_session_cleanup_interval: u64 = 1;
    tokio::task::spawn(session_store.clone().continuously_delete_expired(
        tokio::time::Duration::from_secs(60 * expired_session_cleanup_interval),
    ));

    let session_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::BAD_REQUEST
        }))
        .layer(
            SessionManagerLayer::new(session_store)
                .with_secure(false)
                .with_expiry(Expiry::OnInactivity(Duration::seconds(10))),
        );

    let app = Router::new()
        .route("/", get(handler))
        .layer(session_service);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn handler(session: Session) -> impl IntoResponse {
    let counter: Counter = session.get(COUNTER_KEY).await.unwrap().unwrap_or_default();
    session.insert(COUNTER_KEY, counter.0 + 1).await.unwrap();
    format!("Current count: {}", counter.0)
}
