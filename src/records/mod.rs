//! Record CRUD on user tables. Columns come from the catalog; values are always bound parameters.
//!
//! Most fields live in a column of the record's row. Multi-select and to-many relation fields live in
//! a join table instead, so they are written and read in separate statements.

mod filter;
pub(crate) mod value;

use std::collections::{HashMap, HashSet};

use rusqlite::{params, params_from_iter, types::Value as Sql, Connection, Row, Transaction};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    auth::{new_id, NOW},
    catalog::{self, ident, FieldOut, Table},
    error::AppError,
    search,
};

pub const DEFAULT_LIMIT: usize = 50;
pub const MAX_LIMIT: usize = 200;
const MAX_CURSOR: usize = 20_000;
const SYSTEM_COLUMNS: &str = "\"id\", \"created_at\", \"updated_at\", \"created_by\", \"updated_by\"";

#[derive(Serialize)]
pub struct Record {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    /// Field values keyed by field name, in field order.
    pub values: Map<String, Value>,
}

#[derive(Serialize)]
pub struct RecordList {
    pub records: Vec<Record>,
    pub next_cursor: Option<String>,
}

pub struct ListParams {
    /// Full-text search text. Narrows the list like a filter; the order still follows `sort`.
    pub q: Option<String>,
    pub filter: Option<Value>,
    pub sort: Option<Value>,
    pub limit: usize,
    pub cursor: Option<String>,
}

pub(crate) use value::option_id;

/// Checks that `filter` is valid for `fields`, without running it. Saved views use this.
pub fn validate_filter(fields: &[FieldOut], filter: &Value) -> Result<(), AppError> {
    filter::compile_filter(fields, filter).map(|_| ())
}

/// Checks that `sort` is valid for `fields`, without running it.
pub fn validate_sort(fields: &[FieldOut], sort: &Value) -> Result<(), AppError> {
    filter::compile_sort(fields, Some(sort)).map(|_| ())
}

/// Finds a field by ID, or else by name (case-insensitive).
pub(crate) fn field_by_key<'a>(fields: &'a [FieldOut], key: &str) -> Option<&'a FieldOut> {
    fields
        .iter()
        .find(|f| f.id == key)
        .or_else(|| fields.iter().find(|f| f.name.to_lowercase() == key.to_lowercase()))
}

/// What to store for one field: a column value, or the IDs to link through a join table.
enum Cell {
    Column(Sql),
    Links(Vec<String>),
}

/// Validates a `values` object and pairs each entry with its field.
fn assign<'a>(fields: &'a [FieldOut], values: &Map<String, Value>) -> Result<Vec<(&'a FieldOut, Cell)>, AppError> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(values.len());
    for (key, v) in values {
        let field = field_by_key(fields, key)
            .ok_or_else(|| AppError::BadRequest(format!("unknown field `{key}`")))?;
        if !seen.insert(field.id.as_str()) {
            return Err(AppError::BadRequest(format!("field `{}` is given more than once", field.name)));
        }
        let cell = if field.has_column() {
            Cell::Column(value::to_sql(field, v)?)
        } else {
            Cell::Links(value::to_links(field, v)?)
        };
        out.push((field, cell));
    }
    Ok(out)
}

/// Relation targets must exist. The foreign keys would also refuse a bad link, but with an opaque error.
fn check_targets(tx: &Transaction, assigned: &[(&FieldOut, Cell)]) -> Result<(), AppError> {
    for (field, cell) in assigned {
        if field.field_type != "relation" {
            continue;
        }
        let ids: Vec<&str> = match cell {
            Cell::Column(Sql::Text(id)) => vec![id.as_str()],
            Cell::Links(ids) => ids.iter().map(String::as_str).collect(),
            Cell::Column(_) => continue,
        };
        if ids.is_empty() {
            continue;
        }
        let target = catalog::relation_target(tx, field)?;
        let marks = vec!["?"; ids.len()].join(", ");
        let found: i64 = tx.query_row(
            &format!("SELECT COUNT(*) FROM {} WHERE id IN ({marks})", ident(&target.sql_name)?),
            params_from_iter(ids.iter()),
            |r| r.get(0),
        )?;
        if found as usize != ids.len() {
            return Err(AppError::BadRequest(format!(
                "field `{}`: some linked records do not exist in table `{}`",
                field.name, target.name
            )));
        }
    }
    Ok(())
}

/// Replaces the links of each join-table field in `assigned`.
fn write_links(tx: &Transaction, record_id: &str, assigned: &[(&FieldOut, Cell)]) -> Result<(), AppError> {
    for (field, cell) in assigned {
        let Cell::Links(ids) = cell else { continue };
        let join = ident(&field.sql_name)?;
        let (own, other) = field.link_columns();
        tx.execute(&format!("DELETE FROM {join} WHERE {own} = ?1"), [record_id])?;
        if field.exclusive() {
            // Each of these records may be linked from only one record, so it moves here from its old owner.
            for id in ids {
                tx.execute(&format!("DELETE FROM {join} WHERE {other} = ?1 AND {own} != ?2"), params![id, record_id])?;
            }
        }
        for (position, id) in ids.iter().enumerate() {
            tx.execute(
                &format!("INSERT INTO {join} ({own}, {other}, position) VALUES (?1, ?2, ?3)"),
                params![record_id, id, position as i64],
            )?;
        }
    }
    Ok(())
}

/// The system columns, then one column per field that has one.
fn select_list(fields: &[FieldOut]) -> Result<String, AppError> {
    let mut cols = vec![SYSTEM_COLUMNS.to_string()];
    for f in fields.iter().filter(|f| f.has_column()) {
        cols.push(ident(&f.sql_name)?);
    }
    Ok(cols.join(", "))
}

/// A row as read from the record table, before the join-table fields are filled in.
struct Raw {
    id: String,
    created_at: String,
    updated_at: String,
    created_by: Option<String>,
    updated_by: Option<String>,
    /// One value per field that has a column, in field order.
    cells: Vec<Value>,
}

fn read_raw(fields: &[FieldOut], row: &Row) -> rusqlite::Result<Raw> {
    let mut cells = Vec::new();
    for (i, field) in fields.iter().filter(|f| f.has_column()).enumerate() {
        cells.push(value::from_sql(field, row.get::<_, Sql>(5 + i)?));
    }
    Ok(Raw {
        id: row.get(0)?,
        created_at: row.get(1)?,
        updated_at: row.get(2)?,
        created_by: row.get(3)?,
        updated_by: row.get(4)?,
        cells,
    })
}

/// Join-table values: field ID, then record ID, then the values in link order.
type Links = HashMap<String, HashMap<String, Vec<Value>>>;

fn load_links(conn: &Connection, fields: &[FieldOut], ids: &[&str]) -> Result<Links, AppError> {
    let mut links = Links::new();
    if ids.is_empty() {
        return Ok(links);
    }
    let marks = vec!["?"; ids.len()].join(", ");
    for field in fields.iter().filter(|f| !f.has_column()) {
        let (own, other) = field.link_columns();
        let sql = format!(
            "SELECT {own}, {other} FROM {} WHERE {own} IN ({marks}) ORDER BY position",
            ident(&field.sql_name)?,
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(ids.iter()), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let per_field = links.entry(field.id.clone()).or_default();
        for row in rows {
            let (record_id, linked) = row?;
            let v = if field.field_type == "multi_select" {
                match field.options.iter().find(|o| o.id == linked) {
                    Some(option) => Value::String(option.label.clone()),
                    None => continue,
                }
            } else {
                Value::String(linked)
            };
            per_field.entry(record_id).or_default().push(v);
        }
    }
    Ok(links)
}

/// Joins a raw row with its join-table values into the record the API returns.
fn assemble(fields: &[FieldOut], raw: Raw, links: &Links) -> Record {
    let Raw { id, created_at, updated_at, created_by, updated_by, cells } = raw;
    let mut cells = cells.into_iter();
    let mut values = Map::new();
    for field in fields {
        let v = if field.has_column() {
            cells.next().unwrap_or(Value::Null)
        } else {
            let mut linked = links.get(&field.id).and_then(|per| per.get(&id)).cloned().unwrap_or_default();
            if field.single_link() {
                if linked.is_empty() { Value::Null } else { linked.swap_remove(0) }
            } else {
                Value::Array(linked)
            }
        };
        values.insert(field.name.clone(), v);
    }
    Record { id, created_at, updated_at, created_by, updated_by, values }
}

fn fetch(conn: &Connection, table: &Table, fields: &[FieldOut], id: &str) -> Result<Record, AppError> {
    let sql = format!("SELECT {} FROM {} WHERE id = ?1", select_list(fields)?, ident(&table.sql_name)?);
    let mut stmt = conn.prepare(&sql)?;
    let raw = stmt
        .query_map([id], |r| read_raw(fields, r))?
        .next()
        .transpose()?
        .ok_or(AppError::NotFound)?;
    let links = load_links(conn, fields, &[id])?;
    Ok(assemble(fields, raw, &links))
}

pub fn get(conn: &Connection, table_key: &str, id: &str) -> Result<Record, AppError> {
    // One transaction, so the catalog and the data come from the same snapshot.
    let tx = conn.unchecked_transaction()?;
    let table = catalog::resolve_table(&tx, table_key)?;
    let fields = catalog::read_fields(&tx, &table.id)?;
    fetch(&tx, &table, &fields, id)
}

pub fn list(conn: &Connection, table_key: &str, params: ListParams) -> Result<RecordList, AppError> {
    let tx = conn.unchecked_transaction()?;
    let table = catalog::resolve_table(&tx, table_key)?;
    let fields = catalog::read_fields(&tx, &table.id)?;

    let mut wheres = Vec::new();
    let mut binds: Vec<Sql> = Vec::new();
    if let Some(f) = &params.filter {
        let compiled = filter::compile_filter(&fields, f)?;
        wheres.push(compiled.sql);
        binds.extend(compiled.params);
    }
    if let Some(q) = &params.q {
        let fts = search::fts_ident(&table)?;
        wheres.push(format!("rec.\"seq\" IN (SELECT rowid FROM {fts} WHERE {fts} MATCH ?)"));
        binds.push(Sql::Text(search::match_expr(q)?));
    }
    let sorts = filter::compile_sort(&fields, params.sort.as_ref())?;
    if let Some(cursor) = &params.cursor {
        let after = filter::after_cursor(&sorts, &decode_cursor(cursor, sorts.len())?);
        wheres.push(after.sql);
        binds.extend(after.params);
    }

    // The sort expressions ride along as extra columns, so the last row can become the next cursor.
    let sort_columns: String = sorts.iter().enumerate().map(|(i, k)| format!(", {} AS _s{i}", k.expr)).collect();
    let where_sql = if wheres.is_empty() { String::new() } else { format!("WHERE {}", wheres.join(" AND ")) };
    // The `rec` alias lets filters on join-table fields refer to the row being tested.
    let sql = format!(
        "SELECT {}{sort_columns} FROM {} AS rec {where_sql} ORDER BY {} LIMIT ?",
        select_list(&fields)?,
        ident(&table.sql_name)?,
        filter::order_by(&sorts),
    );
    // One extra row tells us whether another page exists.
    binds.push(Sql::Integer(params.limit as i64 + 1));

    let sort_offset = 5 + fields.iter().filter(|f| f.has_column()).count();
    let mut stmt = tx.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(binds.iter()), |row| {
        let raw = read_raw(&fields, row)?;
        let sort_values = (0..sorts.len())
            .map(|i| row.get::<_, Sql>(sort_offset + i))
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((raw, sort_values))
    })?;
    let mut rows = rows.collect::<Result<Vec<_>, _>>()?;

    let has_more = rows.len() > params.limit;
    rows.truncate(params.limit);
    let next_cursor = match (has_more, rows.last()) {
        (true, Some((_, sort_values))) => Some(encode_cursor(sort_values)),
        _ => None,
    };

    let ids: Vec<&str> = rows.iter().map(|(raw, _)| raw.id.as_str()).collect();
    let links = load_links(&tx, &fields, &ids)?;
    let records = rows.into_iter().map(|(raw, _)| assemble(&fields, raw, &links)).collect();
    Ok(RecordList { records, next_cursor })
}

pub fn create(
    conn: &mut Connection,
    table_key: &str,
    user_id: &str,
    values: &Map<String, Value>,
) -> Result<Record, AppError> {
    let tx = conn.transaction()?;
    let table = catalog::resolve_table(&tx, table_key)?;
    let fields = catalog::read_fields(&tx, &table.id)?;
    let assigned = assign(&fields, values)?;
    check_targets(&tx, &assigned)?;

    let id = new_id();
    let mut columns = SYSTEM_COLUMNS.to_string();
    let mut placeholders = format!("?, {NOW}, {NOW}, ?, ?");
    let mut binds = vec![Sql::Text(id.clone()), Sql::Text(user_id.into()), Sql::Text(user_id.into())];
    for (field, cell) in &assigned {
        if let Cell::Column(v) = cell {
            columns.push_str(&format!(", {}", ident(&field.sql_name)?));
            placeholders.push_str(", ?");
            binds.push(v.clone());
        }
    }
    tx.execute(
        &format!("INSERT INTO {} ({columns}) VALUES ({placeholders})", ident(&table.sql_name)?),
        params_from_iter(binds.iter()),
    )?;
    write_links(&tx, &id, &assigned)?;

    let record = fetch(&tx, &table, &fields, &id)?;
    tx.commit()?;
    Ok(record)
}

/// Sets the given fields and leaves the rest alone. `null` clears a field.
pub fn update(
    conn: &mut Connection,
    table_key: &str,
    id: &str,
    user_id: &str,
    values: &Map<String, Value>,
) -> Result<Record, AppError> {
    let tx = conn.transaction()?;
    let table = catalog::resolve_table(&tx, table_key)?;
    let fields = catalog::read_fields(&tx, &table.id)?;
    let assigned = assign(&fields, values)?;
    check_targets(&tx, &assigned)?;

    let mut sets = format!("\"updated_at\" = {NOW}, \"updated_by\" = ?");
    let mut binds = vec![Sql::Text(user_id.into())];
    for (field, cell) in &assigned {
        if let Cell::Column(v) = cell {
            sets.push_str(&format!(", {} = ?", ident(&field.sql_name)?));
            binds.push(v.clone());
        }
    }
    binds.push(Sql::Text(id.into()));
    let changed = tx.execute(
        &format!("UPDATE {} SET {sets} WHERE id = ?", ident(&table.sql_name)?),
        params_from_iter(binds.iter()),
    )?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    write_links(&tx, id, &assigned)?;

    let record = fetch(&tx, &table, &fields, id)?;
    tx.commit()?;
    Ok(record)
}

/// Records have no soft-delete column, so this removes the row. Foreign keys take care of links:
/// join-table rows on either side are deleted, and to-one relations that pointed here become empty.
pub fn delete(conn: &mut Connection, table_key: &str, id: &str) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let table = catalog::resolve_table(&tx, table_key)?;
    let changed = tx.execute(&format!("DELETE FROM {} WHERE id = ?1", ident(&table.sql_name)?), [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    tx.commit()?;
    Ok(())
}

/// A cursor is the last row's sort values as JSON, hex-encoded so it is safe in a URL.
fn encode_cursor(values: &[Sql]) -> String {
    let json: Vec<Value> = values
        .iter()
        .map(|v| match v {
            Sql::Integer(i) => Value::from(*i),
            Sql::Real(f) => serde_json::Number::from_f64(*f).map_or(Value::Null, Value::Number),
            Sql::Text(s) => Value::String(s.clone()),
            Sql::Null | Sql::Blob(_) => Value::Null,
        })
        .collect();
    Value::Array(json).to_string().bytes().map(|b| format!("{b:02x}")).collect()
}

fn decode_cursor(cursor: &str, expected: usize) -> Result<Vec<Sql>, AppError> {
    let bad = || AppError::BadRequest("invalid cursor".into());
    if cursor.len() > MAX_CURSOR || cursor.len() % 2 != 0 || !cursor.is_ascii() {
        return Err(bad());
    }
    let bytes = (0..cursor.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&cursor[i..i + 2], 16).ok())
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(bad)?;
    let Value::Array(items) = serde_json::from_slice(&bytes).map_err(|_| bad())? else {
        return Err(bad());
    };
    // A cursor only fits the sort it came from.
    if items.len() != expected {
        return Err(AppError::BadRequest("cursor does not match the requested sort".into()));
    }
    items
        .into_iter()
        .map(|item| match item {
            Value::Null => Ok(Sql::Null),
            Value::String(s) => Ok(Sql::Text(s)),
            Value::Number(n) => Ok(n.as_i64().map(Sql::Integer).or_else(|| n.as_f64().map(Sql::Real)).unwrap_or(Sql::Null)),
            _ => Err(bad()),
        })
        .collect()
}
