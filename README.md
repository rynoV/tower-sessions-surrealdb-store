# `tower-sessions-surrealdb-store`

This package is in beta. It has automated tests for the basic functionality, but is untested in production.

See `examples/counter.rs` for usage.

## Using `surrealdb-nightly`

In `Config.toml`:

```toml
tower-sessions-surrealdb-store = { version = "*", features = ["surrealdb-nightly"], default-features = false }

```

The `default-features = false` is necessary, otherwise you'll install both `surrealdb` and `surrealdb-nightly` and get conflicts.

## Security

If for some reason your session table name is based on end-user input, you may be vulnerable to SQL injection with this package (but I can't see why that would be the case). This is due to SurrealDB being weird about the table name in `INSERT` statements; if anyone knows how to deal with this please let me know.
