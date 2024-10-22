<h1 align="center">
    tower-sessions-surrealdb-store
</h1>

<p align="center">
    SurrealDB session stores for `tower-sessions`.
</p>

This package is in beta. It has automated tests for the basic functionality, but is untested in production.

## 🎨 Overview

- **Compact encoding**: session data is stored in
  the database using [MessagePack](https://crates.io/crates/rmp-serde),
  a compact self-describing serialization format.
- **Automatic table setup**: only provide a database connection and a table name;
  the table will be created if it does not exist.

## Using `surrealdb-nightly`

In `Config.toml`:

```toml
tower-sessions-surrealdb-store = { version = "*", features = ["surrealdb-nightly"], default-features = false }
```

The `default-features = false` is necessary, otherwise you'll install both `surrealdb` and `surrealdb-nightly` and get conflicts.

## 🤸 Usage Example
See `examples/counter.rs` for the full example.

```rust
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

    let session_service = ServiceBuilder::new().layer(
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
```
