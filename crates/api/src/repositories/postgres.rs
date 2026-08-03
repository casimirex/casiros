//! PostgreSQL-backed snapshot repository.
//!
//! Implements [`casiros_dag::repository::SnapshotRepository`] using a `SQLx`
//! connection pool. Snapshots are stored as JSONB documents keyed by a caller-
//! supplied identifier.

use async_trait::async_trait;
use casiros_dag::DagError;
use casiros_dag::persistence::EngineSnapshot;
use casiros_dag::repository::{SnapshotRepository, SnapshotSummary};
use sqlx::{PgPool, Row};

/// `PostgreSQL` implementation of [`SnapshotRepository`].
#[derive(Debug, Clone)]
pub struct PostgresSnapshotRepository {
    /// `SQLx` connection pool.
    pool: PgPool,
}

impl PostgresSnapshotRepository {
    /// Creates a repository backed by an existing `SQLx` `PostgreSQL` pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        return Self { pool };
    }

    /// Runs pending `SQLx` migrations against the pool.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if migration fails.
    pub async fn migrate(&self) -> Result<(), DagError> {
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|err| DagError::Repository {
                message: format!("migration failed: {err}"),
            })?;
        return Ok(());
    }
}

#[async_trait]
impl SnapshotRepository for PostgresSnapshotRepository {
    async fn save(&self, id: &str, snapshot: &EngineSnapshot) -> Result<(), DagError> {
        let data = serde_json::to_value(snapshot).map_err(|err| DagError::Repository {
            message: format!("failed to serialize snapshot: {err}"),
        })?;

        sqlx::query(
            "INSERT INTO snapshots (id, data) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET data = $2",
        )
        .bind(id)
        .bind(data)
        .execute(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("postgres save failed: {err}"),
        })?;

        return Ok(());
    }

    async fn load(&self, id: &str) -> Result<EngineSnapshot, DagError> {
        let row: (serde_json::Value,) = sqlx::query_as("SELECT data FROM snapshots WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| DagError::Repository {
                message: format!("postgres load failed: {err}"),
            })?;

        return serde_json::from_value(row.0).map_err(|err| DagError::Repository {
            message: format!("failed to deserialize snapshot: {err}"),
        });
    }

    async fn delete(&self, id: &str) -> Result<(), DagError> {
        let result = sqlx::query("DELETE FROM snapshots WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|err| DagError::Repository {
                message: format!("postgres delete failed: {err}"),
            })?;

        if result.rows_affected() == 0 {
            return Err(DagError::Repository {
                message: format!("snapshot '{id}' not found"),
            });
        }

        return Ok(());
    }

    async fn list(&self) -> Result<Vec<SnapshotSummary>, DagError> {
        let rows = sqlx::query("SELECT id FROM snapshots ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|err| DagError::Repository {
                message: format!("postgres list failed: {err}"),
            })?;

        return Ok(rows
            .into_iter()
            .map(|row| SnapshotSummary {
                id: row.get("id"),
                name: None,
            })
            .collect());
    }
}
