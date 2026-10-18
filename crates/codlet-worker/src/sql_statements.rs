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
//! Understands `--` line comments, `/* */` block comments, `'…'` string literals
//! and `"…"` quoted identifiers (in both quoted forms, a doubled quote character
//! is an escaped one). Everything else is passed through unchanged, which means
//! the following are **not** understood, and a `;` or `--` inside them would be
//! misread as a boundary or a comment: backtick-quoted identifiers (`` `x` ``),
//! bracket-quoted identifiers (`[x]`), and dollar-quoted strings (`$$ … $$`).
//! codlet's migration files use none of them; if a future edit needs one, extend
//! the scanner and add a test, rather than working around it in the SQL.
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
            // A string literal (`'…'`) or quoted identifier (`"…"`) runs to its
            // closing quote; a doubled quote is an escaped one and stays inside.
            // Nothing in it is a comment or a `;`.
            '\'' | '"' => {
                current.push(c);
                copy_quoted(&mut chars, &mut current, c);
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

/// Copy a quoted run to `out`, up to and including its closing `quote`, treating
/// a doubled `quote` as an escaped one. The opening quote has been consumed.
fn copy_quoted(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    out: &mut String,
    quote: char,
) {
    while let Some(next) = chars.next() {
        out.push(next);
        if next == quote {
            if chars.peek() == Some(&quote) {
                out.push(quote);
                chars.next();
            } else {
                return;
            }
        }
    }
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
