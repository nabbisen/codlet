//! Full conformance suite for the SQLite adapter (RFC-023, RFC-022).
//!
//! Each test function wires up a fresh SQLite in-memory pool, runs migrations,
//! and passes the pool to the `codlet-conformance` parameterised runner. The
//! concurrent-claim race test verifies the single-winner guarantee under real
//! async task concurrency (RFC-022). The admin tests verify RFC-030.

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use codlet::admin::{CodeAdminStore, CodeListFilter};
    use codlet::secret::{CodeId, ScopeKey};
    use codlet::store::code::{ClaimRequest, CodeStore};
    use codlet_conformance::fixtures::{LATER, NOW, code_lk, code_record};
    use codlet_sqlx::{SqliteStore, run_migrations};

    async fn fresh_store() -> SqliteStore {
        // Each test gets its own in-memory database. We use a single-connection
        // pool so all queries share the same SQLite in-memory database — multiple
        // connections would each get an independent (empty) database.
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        SqliteStore::new(pool)
    }

    // ── Code store ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn sqlite_code_store_conformance() {
        codlet_conformance::run_code_store_conformance(fresh_store).await;
        codlet_conformance::run_code_store_concurrency_conformance(fresh_store).await;
    }

    /// RFC-048 regression: `claim_code`'s `scope` parameter must be bound,
    /// not interpolated. The literal payload from the finding -- not a
    /// sanitised approximation -- must neither claim the targeted code nor
    /// touch any other row.
    #[tokio::test]
    async fn claim_code_scope_is_bound_not_interpolated() {
        const PAYLOAD: &str = "x\" OR 1=1 --";

        let store = fresh_store().await;
        store
            .insert_code(code_record("victim", "victimsec", LATER, Some("tenant-A")))
            .await
            .unwrap();
        store
            .insert_code(code_record("other", "othersec", LATER, Some("tenant-B")))
            .await
            .unwrap();

        let outcome = store
            .claim_code(&ClaimRequest {
                code_id: &CodeId::new("victim".into()),
                subject: &codlet::secret::SubjectId::new("attacker".into()),
                now: NOW,
                purpose: None,
                scope: Some(PAYLOAD),
            })
            .await
            .unwrap();
        assert_eq!(
            outcome,
            codlet::state::ClaimOutcome::Lost,
            "a scope that matches no real row must not win the claim"
        );

        // Neither code was touched: both must still be redeemable.
        let victim = store
            .lookup_code(&[code_lk("victimsec")], None)
            .await
            .unwrap()
            .expect("victim code must still exist");
        assert!(
            victim.used_at.is_none(),
            "victim code must not have been marked used"
        );
        let other = store
            .lookup_code(&[code_lk("othersec")], None)
            .await
            .unwrap()
            .expect("other code must still exist");
        assert!(
            other.used_at.is_none(),
            "an unrelated row in a different scope must not have been touched"
        );
    }

    // ── Session store ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn sqlite_session_store_conformance() {
        codlet_conformance::run_session_store_conformance(fresh_store).await;
    }

    // ── Form-token store ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn sqlite_form_token_store_conformance() {
        codlet_conformance::run_form_token_store_conformance(fresh_store).await;
    }

    // ── Migration smoke tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        // Running twice must not error (IF NOT EXISTS semantics).
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn schema_includes_key_version_columns() {
        // RFC-011 §10.5: "Schema includes key version columns from first migration."
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();

        for table in ["codlet_codes", "codlet_sessions", "codlet_form_tokens"] {
            let result: Result<Vec<(String,)>, _> = sqlx::query_as(sqlx::AssertSqlSafe(
                format!("SELECT key_version FROM {table} LIMIT 0").as_str(),
            ))
            .fetch_all(&pool)
            .await;
            assert!(
                result.is_ok(),
                "table {table} must have a key_version column"
            );
        }
    }

    // ── CodeAdminStore tests (RFC-030) ────────────────────────────────────────────

    #[tokio::test]
    async fn admin_list_codes_all() {
        let store = fresh_store().await;
        store
            .insert_code(code_record("c1", "sec1", LATER, None))
            .await
            .unwrap();
        store
            .insert_code(code_record("c2", "sec2", LATER, Some("scope-A")))
            .await
            .unwrap();

        let rows = store.list_codes(&CodeListFilter::all(), NOW).await.unwrap();
        assert_eq!(rows.len(), 2, "all filter must return both codes");
        for row in &rows {
            let dbg = format!("{row:?}");
            assert!(
                !dbg.contains("lookup_key"),
                "admin meta must not expose lookup_key"
            );
        }
    }

    #[tokio::test]
    async fn admin_list_codes_active_only() {
        let store = fresh_store().await;
        store
            .insert_code(code_record("ca1", "seca1", LATER, None))
            .await
            .unwrap();
        store
            .insert_code(code_record("ca2", "seca2", LATER, None))
            .await
            .unwrap();
        // Claim one.
        let found = store
            .lookup_code(&[code_lk("seca1")], None)
            .await
            .unwrap()
            .unwrap();
        store
            .claim_code(&ClaimRequest {
                code_id: &found.id,
                subject: &codlet::secret::SubjectId::new("u1".into()),
                now: NOW,
                purpose: None,
                scope: None,
            })
            .await
            .unwrap();

        let active = store
            .list_codes(
                &CodeListFilter {
                    active_only: true,
                    ..Default::default()
                },
                NOW,
            )
            .await
            .unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, CodeId::new("ca2".into()));
    }

    #[tokio::test]
    async fn admin_list_codes_scoped() {
        let store = fresh_store().await;
        store
            .insert_code(code_record("cs1", "secs1", LATER, Some("scope-X")))
            .await
            .unwrap();
        store
            .insert_code(code_record("cs2", "secs2", LATER, Some("scope-Y")))
            .await
            .unwrap();
        store
            .insert_code(code_record("cs3", "secs3", LATER, None))
            .await
            .unwrap();

        let scoped = store
            .list_codes(
                &CodeListFilter::active_in_scope(ScopeKey::new("scope-X")),
                NOW,
            )
            .await
            .unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].id, CodeId::new("cs1".into()));
        assert_eq!(scoped[0].scope.as_deref(), Some("scope-X"));
    }

    #[tokio::test]
    async fn admin_list_codes_limit() {
        let store = fresh_store().await;
        for i in 0..5u8 {
            store
                .insert_code(code_record(
                    &format!("cl{i}"),
                    &format!("secl{i}"),
                    LATER,
                    None,
                ))
                .await
                .unwrap();
        }
        let limited = store
            .list_codes(
                &CodeListFilter {
                    limit: Some(2),
                    ..Default::default()
                },
                NOW,
            )
            .await
            .unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[tokio::test]
    async fn admin_get_code_meta_found() {
        let store = fresh_store().await;
        store
            .insert_code(code_record("cm1", "secm1", LATER, Some("scope-Z")))
            .await
            .unwrap();

        let meta = store
            .get_code_meta(&CodeId::new("cm1".into()))
            .await
            .unwrap()
            .expect("must find the inserted code");
        assert_eq!(meta.id, CodeId::new("cm1".into()));
        assert_eq!(meta.scope.as_deref(), Some("scope-Z"));
        assert_eq!(meta.grant.as_deref(), Some("grant-cm1"));
        assert!(meta.used_at.is_none());
        assert!(meta.revoked_at.is_none());
        assert!(meta.created_at.is_some(), "created_at must be stored");
    }

    #[tokio::test]
    async fn admin_get_code_meta_not_found() {
        let store = fresh_store().await;
        let meta = store
            .get_code_meta(&CodeId::new("ghost".into()))
            .await
            .unwrap();
        assert!(meta.is_none());
    }

    #[tokio::test]
    async fn admin_meta_shows_used_state_after_claim() {
        let store = fresh_store().await;
        store
            .insert_code(code_record("cu1", "secu1", LATER, None))
            .await
            .unwrap();
        let found = store
            .lookup_code(&[code_lk("secu1")], None)
            .await
            .unwrap()
            .unwrap();
        store
            .claim_code(&ClaimRequest {
                code_id: &found.id,
                subject: &codlet::secret::SubjectId::new("alice".into()),
                now: NOW,
                purpose: None,
                scope: None,
            })
            .await
            .unwrap();

        let meta = store
            .get_code_meta(&CodeId::new("cu1".into()))
            .await
            .unwrap()
            .unwrap();
        assert!(meta.used_at.is_some(), "used_at must be set after claim");
        assert_eq!(meta.used_by.as_ref().map(|s| s.as_str()), Some("alice"));
        assert!(!meta.is_redeemable_at(NOW));
    }

    #[tokio::test]
    async fn admin_meta_contains_no_lookup_key() {
        // RFC-030 acceptance: "Listing APIs cannot return plaintext secrets."
        let store = fresh_store().await;
        store
            .insert_code(code_record("cn1", "topsecretsecn1", LATER, None))
            .await
            .unwrap();

        let meta = store
            .get_code_meta(&CodeId::new("cn1".into()))
            .await
            .unwrap()
            .unwrap();
        let dbg = format!("{meta:?}");
        for forbidden in ["lookup_key", "topsecretsecn1", "hmac"] {
            assert!(
                !dbg.contains(forbidden),
                "CodeMeta debug must not contain {forbidden:?}: {dbg}"
            );
        }
    }
}

// ── PostgreSQL conformance tests (RFC-034) ─────────────────────────────────
//
// Require: --features postgres-test AND Docker available.
// Run:     cargo test -p codlet-sqlx --features postgres-test
// CI:      see .github/workflows/ci.yml  test-postgres job

#[cfg(feature = "postgres-test")]
mod postgres_tests {
    use codlet::admin::{CodeAdminStore, CodeListFilter};
    use codlet::secret::{CodeId, ScopeKey};
    use codlet::store::code::{ClaimRequest, CodeStore};
    use codlet_conformance::fixtures::{LATER, NOW, code_lk, code_record};
    use codlet_sqlx::{PostgresStore, run_postgres_migrations};
    use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};

    async fn fresh_pg_store() -> PostgresStore {
        // Spin up a real PostgreSQL container for each test group.
        // testcontainers drops the container when the returned handle is dropped.
        let container = Postgres::default().start().await.unwrap();
        let url = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            container.get_host_port_ipv4(5432).await.unwrap()
        );
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        run_postgres_migrations(&pool).await.unwrap();
        // Leak the container handle so it lives for the duration of the test.
        std::mem::forget(container);
        PostgresStore::new(pool)
    }

    // ── Conformance suite ─────────────────────────────────────────────────

    #[tokio::test]
    async fn postgres_code_store_conformance() {
        codlet_conformance::run_code_store_conformance(fresh_pg_store).await;
        codlet_conformance::run_code_store_concurrency_conformance(fresh_pg_store).await;
    }

    /// RFC-048 regression: `claim_code`'s `scope` parameter must be bound,
    /// not interpolated. The literal payload from the finding -- not a
    /// sanitised approximation -- must neither claim the targeted code nor
    /// touch any other row. PostgreSQL's `"…"` quoting semantics differ from
    /// SQLite's, but binding removes that distinction entirely: the payload
    /// is compared as an opaque string value regardless of its content.
    #[tokio::test]
    async fn claim_code_scope_is_bound_not_interpolated() {
        const PAYLOAD: &str = "x\" OR 1=1 --";

        let store = fresh_pg_store().await;
        store
            .insert_code(code_record("victim", "victimsec", LATER, Some("tenant-A")))
            .await
            .unwrap();
        store
            .insert_code(code_record("other", "othersec", LATER, Some("tenant-B")))
            .await
            .unwrap();

        let outcome = store
            .claim_code(&ClaimRequest {
                code_id: &CodeId::new("victim".into()),
                subject: &codlet::secret::SubjectId::new("attacker".into()),
                now: NOW,
                purpose: None,
                scope: Some(PAYLOAD),
            })
            .await
            .unwrap();
        assert_eq!(
            outcome,
            codlet::state::ClaimOutcome::Lost,
            "a scope that matches no real row must not win the claim"
        );

        let victim = store
            .lookup_code(&[code_lk("victimsec")], None)
            .await
            .unwrap()
            .expect("victim code must still exist");
        assert!(
            victim.used_at.is_none(),
            "victim code must not have been marked used"
        );
        let other = store
            .lookup_code(&[code_lk("othersec")], None)
            .await
            .unwrap()
            .expect("other code must still exist");
        assert!(
            other.used_at.is_none(),
            "an unrelated row in a different scope must not have been touched"
        );
    }

    #[tokio::test]
    async fn postgres_session_store_conformance() {
        codlet_conformance::run_session_store_conformance(fresh_pg_store).await;
    }

    /// Column names of `codlet_sessions`, as PostgreSQL reports them.
    async fn session_columns(pool: &sqlx::PgPool) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT column_name FROM information_schema.columns
             WHERE table_name = 'codlet_sessions'",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    /// A PostgreSQL container with a connected pool; the schema is not applied.
    async fn bare_pg_pool() -> sqlx::PgPool {
        let container = Postgres::default().start().await.unwrap();
        let url = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            container.get_host_port_ipv4(5432).await.unwrap()
        );
        std::mem::forget(container);
        sqlx::PgPool::connect(&url).await.unwrap()
    }

    /// RFC-057 follow-up: PostgreSQL's `last_seen_at` upgrade rests on
    /// `ADD COLUMN IF NOT EXISTS`, which is a server guarantee — and an untested
    /// migration path is exactly how RFC-038 and RFC-057 happened. Start from a
    /// `codlet_sessions` table without the column, prove it is absent, run the
    /// shipped runner, prove it is present, and keep the existing row's NULL.
    #[tokio::test]
    async fn postgres_last_seen_at_is_added_to_a_pre_rfc_044_sessions_table() {
        let pool = bare_pg_pool().await;
        run_postgres_migrations(&pool).await.unwrap();

        // Take the shipped table back to its pre-RFC-044 shape, holding a row a
        // real deployment would already have.
        sqlx::query("ALTER TABLE codlet_sessions DROP COLUMN last_seen_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO codlet_sessions (id, lookup_key, key_version, subject, created_at, expires_at)
             VALUES ('s1', 'lk1', 'v1', 'user-1', 1000, 2000)",
        )
        .execute(&pool)
        .await
        .unwrap();
        // The setup must have worked, or the assertion below proves nothing.
        assert!(
            !session_columns(&pool)
                .await
                .contains(&"last_seen_at".to_string()),
            "setup: last_seen_at must be absent before the runner is called"
        );

        run_postgres_migrations(&pool).await.unwrap();

        assert!(
            session_columns(&pool)
                .await
                .contains(&"last_seen_at".to_string()),
            "last_seen_at was not added"
        );
        // Additive, no backfill (RFC-044 §5): the old row reads back NULL.
        let last_seen_at: Option<i64> =
            sqlx::query_scalar("SELECT last_seen_at FROM codlet_sessions WHERE id = 's1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(last_seen_at, None);
        // Idempotent: every host startup calls it again.
        run_postgres_migrations(&pool)
            .await
            .expect("second run must be a no-op");
    }

    /// The whole session suite against a database that was upgraded from the
    /// pre-RFC-044 shape by the shipped runner (one container; each sub-test
    /// starts from an emptied table whose column is dropped and re-added).
    #[tokio::test]
    async fn postgres_session_store_conformance_on_an_upgraded_database() {
        let pool = bare_pg_pool().await;
        run_postgres_migrations(&pool).await.unwrap();
        codlet_conformance::run_session_store_conformance(|| {
            let pool = pool.clone();
            async move {
                sqlx::query("TRUNCATE codlet_sessions")
                    .execute(&pool)
                    .await
                    .unwrap();
                sqlx::query("ALTER TABLE codlet_sessions DROP COLUMN last_seen_at")
                    .execute(&pool)
                    .await
                    .unwrap();
                assert!(
                    !session_columns(&pool)
                        .await
                        .contains(&"last_seen_at".to_string()),
                    "setup: the column must be absent before the upgrade"
                );
                run_postgres_migrations(&pool).await.unwrap();
                PostgresStore::new(pool)
            }
        })
        .await;
    }

    #[tokio::test]
    async fn postgres_form_token_store_conformance() {
        codlet_conformance::run_form_token_store_conformance(fresh_pg_store).await;
    }

    // ── Migration idempotency ─────────────────────────────────────────────

    #[tokio::test]
    async fn postgres_migrations_are_idempotent() {
        let container = Postgres::default().start().await.unwrap();
        let url = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            container.get_host_port_ipv4(5432).await.unwrap()
        );
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        run_postgres_migrations(&pool).await.unwrap();
        // Second run must succeed (IF NOT EXISTS).
        run_postgres_migrations(&pool).await.unwrap();
        std::mem::forget(container);
    }

    #[tokio::test]
    async fn postgres_schema_uses_bigint_timestamps() {
        // RFC-034 §6: timestamps must be BIGINT, not INTEGER.
        let container = Postgres::default().start().await.unwrap();
        let url = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            container.get_host_port_ipv4(5432).await.unwrap()
        );
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        run_postgres_migrations(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as(
            "SELECT data_type FROM information_schema.columns
             WHERE table_name = 'codlet_codes' AND column_name = 'expires_at'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "bigint", "expires_at must be BIGINT");
        std::mem::forget(container);
    }

    // ── CodeAdminStore ────────────────────────────────────────────────────

    #[tokio::test]
    async fn postgres_admin_list_and_get() {
        let store = fresh_pg_store().await;
        store
            .insert_code(code_record("pg1", "secpg1", LATER, None))
            .await
            .unwrap();
        store
            .insert_code(code_record("pg2", "secpg2", LATER, Some("scope-P")))
            .await
            .unwrap();

        let all = store.list_codes(&CodeListFilter::all(), NOW).await.unwrap();
        assert_eq!(all.len(), 2);

        let scoped = store
            .list_codes(
                &CodeListFilter::active_in_scope(ScopeKey::new("scope-P")),
                NOW,
            )
            .await
            .unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].id, CodeId::new("pg2".into()));

        let meta = store
            .get_code_meta(&CodeId::new("pg1".into()))
            .await
            .unwrap()
            .expect("must exist");
        assert!(meta.created_at.is_some());
        assert!(meta.used_at.is_none());
    }

    // ── No RETURNING, no FOR UPDATE ───────────────────────────────────────

    #[tokio::test]
    async fn postgres_claim_uses_rows_affected_not_returning() {
        // RFC-034 §7: no RETURNING. The claim result is derived solely
        // from rows_affected() == classify_claim().
        // This test verifies the contract indirectly: a won claim produces
        // exactly one affected row, observable via the admin store.
        let store = fresh_pg_store().await;
        store
            .insert_code(code_record("pgclaim", "secpgclaim", LATER, None))
            .await
            .unwrap();
        let found = store
            .lookup_code(&[code_lk("secpgclaim")], None)
            .await
            .unwrap()
            .unwrap();
        let outcome = store
            .claim_code(&ClaimRequest {
                code_id: &found.id,
                subject: &codlet::secret::SubjectId::new("alice".into()),
                now: NOW,
                purpose: None,
                scope: None,
            })
            .await
            .unwrap();
        assert!(
            matches!(outcome, codlet::state::ClaimOutcome::Won),
            "expected Won, got {outcome:?}"
        );
        let meta = store
            .get_code_meta(&CodeId::new("pgclaim".into()))
            .await
            .unwrap()
            .unwrap();
        assert!(meta.used_at.is_some());
        assert_eq!(meta.used_by.as_ref().map(|s| s.as_str()), Some("alice"));
    }
}
