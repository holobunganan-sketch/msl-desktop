//! Historical database fixtures. Never compiled into the application.
use super::{migrations, Connection, Database};

pub(crate) fn v29_database(conn: Connection) -> Database {
    let db = Database { conn };
    db.configure().unwrap();
    db.conn.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at INTEGER NOT NULL);").unwrap();
    for migration in migrations::MIGRATIONS
        .iter()
        .filter(|entry| entry.version <= 29)
    {
        db.conn.execute_batch(migration.sql).unwrap();
        db.conn
            .execute(
                "INSERT INTO schema_migrations VALUES(?1,?2,0)",
                rusqlite::params![migration.version, migration.name],
            )
            .unwrap();
    }
    db
}
