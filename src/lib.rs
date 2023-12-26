use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use tower_sessions_core::{
    session::{Id, Record},
    session_store::{Error, Result},
    ExpiredDeletion, SessionStore,
};
use tracing::info;

/// Representation of a session in the database.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SessionRecord {
    data: Vec<u8>,
    expiry_date: i64,
}

impl SessionRecord {
    fn from_session(session: &Record) -> Result<Self> {
        Ok(SessionRecord {
            data: rmp_serde::to_vec(session).map_err(|e| Error::Decode(e.to_string()))?,
            expiry_date: session.expiry_date.unix_timestamp(),
        })
    }

    fn to_session(&self) -> Result<Record> {
        let session: Record =
            rmp_serde::from_slice(&self.data).map_err(|e| Error::Decode(e.to_string()))?;
        Ok(session)
    }
}

/// A SurrealDB session store.
#[derive(Debug, Clone)]
pub struct SurrealSessionStore<DB: std::fmt::Debug + surrealdb::Connection> {
    client: Surreal<DB>,
    session_table: String,
}

impl<DB: std::fmt::Debug + surrealdb::Connection> SurrealSessionStore<DB> {
    /// Create a new SurrealDB session store with the provided client,
    /// storing sessions in the given table. Note that the table must
    /// be defined ahead of time if strict mode is enabled.
    pub fn new(client: Surreal<DB>, session_table: String) -> Self {
        Self {
            client,
            session_table,
        }
    }
}

#[async_trait]
impl<DB: std::fmt::Debug + surrealdb::Connection> ExpiredDeletion for SurrealSessionStore<DB> {
    async fn delete_expired(&self) -> Result<()> {
        info!("Deleting expired sessions");
        self.client
            .query(format!(
                "delete type::table($table) where expiry_date <= time::unix(time::now())"
            ))
            .bind(("table", self.session_table.clone()))
            .await
            .map_err(|e| Error::Backend(e.to_string()))?
            .check()
            .map_err(|e| Error::Backend(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl<DB: std::fmt::Debug + surrealdb::Connection> SessionStore for SurrealSessionStore<DB> {
    async fn save(&self, session: &Record) -> Result<()> {
        let _: Option<SessionRecord> = self
            .client
            .update((self.session_table.clone(), session.id.to_string()))
            .content(SessionRecord::from_session(session)?)
            .await
            .map_err(|e| Error::Backend(e.to_string()))?;

        Ok(())
    }

    async fn load(&self, session_id: &Id) -> Result<Option<Record>> {
        let record: Option<SessionRecord> = self
            .client
            .query(
                "select expiry_date, data from type::thing($table, $id)
where expiry_date > time::unix(time::now())",
            )
            .bind(("id", session_id.to_string()))
            .bind(("table", self.session_table.clone()))
            .await
            .map_err(|e| Error::Backend(e.to_string()))?
            .take(0)
            .map_err(|e| Error::Backend(e.to_string()))?;
        record.map(|r| r.to_session()).transpose()
    }

    async fn delete(&self, session_id: &Id) -> Result<()> {
        self.client
            .delete::<Option<SessionRecord>>((&self.session_table, &session_id.to_string()))
            .await
            .map_err(|e| Error::Backend(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use tower_sessions_core::cookie::time::{Duration, OffsetDateTime};

    use super::*;

    static SESSIONS_TABLE: &str = "sessions";

    type DB = surrealdb::engine::local::Db;

    async fn new_db_connection() -> Surreal<DB> {
        let db = Surreal::new::<surrealdb::engine::local::Mem>(())
            .await
            .expect("Surreal initialization failure");
        db.use_ns("testing")
            .await
            .expect("Surreal namespace initialization failure");
        db.use_db("testing")
            .await
            .expect("Surreal database initialization failure");
        db
    }

    #[tokio::test]
    async fn basic_roundtrip() {
        let db = new_db_connection().await;
        let store = SurrealSessionStore::new(db.clone(), SESSIONS_TABLE.to_string());
        let record = make_record(None, [("key", "value")].to_vec(), Duration::days(1));
        store.save(&record).await.expect("Error saving");
        let loaded = store
            .load(&record.id)
            .await
            .expect("Error loading")
            .expect("Value missing");
        assert_eq!(record, loaded, "Loaded value should equal original");
    }

    #[tokio::test]
    async fn delete_expired() {
        let db = new_db_connection().await;
        let store = SurrealSessionStore::new(db.clone(), SESSIONS_TABLE.to_string());
        let expired = make_record(None, [].to_vec(), Duration::ZERO);
        let expired2 = make_record(None, [("key", "value")].to_vec(), Duration::days(-1));
        let not_expired = make_record(None, [].to_vec(), Duration::days(1));
        let not_expired2 = make_record(None, [("key", "value")].to_vec(), Duration::minutes(1));

        for session in [&expired, &expired2, &not_expired, &not_expired2] {
            save_session(&store, &session).await;
            select_session(&db, &session)
                .await
                .expect("Session should be in the database");
        }

        store
            .delete_expired()
            .await
            .expect("Error deleting expired");

        for not_expired in [&not_expired, &not_expired2] {
            select_session(&db, &not_expired)
                .await
                .expect("Not-expired session should be in the database");

            let loaded = load_session(&store, &not_expired)
                .await
                .expect("No session loaded");

            assert_eq!(
                not_expired, &loaded,
                "Not-expired session should be loaded from the store",
            );
        }

        for expired in [&expired, &expired2] {
            let loaded = select_session(&db, &expired).await;
            assert!(
                loaded.is_none(),
                "Expired session should not be in the database"
            );

            let loaded = load_session(&store, &expired).await;

            assert!(
                loaded.is_none(),
                "Expired session should not be loaded from the store",
            );
        }
    }

    #[tokio::test]
    async fn load_non_existent() {
        let db = new_db_connection().await;
        let session_store = SurrealSessionStore::new(db, SESSIONS_TABLE.to_string());
        let loaded = session_store
            .load(&Id::default())
            .await
            .expect("Error loading session");
        assert_serialized_eq(None, loaded, "Non existent session should not be loaded");
    }

    #[tokio::test]
    async fn load_expired() {
        let db = new_db_connection().await;
        let store = SurrealSessionStore::new(db.clone(), SESSIONS_TABLE.to_string());
        let session = make_record(None, [("some key", "some value")].to_vec(), Duration::ZERO);
        save_session(&store, &session).await;
        let loaded = load_session(&store, &session).await;
        assert_serialized_eq(None, loaded, "Expired session should not be loaded");
    }

    #[tokio::test]
    async fn save_load_update_delete() {
        let db = new_db_connection().await;
        let store = SurrealSessionStore::new(db.clone(), SESSIONS_TABLE.to_string());
        let session = make_record(
            None,
            [("some key", "some value")].to_vec(),
            Duration::hours(1),
        );

        // | Initial save and load |
        save_session(&store, &session).await;

        let record = select_session(&db, &session)
            .await
            .expect("No session record found in DB");

        let expected = make_session_record(&session).await;
        assert_eq!(expected, record, "Record in database");

        let loaded = load_session(&store, &session).await.expect("No session");
        assert_eq!(session, loaded, "Loaded session");

        // | Update |
        let mut new_data = session.data.clone();
        new_data.insert("some new key".to_string(), to_value("some new value"));
        let session = Record {
            data: new_data,
            ..session
        };

        save_session(&store, &session).await;

        let record = select_session(&db, &session)
            .await
            .expect("No session record found in DB");

        let expected = make_session_record(&session).await;
        assert_eq!(expected, record, "Record in database after update");

        let loaded = load_session(&store, &session).await.expect("No session");
        assert_eq!(session, loaded, "Loaded session after update",);

        // | Delete |
        store
            .delete(&session.id)
            .await
            .expect("Error deleting session");

        let record = select_session(&db, &session).await;
        assert!(record.is_none(), "Deleted session record in database");

        let loaded = load_session(&store, &session).await;
        assert!(loaded.is_none(), "Deleted session");
    }

    fn make_record(id: Option<Id>, values: Vec<(&str, &str)>, date_offset: Duration) -> Record {
        Record {
            id: id.unwrap_or_default(),
            data: HashMap::from_iter(values.iter().map(|(k, v)| (k.to_string(), to_value(v)))),
            expiry_date: OffsetDateTime::now_utc()
                .checked_add(date_offset)
                .expect("Overflow making expiry"),
        }
    }

    fn to_value(v: &str) -> serde_json::Value {
        serde_json::to_value(v).expect("Error encoding")
    }

    async fn make_session_record(session: &Record) -> SessionRecord {
        SessionRecord::from_session(&session).expect("Error deserializing")
    }

    async fn save_session(store: &SurrealSessionStore<DB>, session: &Record) {
        store.save(session).await.expect("Error saving session")
    }

    async fn load_session(store: &SurrealSessionStore<DB>, session: &Record) -> Option<Record> {
        store
            .load(&session.id)
            .await
            .expect("Error loading session")
    }

    async fn select_session(db: &Surreal<DB>, session: &Record) -> Option<SessionRecord> {
        db.select((SESSIONS_TABLE, session.id.to_string()))
            .await
            .expect("Error retrieving session record")
    }

    fn assert_serialized_eq<T>(v1: T, v2: T, msg: &str)
    where
        T: Serialize,
    {
        assert_eq!(
            serde_json::to_value(v1).expect("Serialization of v1 failed"),
            serde_json::to_value(v2).expect("Serialization of v2 failed"),
            "{}",
            msg
        );
    }
}
