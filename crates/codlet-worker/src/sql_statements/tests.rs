use super::split_statements;

#[test]
fn semicolon_inside_a_line_comment_does_not_split() {
    // RFC-038's trap, and the shape of the real shipped file: a `;` in a `--`
    // comment must neither end a statement nor leak the comment's tail into SQL.
    let sql = "CREATE TABLE a (\n  x INTEGER -- note (RFC-044); bound as REAL\n);\nSELECT 1;";
    assert_eq!(
        split_statements(sql),
        vec!["CREATE TABLE a (\n  x INTEGER \n)", "SELECT 1"]
    );
}

#[test]
fn comment_only_lines_are_dropped() {
    let sql = "-- header\n-- more header\nCREATE TABLE a (x INTEGER);\n-- trailing comment";
    assert_eq!(split_statements(sql), vec!["CREATE TABLE a (x INTEGER)"]);
}

#[test]
fn semicolon_inside_a_string_literal_does_not_split() {
    let sql = "INSERT INTO t VALUES ('a;b'); SELECT 2;";
    assert_eq!(
        split_statements(sql),
        vec!["INSERT INTO t VALUES ('a;b')", "SELECT 2"]
    );
}

#[test]
fn comment_marker_inside_a_string_literal_is_text() {
    let sql = "INSERT INTO t VALUES ('--not a comment; really'); SELECT 3;";
    assert_eq!(
        split_statements(sql),
        vec![
            "INSERT INTO t VALUES ('--not a comment; really')",
            "SELECT 3"
        ]
    );
}

#[test]
fn escaped_quote_does_not_end_the_string() {
    let sql = "INSERT INTO t VALUES ('it''s; fine'); SELECT 4;";
    assert_eq!(
        split_statements(sql),
        vec!["INSERT INTO t VALUES ('it''s; fine')", "SELECT 4"]
    );
}

#[test]
fn block_comments_are_removed_and_may_contain_semicolons() {
    let sql = "SELECT /* a; b */ 5; /* whole; statement; */ SELECT 6;";
    assert_eq!(split_statements(sql), vec!["SELECT   5", "SELECT 6"]);
}

#[test]
fn a_final_statement_without_a_semicolon_is_kept() {
    assert_eq!(
        split_statements("SELECT 1; SELECT 2"),
        vec!["SELECT 1", "SELECT 2"]
    );
}

#[test]
fn empty_and_whitespace_only_input_yields_nothing() {
    assert!(split_statements("").is_empty());
    assert!(split_statements(" \n ; ;\n-- only a comment\n").is_empty());
}

#[test]
fn the_shipped_migration_splits_into_its_seven_statements() {
    let sql = include_str!("../../migrations/0001_initial.sql");
    let statements = split_statements(sql);
    // 3 tables + 4 indexes.
    assert_eq!(statements.len(), 7, "{statements:#?}");
    for s in &statements {
        assert!(!s.contains("--"), "a comment survived: {s}");
        assert!(
            s.starts_with("CREATE TABLE IF NOT EXISTS")
                || s.starts_with("CREATE INDEX IF NOT EXISTS"),
            "not a whole statement: {s}"
        );
    }
    // The statement carrying the `;`-in-a-comment must be intact, not cut at it.
    let sessions = statements
        .iter()
        .find(|s| s.contains("codlet_sessions (") && s.starts_with("CREATE TABLE"))
        .expect("sessions table");
    assert!(sessions.contains("last_seen_at  INTEGER"), "{sessions}");
}
