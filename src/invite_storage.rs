use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};

pub fn hash_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

#[derive(Clone, Debug)]
pub struct InviteRepository {
    pool: SqlitePool,
}

#[derive(Debug, FromRow, PartialEq, Eq)]
pub struct InviteRecord {
    pub id: String,
    pub token_hash: String,
    pub groups_json: String,
    pub created_at: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub consumed_by: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, FromRow, PartialEq, Eq)]
pub struct InviteEvent {
    pub id: i64,
    pub invite_id: String,
    pub event_type: String,
    pub occurred_at: String,
    pub actor: Option<String>,
    pub metadata_json: Option<String>,
}

impl InviteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations").run(&self.pool).await
    }

    pub async fn create_invite(
        &self,
        id: &str,
        token_hash: &str,
        groups_json: &str,
        created_at: &str,
        expires_at: &str,
        created_by: Option<&str>,
    ) -> Result<InviteRecord, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO invites
				(id, token_hash, groups_json, created_at, expires_at)
			 VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(token_hash)
        .bind(groups_json)
        .bind(created_at)
        .bind(expires_at)
        .execute(&mut *transaction)
        .await?;

        insert_event(
            &mut transaction,
            id,
            "created",
            created_at,
            created_by,
            None,
        )
        .await?;

        let invite = sqlx::query_as::<_, InviteRecord>(
            "SELECT id, token_hash, groups_json, created_at, expires_at,
					consumed_at, consumed_by, revoked_at
			 FROM invites WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(invite)
    }

    pub async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<InviteRecord>, sqlx::Error> {
        sqlx::query_as::<_, InviteRecord>(
            "SELECT id, token_hash, groups_json, created_at, expires_at,
					consumed_at, consumed_by, revoked_at
			 FROM invites WHERE token_hash = ?",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn consume_invite(
        &self,
        token_hash: &str,
        consumed_at: &str,
        consumed_by: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let invite = sqlx::query_as::<_, InviteRecord>(
            "SELECT id, token_hash, groups_json, created_at, expires_at,
					consumed_at, consumed_by, revoked_at
			 FROM invites
			 WHERE token_hash = ?
			   AND consumed_at IS NULL
			   AND revoked_at IS NULL
			   AND expires_at > ?",
        )
        .bind(token_hash)
        .bind(consumed_at)
        .fetch_optional(&mut *transaction)
        .await?;

        let Some(invite) = invite else {
            transaction.rollback().await?;
            return Ok(false);
        };

        sqlx::query("UPDATE invites SET consumed_at = ?, consumed_by = ? WHERE id = ?")
            .bind(consumed_at)
            .bind(consumed_by)
            .bind(&invite.id)
            .execute(&mut *transaction)
            .await?;

        insert_event(
            &mut transaction,
            &invite.id,
            "consumed",
            consumed_at,
            Some(consumed_by),
            None,
        )
        .await?;

        transaction.commit().await?;
        Ok(true)
    }

    pub async fn history(&self, invite_id: &str) -> Result<Vec<InviteEvent>, sqlx::Error> {
        sqlx::query_as::<_, InviteEvent>(
            "SELECT id, invite_id, event_type, occurred_at, actor, metadata_json
			 FROM invite_events WHERE invite_id = ? ORDER BY id",
        )
        .bind(invite_id)
        .fetch_all(&self.pool)
        .await
    }
}

async fn insert_event(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    invite_id: &str,
    event_type: &str,
    occurred_at: &str,
    actor: Option<&str>,
    metadata_json: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO invite_events
			(invite_id, event_type, occurred_at, actor, metadata_json)
		 VALUES (?, ?, ?, ?, ?)",
    )
    .bind(invite_id)
    .bind(event_type)
    .bind(occurred_at)
    .bind(actor)
    .bind(metadata_json)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::InviteRepository;

    #[tokio::test]
    async fn creates_and_consumes_an_invite_once_with_history() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory database should connect");
        let repository = InviteRepository::new(pool);
        repository.migrate().await.expect("migration should run");

        let invite = repository
            .create_invite(
                "invite-1",
                "hash-1",
                "[\"developers\"]",
                "2026-09-12T10:00:00Z",
                "2026-09-14T10:00:00Z",
                Some("admin"),
            )
            .await
            .expect("invite should be created");
        assert_eq!(invite.id, "invite-1");
        assert!(invite.consumed_at.is_none());

        assert!(
            repository
                .consume_invite("hash-1", "2026-09-12T11:00:00Z", "new-user")
                .await
                .expect("invite should be consumed")
        );
        assert!(
            !repository
                .consume_invite("hash-1", "2026-09-12T12:00:00Z", "another-user")
                .await
                .expect("second consume should be rejected")
        );

        let history = repository
            .history("invite-1")
            .await
            .expect("history should be readable");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].event_type, "created");
        assert_eq!(history[1].event_type, "consumed");
    }
}
