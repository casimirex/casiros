//! `PostgreSQL`-backed snapshot repository.
//!
//! Implements [`casiros_dag::repository::SnapshotRepository`] using a `SQLx`
//! connection pool. Snapshots are stored as JSONB documents keyed by a caller-
//! supplied identifier.

use async_trait::async_trait;
use casiros_dag::DagError;
use casiros_dag::persistence::EngineSnapshot;
use casiros_dag::repository::{SnapshotRepository, SnapshotSummary};
use sqlx::{PgPool, Row};

/// `PostgreSQL`-backed implementation of [`SnapshotRepository`].
#[derive(Debug, Clone)]
pub struct PostgresSnapshotRepository {
    /// `SQLx` connection pool.
    pool: PgPool,
}

impl PostgresSnapshotRepository {
    /// Creates a repository backed by an existing `SQLx` [`PgPool`].
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

#[cfg(test)]
mod tests {
    use super::*;
    use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
    use casiros_dag::persistence::EngineSnapshot;
    use rust_decimal_macros::dec;

    /// Connection URL used by integration tests.
    ///
    /// Tests expect a `PostgreSQL` server reachable at this URL. In CI it is
    /// provided by a service container; for local runs set `CASIROS_POSTGRES__URL`.
    fn test_db_url() -> String {
        return std::env::var("CASIROS_POSTGRES__URL")
            .unwrap_or_else(|_| "postgresql://casiros:casiros@localhost:5432/casiros".to_string());
    }

    async fn create_repo() -> Option<PostgresSnapshotRepository> {
        let url = test_db_url();
        let pool = match PgPool::connect(&url).await {
            Ok(pool) => pool,
            Err(err) => {
                eprintln!("Skipping Postgres tests: failed to connect ({err})");
                return None;
            }
        };
        let repo = PostgresSnapshotRepository::new(pool);
        if let Err(err) = repo.migrate().await {
            eprintln!("Skipping Postgres tests: migration failed ({err})");
            return None;
        }
        let _ = sqlx::query("TRUNCATE snapshots").execute(&repo.pool).await;
        return Some(repo);
    }

    fn sample_snapshot() -> EngineSnapshot {
        let mut engine = CausalityEngine::new();
        let principal = engine.add_input("principal");
        let fv = engine.add_formula(
            "fv",
            FormulaKind::FutureValue {
                present_value: Port::Output(principal),
                rate: Port::Constant(dec!(0.05)),
                periods: Port::Constant(dec!(10)),
            },
        );
        engine.add_edge(principal, fv).unwrap();
        return engine.to_snapshot();
    }

    #[tokio::test]
    async fn save_and_load_round_trip() {
        let Some(repo) = create_repo().await else {
            return;
        };
        let snapshot = sample_snapshot();
        let id = "round-trip-1";

        repo.save(id, &snapshot).await.unwrap();
        let loaded = repo.load(id).await.unwrap();
        assert_eq!(loaded.nodes.len(), snapshot.nodes.len());
    }

    #[tokio::test]
    async fn save_overwrites_existing_snapshot() {
        let Some(repo) = create_repo().await else {
            return;
        };
        let id = "overwrite-1";
        let first = sample_snapshot();
        repo.save(id, &first).await.unwrap();

        let mut second = sample_snapshot();
        second.nodes.push(casiros_dag::persistence::SnapshotNode {
            name: "extra".to_string(),
            kind: casiros_dag::persistence::SnapshotNodeKind::Input,
        });
        repo.save(id, &second).await.unwrap();

        let loaded = repo.load(id).await.unwrap();
        assert_eq!(loaded.nodes.len(), second.nodes.len());
    }

    #[tokio::test]
    async fn delete_removes_snapshot() {
        let Some(repo) = create_repo().await else {
            return;
        };
        let id = "delete-1";
        repo.save(id, &sample_snapshot()).await.unwrap();
        repo.delete(id).await.unwrap();

        assert!(repo.load(id).await.is_err());
    }

    #[tokio::test]
    async fn list_returns_saved_ids() {
        let Some(repo) = create_repo().await else {
            return;
        };
        let id_a = "list-a";
        let id_b = "list-b";
        repo.save(id_a, &sample_snapshot()).await.unwrap();
        repo.save(id_b, &sample_snapshot()).await.unwrap();

        let list = repo.list().await.unwrap();
        let ids: Vec<_> = list.into_iter().map(|s| s.id).collect();
        assert!(ids.contains(&id_a.to_string()));
        assert!(ids.contains(&id_b.to_string()));
    }

    #[tokio::test]
    async fn load_missing_snapshot_fails() {
        let Some(repo) = create_repo().await else {
            return;
        };
        assert!(repo.load("missing-id").await.is_err());
    }
}
