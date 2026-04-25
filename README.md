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

`SurrealSessionStore::new` now accepts either an owned `Surreal<DB>` client or an
`Arc<Surreal<DB>>` which can be used to share single `Surreal<DB>` instance between the session store and some other
parts of an application. With the recent changes in
[clone semantics in SurrealDB 3](https://surrealdb.com/docs/languages/rust/concepts/multi-tenancy) it might be useful
to share instances in this way. For example, if authentication is done using short-lived tokens it might be useful
to be able to have code outside the session store refresh the token, something that is not spported when passing
owneship of the `Surreal<DB>` instance to the session store.
