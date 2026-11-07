//! Full-text search with SQLite FTS5.
//!
//! Each user table gets an external-content FTS5 table (`s_<hex>`) over its live text and phone columns,
//! kept current by triggers. The index points at the record table's `seq` column, an explicit integer key,
//! so it survives `VACUUM`, which may renumber implicit rowids.
//!
//! Request text never reaches `MATCH` as written. `match_expr` reduces it to quoted words, so FTS5 operators
//! and column filters in the input have no effect.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::{
    catalog::{self, ident, FieldOut, Table},
    error::AppError,
};

const MAX_TOKENS: usize = 10;
const MAX_TOKEN_LEN: usize = 100;
pub const DEFAULT_LIMIT: usize = 5;
pub const MAX_LIMIT: usize = 50;

/// Field types whose values are indexed.
fn is_indexed(field: &FieldOut) -> bool {
    matches!(field.field_type, "text" | "phone")
}

/// The hex part of a table's generated name. Checking it as an identifier proves it is safe to reuse in other names.
fn hex(table: &Table) -> Result<&str, AppError> {
    ident(&table.sql_name)?;
    Ok(&table.sql_name[2..])
}

/// The quoted name of the table's FTS5 table.
pub fn fts_ident(table: &Table) -> Result<String, AppError> {
    ident(&format!("s_{}", hex(table)?))
}

/// Drops and recreates the table's FTS5 table and triggers for `fields`, then fills the index from the rows.
/// Call it whenever an indexed field is added or removed. `fields` must be the table's live fields.
pub fn rebuild_index(conn: &Connection, table: &Table, fields: &[FieldOut]) -> Result<(), AppError> {
    let hex = hex(table)?;
    let t = ident(&table.sql_name)?;
    let fts = ident(&format!("s_{hex}"))?;
    let on_insert = ident(&format!("a_{hex}"))?;
    let on_delete = ident(&format!("d_{hex}"))?;
    let on_update = ident(&format!("u_{hex}"))?;

    conn.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS {on_insert};
         DROP TRIGGER IF EXISTS {on_delete};
         DROP TRIGGER IF EXISTS {on_update};
         DROP TABLE IF EXISTS {fts};"
    ))?;

    let columns = fields
        .iter()
        .filter(|f| is_indexed(f))
        .map(|f| ident(&f.sql_name))
        .collect::<Result<Vec<_>, _>>()?;
    if columns.is_empty() {
        return Ok(());
    }
    let list = columns.join(", ");
    let row = |prefix: &str| columns.iter().map(|c| format!("{prefix}.{c}")).collect::<Vec<_>>().join(", ");
    let (new_row, old_row) = (row("new"), row("old"));

    // With external content, a delete must repeat the old values, so the index can remove the right terms.
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE {fts} USING fts5(
            {list}, content='{name}', content_rowid='seq', tokenize='unicode61 remove_diacritics 2'
         );
         CREATE TRIGGER {on_insert} AFTER INSERT ON {t} BEGIN
            INSERT INTO {fts}(rowid, {list}) VALUES (new.seq, {new_row});
         END;
         CREATE TRIGGER {on_delete} AFTER DELETE ON {t} BEGIN
            INSERT INTO {fts}({fts}, rowid, {list}) VALUES ('delete', old.seq, {old_row});
         END;
         CREATE TRIGGER {on_update} AFTER UPDATE OF {list} ON {t} BEGIN
            INSERT INTO {fts}({fts}, rowid, {list}) VALUES ('delete', old.seq, {old_row});
            INSERT INTO {fts}(rowid, {list}) VALUES (new.seq, {new_row});
         END;
         INSERT INTO {fts}({fts}) VALUES ('rebuild');",
        name = table.sql_name,
    ))?;
    Ok(())
}

/// Turns search text into an FTS5 `MATCH` expression: every word quoted, all of them required,
/// and the last one matching as a prefix, so partial words work while typing.
pub fn match_expr(q: &str) -> Result<String, AppError> {
    let words: Vec<&str> = q.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return Err(AppError::BadRequest("search text needs at least one letter or digit".into()));
    }
    if words.len() > MAX_TOKENS {
        return Err(AppError::BadRequest(format!("search text has more than {MAX_TOKENS} words")));
    }
    if words.iter().any(|w| w.chars().count() > MAX_TOKEN_LEN) {
        return Err(AppError::BadRequest(format!("search words are limited to {MAX_TOKEN_LEN} characters")));
    }
    let last = words.len() - 1;
    // Only alphanumeric characters survive the split, so a word cannot contain a quote.
    let expr = words
        .iter()
        .enumerate()
        .map(|(i, w)| if i == last { format!("\"{w}\"*") } else { format!("\"{w}\"") })
        .collect::<Vec<_>>()
        .join(" ");

    // A phone number is indexed as one run of digits, but people type it with spaces, dashes and parentheses.
    // For text made only of digits and such punctuation, also try the digits joined together.
    let numeric = q.chars().all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | '(' | ')' | ' '));
    if numeric && words.len() > 1 {
        let digits = words.concat();
        if digits.len() <= MAX_TOKEN_LEN {
            return Ok(format!("({expr}) OR \"{digits}\"*"));
        }
    }
    Ok(expr)
}

#[derive(Serialize)]
pub struct TableRef {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct Match {
    pub id: String,
    /// The record's primary field.
    pub title: Option<String>,
    /// Plain text around the match, with the matched words wrapped in « ».
    pub snippet: String,
}

#[derive(Serialize)]
pub struct TableMatches {
    pub table: TableRef,
    pub matches: Vec<Match>,
}

/// Searches every live table and returns up to `limit` matches per table, best first. Tables without matches are left out.
pub fn global(conn: &Connection, q: &str, limit: usize) -> Result<Vec<TableMatches>, AppError> {
    let expr = match_expr(q)?;
    let tx = conn.unchecked_transaction()?;
    let mut results = Vec::new();
    for table in catalog::list_tables(&tx)? {
        let fields = catalog::read_fields(&tx, &table.id)?;
        let Some(primary) = fields.iter().find(|f| f.is_primary) else { continue };
        let (t, fts, title) = (ident(&table.sql_name)?, fts_ident(&table)?, ident(&primary.sql_name)?);
        let sql = format!(
            "SELECT r.id, r.{title}, snippet({fts}, -1, '«', '»', '…', 12)
             FROM {fts} JOIN {t} AS r ON r.seq = {fts}.rowid
             WHERE {fts} MATCH ?1 ORDER BY rank LIMIT ?2"
        );
        let mut stmt = tx.prepare(&sql)?;
        let rows = stmt.query_map(params![expr, limit as i64], |r| {
            Ok(Match { id: r.get(0)?, title: r.get(1)?, snippet: r.get(2)? })
        })?;
        let matches = rows.collect::<Result<Vec<_>, _>>()?;
        if !matches.is_empty() {
            results.push(TableMatches { table: TableRef { id: table.id, name: table.name }, matches });
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::{
        catalog::{add_field, create_table, delete_field, resolve_table, FieldConfig, NewField},
        db::Db,
        records,
    };

    #[test]
    fn match_expr_quotes_words_and_prefixes_the_last() {
        assert_eq!(match_expr("acme").unwrap(), "\"acme\"*");
        assert_eq!(match_expr("  Acme   corp ").unwrap(), "\"Acme\" \"corp\"*");
        assert_eq!(match_expr("café münster").unwrap(), "\"café\" \"münster\"*");
    }

    #[test]
    fn match_expr_also_tries_numbers_with_the_punctuation_removed() {
        assert_eq!(
            match_expr("+1 (415) 555-2671").unwrap(),
            "(\"1\" \"415\" \"555\" \"2671\"*) OR \"14155552671\"*"
        );
        assert_eq!(match_expr("2026-03-01").unwrap(), "(\"2026\" \"03\" \"01\"*) OR \"20260301\"*");
        // One group of digits is already a single word, and mixed text is not a number.
        assert_eq!(match_expr("14155552671").unwrap(), "\"14155552671\"*");
        assert_eq!(match_expr("room 12 34").unwrap(), "\"room\" \"12\" \"34\"*");
    }

    #[test]
    fn match_expr_neutralises_fts_syntax() {
        // Operators, column filters, quotes and wildcards become ordinary quoted words, or vanish.
        assert_eq!(match_expr("a OR b").unwrap(), "\"a\" \"OR\" \"b\"*");
        assert_eq!(match_expr("col:x NEAR(a b)").unwrap(), "\"col\" \"x\" \"NEAR\" \"a\" \"b\"*");
        assert_eq!(match_expr("\"quoted\" -x *").unwrap(), "\"quoted\" \"x\"*");
        assert_eq!(match_expr("x\" OR \"y").unwrap(), "\"x\" \"OR\" \"y\"*");
    }

    #[test]
    fn match_expr_rejects_unusable_text() {
        for bad in ["", "   ", "***", "\"\"", "()-:", &"a ".repeat(11), &"x".repeat(101)] {
            assert!(matches!(match_expr(bad), Err(AppError::BadRequest(_))), "{bad:?}");
        }
        assert!(match_expr(&"a ".repeat(10)).is_ok());
        assert!(match_expr(&"x".repeat(100)).is_ok());
    }

    /// A database holding the user `u1`, because `tables.created_by` is a foreign key.
    fn db() -> (Arc<Db>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("test.db")).unwrap();
        let conn = Connection::open(dir.path().join("test.db")).unwrap();
        conn.execute(
            "INSERT INTO users (id, email, password_hash, is_admin, created_at) VALUES ('u1', 'u1@example.com', 'x', 1, 'x')",
            [],
        )
        .unwrap();
        (db, dir)
    }

    fn new_field(name: &str, ty: &str) -> NewField {
        NewField { name: name.into(), field_type: ty.into(), config: FieldConfig::default(), options: None }
    }

    fn values(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        v.as_object().unwrap().clone()
    }

    /// FTS5 compares the index to the content table and fails if they disagree.
    fn assert_index_matches_content(conn: &Connection, table: &Table) {
        let fts = fts_ident(table).unwrap();
        conn.execute(&format!("INSERT INTO {fts}({fts}, rank) VALUES ('integrity-check', 1)"), [])
            .unwrap_or_else(|e| panic!("search index disagrees with the table: {e}"));
    }

    #[tokio::test]
    async fn the_index_stays_consistent_through_writes_and_schema_changes() {
        let (db, _dir) = db();
        db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
        db.write(|c| add_field(c, "Deals", new_field("Notes", "text"))).await.unwrap();

        let ids = db
            .write(|c| {
                let a = records::create(c, "Deals", "u1", &values(json!({ "Name": "Acme", "Notes": "first note" })))?;
                let b = records::create(c, "Deals", "u1", &values(json!({ "Name": "Beta", "Notes": "second note" })))?;
                let g = records::create(c, "Deals", "u1", &values(json!({ "Name": "Gamma" })))?;
                Ok((a.id, b.id, g.id))
            })
            .await
            .unwrap();
        let table = db.read(|c| resolve_table(c, "Deals")).await.unwrap();
        let t = table.clone();
        db.read(move |c| Ok(assert_index_matches_content(c, &t))).await.unwrap();

        // Updates, clearing a value, and deletes.
        let (a, b, g) = ids;
        db.write(move |c| {
            records::update(c, "Deals", &a, "u1", &values(json!({ "Notes": "rewritten" })))?;
            records::update(c, "Deals", &b, "u1", &values(json!({ "Notes": null })))?;
            records::update(c, "Deals", &g, "u1", &values(json!({ "Name": "Gamma Two" })))?;
            records::delete(c, "Deals", &g)?;
            Ok(())
        })
        .await
        .unwrap();
        let t = table.clone();
        db.read(move |c| Ok(assert_index_matches_content(c, &t))).await.unwrap();

        // A new text field is indexed from then on, and a deleted one is dropped from the index.
        db.write(|c| add_field(c, "Deals", new_field("Mobile", "phone"))).await.unwrap();
        db.write(|c| delete_field(c, "Deals", "Notes")).await.unwrap();
        db.write(|c| records::create(c, "Deals", "u1", &values(json!({ "Name": "Delta", "Mobile": "+14155552671" }))))
            .await
            .unwrap();
        let t = table.clone();
        db.read(move |c| Ok(assert_index_matches_content(c, &t))).await.unwrap();

        let hits = |q: &'static str| {
            let db = db.clone();
            async move { db.read(move |c| global(c, q, 10)).await.unwrap() }
        };
        assert!(hits("rewritten").await.is_empty(), "text of a deleted field is no longer searchable");
        assert!(!hits("14155552671").await.is_empty());
        assert!(!hits("delta").await.is_empty());
        assert!(hits("gamma").await.is_empty(), "deleted records leave the index");
    }

    #[tokio::test]
    async fn the_index_survives_vacuum() {
        let (db, _dir) = db();
        db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
        // Delete the first record so the survivors' positions in the table would shift if rowids were implicit.
        let ids = db
            .write(|c| {
                let a = records::create(c, "Deals", "u1", &values(json!({ "Name": "alpha" })))?;
                let b = records::create(c, "Deals", "u1", &values(json!({ "Name": "bravo" })))?;
                let g = records::create(c, "Deals", "u1", &values(json!({ "Name": "charlie" })))?;
                records::delete(c, "Deals", &a.id)?;
                Ok((b.id, g.id))
            })
            .await
            .unwrap();

        db.write(|c| {
            c.execute_batch("VACUUM")?;
            Ok(())
        })
        .await
        .unwrap();

        let table = db.read(|c| resolve_table(c, "Deals")).await.unwrap();
        db.read(move |c| Ok(assert_index_matches_content(c, &table))).await.unwrap();
        let found = db.read(|c| global(c, "charlie", 5)).await.unwrap();
        assert_eq!(found[0].matches.len(), 1);
        assert_eq!(found[0].matches[0].id, ids.1, "the match still points at the right record");
        assert_eq!(found[0].matches[0].title.as_deref(), Some("charlie"));
        let found = db.read(|c| global(c, "bravo", 5)).await.unwrap();
        assert_eq!(found[0].matches[0].id, ids.0);
    }

    #[tokio::test]
    async fn every_table_has_an_index_and_triggers() {
        let (db, _dir) = db();
        let detail = db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
        let hex = detail.table.sql_name[2..].to_string();
        let names = db
            .read(move |c| {
                let mut stmt =
                    c.prepare("SELECT name FROM sqlite_master WHERE name LIKE ?1 ESCAPE '\\' ORDER BY name")?;
                let rows = stmt.query_map([format!("%\\_{hex}")], |r| r.get::<_, String>(0))?;
                Ok(rows.collect::<Result<Vec<_>, _>>()?)
            })
            .await
            .unwrap();
        let prefixes: Vec<&str> = names.iter().map(|n| &n[..2]).collect();
        assert_eq!(prefixes, ["a_", "d_", "s_", "t_", "u_"], "insert, delete and update triggers, the index, and the table");
    }
}
