<h1 align="center">
    tower-sessions-surrealdb-store
</h1>

<p align="center">
    SurrealDB session stores for <code>tower-sessions</code>.
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
See `examples/counter.rs`.

