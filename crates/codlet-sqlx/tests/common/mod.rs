//! Shared PostgreSQL test fixtures (RFC-058). Not a test target itself.
//!
//! **The server version is chosen here, in codlet's own source, and checked.**
//! Before RFC-058 every PostgreSQL test ran on `postgres:11-alpine` — an
//! end-of-life server — because `testcontainers-modules` hardcodes that tag and
//! nothing here overrode it. Pinning the tag alone would not have prevented
//! that: a dropped override, an unset CI variable, or a dependency bump would
//! quietly test the wrong version and stay green. So the fixtures also ask the
//! server it connected to which major version it is and fail if that is not the
//! one requested ([`assert_server_major`]).
#![allow(dead_code)] // each test binary uses a subset of these helpers

use sqlx::PgPool;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, ContainerRequest, ImageExt, runners::AsyncRunner},
};

/// The PostgreSQL major version tested when nothing says otherwise: the
/// **floor** of the supported range (15–18, RFC-058), so a plain local
/// `cargo test` runs a supported server.
pub const DEFAULT_PG_MAJOR: u32 = 15;

/// Environment variable CI sets per matrix leg to choose the major version.
pub const PG_MAJOR_ENV: &str = "CODLET_TEST_PG_MAJOR";

/// The major version these tests are asked to run against.
pub fn requested_pg_major() -> u32 {
    match std::env::var(PG_MAJOR_ENV) {
        Ok(v) => v.trim().parse().unwrap_or_else(|_| {
            panic!("{PG_MAJOR_ENV} must be a PostgreSQL major version number, got {v:?}")
        }),
        Err(_) => DEFAULT_PG_MAJOR,
    }
}

/// The PostgreSQL image to start, with the tag set here — never inherited from
/// `testcontainers-modules`' default.
pub fn postgres_image() -> ContainerRequest<Postgres> {
    Postgres::default().with_tag("18-alpine") // TRIAL: ignores the request
}

/// Fail unless the server behind `pool` is the major version that was requested.
///
/// Reads `server_version_num` (e.g. `150004` for 15.4) and compares only the
/// major component: minors drift as images are rebuilt, so they are not pinned.
pub async fn assert_server_major(pool: &PgPool) {
    let num: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(pool)
        .await
        .expect("read server_version_num");
    let actual = (num / 10_000) as u32;
    let requested = requested_pg_major();
    assert_eq!(
        actual, requested,
        "connected to PostgreSQL {actual} but {requested} was requested \
         ({PG_MAJOR_ENV}): the version pin is not doing its job"
    );
    println!("PostgreSQL server major version {actual} (requested {requested})");
}

/// Connect to `url`, then check the server's major version (see
/// [`assert_server_major`]).
pub async fn connect_checked(url: &str) -> PgPool {
    let pool = PgPool::connect(url)
        .await
        .unwrap_or_else(|e| panic!("connect to {url} failed: {e}"));
    assert_server_major(&pool).await;
    pool
}

/// Start a pinned PostgreSQL container and connect to it, version-checked.
/// The caller decides the container's lifetime (the existing tests `forget` it).
pub async fn start_pg() -> (ContainerAsync<Postgres>, PgPool) {
    let container = postgres_image()
        .start()
        .await
        .expect("start PostgreSQL container");
    let url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        container.get_host_port_ipv4(5432).await.unwrap()
    );
    let pool = connect_checked(&url).await;
    (container, pool)
}
