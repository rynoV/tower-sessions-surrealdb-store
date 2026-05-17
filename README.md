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
