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
- **Simple setup**: only provide a database connection and a table name. The table needs to be defined with
  `DEFINE TABLE`.

## 🤸 Usage Example

See `examples/counter.rs`.

## SurrealDB V3

Version 3 of SurrealDB is supported since `tower-sessions-surrealdb-store` 0.8.

`SurrealSessionStore::new` now accepts either an owned `Surreal<DB>` client or an
`Arc<Surreal<DB>>` which can be used to share a single `Surreal<DB>` instance between the session store and some other
parts of an application. With the recent changes in
[clone semantics in SurrealDB 3](https://surrealdb.com/docs/languages/rust/concepts/multi-tenancy) it might be useful
to share instances in this way. For example, if authentication is done using short-lived tokens it might be useful
to have code outside the session store refresh the token, something that is not supported when passing
ownership of the `Surreal<DB>` instance to the session store.

### `surrealdb-nightly` feature

This was removed in version 0.8 with the update to SurrealDB V3, since the `surrealdb-nightly` package is no longer updated.

## Minimum Supported Rust Version (MSRV)

The MSRV is listed in the `Cargo.toml`. It is decided based on the MSRVs of this crate's dependencies.

