//! Splitting a migration script into statements (RFC-057).
//!
//! D1 cannot take codlet's migration as one script: `D1Database::exec` treats
//! each *line* as a statement, so the file's first line (a comment) is rejected
//! and every multi-line `CREATE TABLE` is cut in half. The runner therefore
//! applies the statements one at a time, which means something has to find the
//! statement boundaries.
//!
//! That is a hand-rolled SQL scanner, which RFC-038 found is how a `;` inside a
//! `--` comment ends up submitted as a statement. So this is **the one place in
//! the workspace that does it**, it scans comments and quoted text as it goes
//! rather than splitting first and cleaning up after, and it has its own tests.
//! `codlet-sqlx` never needs this: its drivers parse the whole script.
//!
//! Understands exactly what codlet's migration files use: `--` line comments,
//! `/* */` block comments, and `'…'` string literals (with `''` as an escaped
//! quote). Anything else is passed through unchanged.
//!
//! This module has no `worker` dependency, so its tests run on native targets.

/// Split `sql` into statements: comments removed, split on `;` that is outside a
/// comment or a string literal, each statement trimmed, empty ones dropped.
/// The terminating `;` is not included.
#[must_use]
pub(crate) fn split_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // `-- …` to end of line. The newline itself is kept so two lines
            // are never fused into one token.
            '-' if chars.peek() == Some(&'-') => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        current.push('\n');
                        break;
                    }
                }
            }
            // `/* … */`, replaced by a space for the same reason.
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = '\0';
                for next in chars.by_ref() {
                    if prev == '*' && next == '/' {
                        break;
                    }
                    prev = next;
                }
                current.push(' ');
            }
            // A string literal runs to its closing quote; `''` is an escaped
            // quote and stays inside it. Nothing in it is a comment or a `;`.
            '\'' => {
                current.push(c);
                while let Some(next) = chars.next() {
                    current.push(next);
                    if next == '\'' {
                        if chars.peek() == Some(&'\'') {
                            current.push(chars.next().unwrap_or('\''));
                        } else {
                            break;
                        }
                    }
                }
            }
            ';' => {
                push_trimmed(&mut statements, &mut current);
            }
            other => current.push(other),
        }
    }
    push_trimmed(&mut statements, &mut current);
    statements
}

fn push_trimmed(statements: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        statements.push(trimmed.to_string());
    }
    current.clear();
}

#[cfg(test)]
mod tests;
