# `tower-sessions-surrealdb-store`

This package is in beta. It has automated tests for the basic functionality, but is untested in production.

See `examples/counter.rs` for usage.

## Using `surrealdb-nightly`

In `Config.toml`:

```toml
tower-sessions-surrealdb-store = { version = "*", features = ["surrealdb-nightly"], default-features = false }

```

The `default-features = false` is necessary, otherwise you'll install both `surrealdb` and `surrealdb-nightly` and get conflicts.
