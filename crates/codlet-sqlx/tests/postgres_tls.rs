//! RFC-056: PostgreSQL connections over TLS, against a real server.
//!
//! Requires `--features postgres-test` and Docker (CI: the `test-postgres` and
//! `test-postgres-tls` jobs). Two groups, selected by whether `postgres-rustls`
//! is enabled:
//!
//! - **with** `postgres-rustls`: a `sslmode=require` connection to a TLS-enabled
//!   server succeeds, a real store operation runs over it, and the server itself
//!   reports the session as encrypted (`pg_stat_ssl`).
//! - **without** it: what a host gets today — `sslmode=require` fails closed, and
//!   the default `sslmode` silently sends everything in the clear.
//!
//! The encrypted/plaintext facts are read from the server's own view
//! (`pg_stat_ssl.ssl`), not inferred from the connect call succeeding.
#![cfg(feature = "postgres-test")]

use sqlx::postgres::PgPoolOptions;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};

/// `postgres://…` for a container, with an optional `?sslmode=` suffix.
async fn url(container: &ContainerAsync<Postgres>, sslmode: Option<&str>) -> String {
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let base = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    match sslmode {
        Some(mode) => format!("{base}?sslmode={mode}"),
        None => base,
    }
}

/// One connection (so `pg_backend_pid()` is stable), failing the test on error.
async fn connect(url: &str) -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect(url)
        .await
        .unwrap_or_else(|e| panic!("connect to {url} failed: {e}"))
}

/// The server's own report of whether *this* connection is encrypted.
async fn server_says_encrypted(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar("SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()")
        .fetch_one(pool)
        .await
        .expect("pg_stat_ssl row for this backend")
}

// ── Without the feature: today's behaviour, observed against a real server ──────

#[cfg(not(feature = "postgres-rustls"))]
mod without_postgres_rustls {
    use super::*;

    /// `sslmode=require` cannot be honoured, and says so rather than falling back.
    #[tokio::test]
    async fn sslmode_require_fails_closed() {
        let container = Postgres::default().start().await.unwrap();
        let url = url(&container, Some("require")).await;
        let err = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect_err("require must not silently succeed without TLS support");
        assert!(
            err.to_string().contains("without TLS support"),
            "unexpected error: {err}"
        );
    }

    /// The finding the RFC's threat rests on: with sqlx's default `sslmode`
    /// (`prefer`) and no TLS feature, the connection works — in plaintext, with
    /// no error. A host that does nothing gets this.
    #[tokio::test]
    async fn default_sslmode_silently_connects_in_plaintext() {
        let container = Postgres::default().start().await.unwrap();
        let pool = connect(&url(&container, None).await).await;
        assert!(
            !server_says_encrypted(&pool).await,
            "expected a plaintext connection; if this is now encrypted, the docs \
             that call the default unsafe need revisiting"
        );
    }
}

// ── With the feature: a real TLS connection and a real store operation ──────────

#[cfg(feature = "postgres-rustls")]
mod with_postgres_rustls {
    use super::*;
    use codlet::state::ClaimOutcome;
    use codlet::store::code::{ClaimRequest, CodeStore};
    use codlet_conformance::fixtures::{LATER, NOW, code_lk, code_record};
    use codlet_sqlx::{PostgresStore, run_postgres_migrations};
    use std::process::Command;
    use testcontainers_modules::testcontainers::ImageExt;

    /// A throwaway self-signed certificate and key (PEM), made with the `openssl`
    /// binary. `sslmode=require` encrypts without verifying the chain, so no CA is
    /// needed. A missing `openssl` fails the test; it is never skipped.
    fn self_signed() -> (Vec<u8>, Vec<u8>) {
        let dir = tempfile::tempdir().unwrap();
        let (crt, key) = (dir.path().join("server.crt"), dir.path().join("server.key"));
        let out = Command::new("openssl")
            .args(["req", "-x509", "-newkey", "rsa:2048", "-nodes"])
            .args(["-subj", "/CN=localhost", "-days", "1"])
            .arg("-keyout")
            .arg(&key)
            .arg("-out")
            .arg(&crt)
            .output()
            .expect("the `openssl` binary is required to run the TLS test");
        assert!(
            out.status.success(),
            "openssl failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        (std::fs::read(&crt).unwrap(), std::fs::read(&key).unwrap())
    }

    /// A PostgreSQL container with `ssl=on`.
    ///
    /// The server refuses a key file it cannot own, or one that is group/world
    /// readable, so the files are copied in and then `chown`ed to `postgres` and
    /// `chmod 600`ed by a small startup script, which then hands over to the
    /// image's normal entrypoint.
    async fn tls_container() -> ContainerAsync<Postgres> {
        let (crt, key) = self_signed();
        let script = "set -e; \
            mkdir -p /var/lib/postgresql-tls; \
            cp /tmp/tls/server.crt /tmp/tls/server.key /var/lib/postgresql-tls/; \
            chown postgres:postgres /var/lib/postgresql-tls/server.crt /var/lib/postgresql-tls/server.key; \
            chmod 600 /var/lib/postgresql-tls/server.key; \
            exec docker-entrypoint.sh postgres -c fsync=off -c ssl=on \
              -c ssl_cert_file=/var/lib/postgresql-tls/server.crt \
              -c ssl_key_file=/var/lib/postgresql-tls/server.key";
        Postgres::default()
            .with_copy_to("/tmp/tls/server.crt", crt)
            .with_copy_to("/tmp/tls/server.key", key)
            .with_cmd(["sh", "-c", script])
            .start()
            .await
            .expect("TLS-enabled PostgreSQL container")
    }

    #[tokio::test]
    async fn sslmode_require_connects_and_a_store_operation_runs_over_it() {
        let container = tls_container().await;

        // The control: the server really is reachable without TLS and says so.
        // It makes the encrypted assertion below meaningful — `pg_stat_ssl`
        // distinguishes the two.
        let plain = connect(&url(&container, Some("disable")).await).await;
        assert!(
            !server_says_encrypted(&plain).await,
            "control: sslmode=disable must be a plaintext connection"
        );

        // The feature under test.
        let pool = connect(&url(&container, Some("require")).await).await;
        assert!(
            server_says_encrypted(&pool).await,
            "sslmode=require with postgres-rustls must give a TLS connection"
        );

        // A real store operation, end to end, over that connection: schema,
        // insert, lookup, and the INV-5 conditional claim.
        run_postgres_migrations(&pool).await.unwrap();
        let store = PostgresStore::new(pool);
        store
            .insert_code(code_record("tls1", "tlssecret1", LATER, None))
            .await
            .unwrap();
        let found = store
            .lookup_code(&[code_lk("tlssecret1")], None)
            .await
            .unwrap()
            .expect("the code must be found over TLS");
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
        assert_eq!(outcome, ClaimOutcome::Won);
    }
}
