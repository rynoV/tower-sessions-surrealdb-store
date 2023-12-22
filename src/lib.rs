use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use tower_sessions_core::{
    session::{Id, Record},
    Error, ExpiredDeletion, Result, SessionStore,
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
        // For some reason surreal errors when I try to use a
        // variable `$table` or `type::table($table)` instead of
        // inserting the table name like this. It shouldn't be an
        // SQL injection risk though, because the session table
        // shouldn't be decided based on user input.
        self.client
            .query(format!(
                "insert into {} (id, data, expiry_date)
values ($id, $data, $expiry_date)
on duplicate key update data = $data, expiry_date = $expiry_date",
                self.session_table
            ))
            .bind(("id", session.id.to_string()))
            .bind(SessionRecord::from_session(session)?)
            .await
            .map_err(|e| Error::Backend(e.to_string()))?
            .check()
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
    use std::sync::Arc;

    use tower_sessions_core::{
        cookie::time::{Duration, OffsetDateTime},
        Expiry, Session,
    };

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
    async fn delete_expired() {
        let db = new_db_connection().await;
        let store = Arc::new(SurrealSessionStore::new(
            db.clone(),
            SESSIONS_TABLE.to_string(),
        ));
        let expiry = OffsetDateTime::now_utc();
        let expired = Session::new(None, store.clone(), Some(Expiry::AtDateTime(expiry)));
        let expired2 = Session::new(
            None,
            store.clone(),
            Some(Expiry::AtDateTime(
                expiry
                    .checked_sub(Duration::days(1))
                    .expect("Overflow while making expiry datetime"),
            )),
        );
        let not_expired = Session::new(None, store.clone(), Some(Expiry::OnSessionEnd));
        let not_expired2 = Session::new(None, store.clone(), None);

        for session in [&expired, &expired2, &not_expired, &not_expired2] {
            save_session(&store, &session).await;
            let expected = make_session_record(&session).await;
            let loaded = select_session(&db, &session)
                .await
                .expect("No session record");
            // We're okay to use PartialEq here, since we just want to
            // check that they're in the database
            assert_eq!(expected, loaded, "Session should be in the database");
        }

        store
            .delete_expired()
            .await
            .expect("Error deleting expired");

        for not_expired in [&not_expired, &not_expired2] {
            let loaded = select_session(&db, &not_expired)
                .await
                .expect("No session record");
            let expected = make_session_record(&not_expired).await;
            assert_eq!(
                expected, loaded,
                "Not-expired session should be in the database"
            );
            let loaded = load_session(&store, &not_expired)
                .await
                .expect("No session");
            assert_serialized_eq(
                get_record(&not_expired).await,
                loaded,
                "Not-expired session should be loaded from the store",
            );
        }

        for expired in [&expired, &expired2] {
            let loaded = select_session(&db, &expired).await;
            assert_eq!(
                None, loaded,
                "Expired session should not be in the database"
            );
            let loaded = load_session(&store, &expired).await;
            assert_serialized_eq(
                None,
                loaded,
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
        let store = Arc::new(SurrealSessionStore::new(
            db.clone(),
            SESSIONS_TABLE.to_string(),
        ));
        let session = Session::new(
            None,
            store.clone(),
            Some(Expiry::AtDateTime(OffsetDateTime::now_utc())),
        );
        session
            .insert("some key", "some value")
            .await
            .expect("Error inserting into session");
        save_session(&store, &session).await;
        let loaded = load_session(&store, &session).await;
        assert_serialized_eq(None, loaded, "Expired session should not be loaded");
    }

    #[tokio::test]
    async fn save_load_update_delete() {
        let db = new_db_connection().await;
        let store = Arc::new(SurrealSessionStore::new(
            db.clone(),
            SESSIONS_TABLE.to_string(),
        ));
        let session = Session::new(
            None,
            store.clone(),
            Some(Expiry::OnInactivity(Duration::hours(1))),
        );

        // | Initial save and load |
        session
            .insert("some key", "some value")
            .await
            .expect("Error inserting into session");

        save_session(&store, &session).await;

        let record = select_session(&db, &session)
            .await
            .expect("No session record found in DB");

        let expected = make_session_record(&session).await;
        assert_eq!(expected, record, "Record in database");

        let loaded = load_session(&store, &session).await.expect("No session");
        // To check that the session is correct we compare the
        // serialized value, because the Session PartialEq
        // implementation only compares the session ids
        assert_serialized_eq(
            get_record(&session).await,
            loaded,
            "Loaded session",
        );

        // | Update |

        // Simulate updating the session by making a new session with
        // the same id
        session
            .insert("some new key", "some new value")
            .await
            .expect("Error inserting into session");

        save_session(&store, &session).await;

        let record = select_session(&db, &session)
            .await
            .expect("No session record found in DB");

        let expected = make_session_record(&session).await;
        assert_eq!(expected, record, "Record in database after update");

        let loaded = load_session(&store, &session).await.expect("No session");
        assert_serialized_eq(
            get_record(&session).await,
            loaded,
            "Loaded session after update",
        );

        // | Delete |
        store
            .delete(&get_record(&session).await.id)
            .await
            .expect("Error deleting session");

        let record = select_session(&db, &session).await;
        assert_eq!(None, record, "Deleted session record in database");

        let loaded = load_session(&store, &session).await;
        assert_serialized_eq(None, loaded, "Deleted session");
    }

    async fn get_record(session: &Session) -> Record {
        session
            .get_record()
            .await
            .expect("Error getting session record")
            .as_ref()
            .expect("Session record was None")
            .to_owned()
    }

    async fn make_session_record(session: &Session) -> SessionRecord {
        SessionRecord::from_session(&get_record(session).await).expect("Error encoding session")
    }

    async fn save_session(store: &SurrealSessionStore<DB>, session: &Session) {
        store
            .save(&get_record(session).await)
            .await
            .expect("Error saving session")
    }

    async fn load_session(store: &SurrealSessionStore<DB>, session: &Session) -> Option<Record> {
        store
            .load(&get_record(session).await.id)
            .await
            .expect("Error loading session")
    }

    async fn select_session(db: &Surreal<DB>, session: &Session) -> Option<SessionRecord> {
        db.select((SESSIONS_TABLE, get_record(session).await.id.to_string()))
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
