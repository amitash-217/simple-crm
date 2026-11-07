//! Saved views: a named filter, sort and layout for one table.
//!
//! A view's `config` is stored with field IDs and option IDs only, so renaming a field or a select
//! option never breaks it. Requests may use names and labels; `normalize` rewrites them to IDs.
//! If a field or option a view uses is later deleted, the view is kept and reported as `stale`
//! until someone edits it.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::{
    auth::{new_id, NOW},
    catalog::{self, FieldOut},
    error::AppError,
    records::{self, field_by_key, option_id},
};

const MAX_VIEWS_PER_TABLE: i64 = 100;
const MAX_HIDDEN: usize = 200;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Grid,
    Kanban,
}

impl Kind {
    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "grid" => Ok(Kind::Grid),
            "kanban" => Ok(Kind::Kanban),
            other => Err(AppError::BadRequest(format!("unknown view kind `{other}`; use `grid` or `kanban`"))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Kind::Grid => "grid",
            Kind::Kanban => "kanban",
        }
    }
}

#[derive(Serialize)]
pub struct ViewOut {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    pub config: Value,
    pub position: i64,
    /// True when the config refers to a field or option that no longer exists.
    pub stale: bool,
    pub created_at: String,
    pub created_by: Option<String>,
}

pub struct NewView {
    pub name: String,
    pub kind: String,
    pub config: Value,
}

pub struct ViewPatch {
    pub name: Option<String>,
    pub config: Option<Value>,
    pub position: Option<i64>,
}

struct Row {
    id: String,
    name: String,
    kind: String,
    config: String,
    position: i64,
    created_at: String,
    created_by: Option<String>,
}

const COLUMNS: &str = "id, name, kind, config_json, position, created_at, created_by";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        name: r.get(1)?,
        kind: r.get(2)?,
        config: r.get(3)?,
        position: r.get(4)?,
        created_at: r.get(5)?,
        created_by: r.get(6)?,
    })
}

fn out(fields: &[FieldOut], r: Row) -> Result<ViewOut, AppError> {
    let kind = Kind::parse(&r.kind).map_err(|_| AppError::Internal(format!("view {} has kind `{}`", r.id, r.kind)))?;
    let config: Value = serde_json::from_str(&r.config)
        .map_err(|e| AppError::Internal(format!("view {} has invalid config: {e}", r.id)))?;
    let stale = normalize(fields, kind, &config).is_err();
    Ok(ViewOut {
        id: r.id,
        name: r.name,
        kind: kind.as_str(),
        config,
        position: r.position,
        stale,
        created_at: r.created_at,
        created_by: r.created_by,
    })
}

/// Validates a view config against the table's current fields and returns it with IDs in place of names.
pub fn normalize(fields: &[FieldOut], kind: Kind, raw: &Value) -> Result<Value, AppError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| AppError::BadRequest("view config must be a JSON object".into()))?;
    if let Some(extra) = obj.keys().find(|k| !matches!(k.as_str(), "filter" | "sort" | "hidden_fields" | "group_by")) {
        return Err(AppError::BadRequest(format!("unknown key `{extra}` in view config")));
    }
    let mut out = Map::new();

    if let Some(filter) = obj.get("filter").filter(|v| !v.is_null()) {
        records::validate_filter(fields, filter)?;
        out.insert("filter".into(), canonical_filter(fields, filter));
    }

    if let Some(sort) = obj.get("sort").filter(|v| !v.is_null()) {
        records::validate_sort(fields, sort)?;
        let items = sort.as_array().map(Vec::as_slice).unwrap_or_default();
        let items = items
            .iter()
            .map(|item| {
                let mut item = item.clone();
                canonical_key(fields, &mut item);
                item
            })
            .collect();
        out.insert("sort".into(), Value::Array(items));
    }

    if let Some(hidden) = obj.get("hidden_fields").filter(|v| !v.is_null()) {
        let items = hidden
            .as_array()
            .ok_or_else(|| AppError::BadRequest("`hidden_fields` must be an array".into()))?;
        if items.len() > MAX_HIDDEN {
            return Err(AppError::BadRequest(format!("`hidden_fields` accepts at most {MAX_HIDDEN} entries")));
        }
        let mut ids: Vec<Value> = Vec::new();
        for item in items {
            let key = item
                .as_str()
                .ok_or_else(|| AppError::BadRequest("`hidden_fields` entries must be strings".into()))?;
            let field = field_by_key(fields, key)
                .ok_or_else(|| AppError::BadRequest(format!("`hidden_fields`: unknown field `{key}`")))?;
            if field.is_primary {
                return Err(AppError::BadRequest(format!(
                    "`hidden_fields`: `{}` is the primary field and cannot be hidden",
                    field.name
                )));
            }
            let id = Value::String(field.id.clone());
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        out.insert("hidden_fields".into(), Value::Array(ids));
    }

    match (kind, obj.get("group_by").filter(|v| !v.is_null())) {
        (Kind::Kanban, Some(key)) => {
            let key = key
                .as_str()
                .ok_or_else(|| AppError::BadRequest("`group_by` must be a field name or id".into()))?;
            let field = field_by_key(fields, key)
                .ok_or_else(|| AppError::BadRequest(format!("`group_by`: unknown field `{key}`")))?;
            if field.field_type != "select" {
                return Err(AppError::BadRequest(format!("`group_by`: `{}` is not a select field", field.name)));
            }
            out.insert("group_by".into(), Value::String(field.id.clone()));
        }
        (Kind::Kanban, None) => {
            return Err(AppError::BadRequest(
                "a kanban view needs `group_by`, the select field that makes its columns".into(),
            ));
        }
        (Kind::Grid, Some(_)) => {
            return Err(AppError::BadRequest("`group_by` applies only to kanban views".into()));
        }
        (Kind::Grid, None) => {}
    }

    Ok(Value::Object(out))
}

/// Replaces field names with field IDs, and select option labels with option IDs, in a filter that
/// has already passed validation. System fields (`$created_at` and so on) are left as they are.
fn canonical_filter(fields: &[FieldOut], node: &Value) -> Value {
    let Some(obj) = node.as_object() else { return node.clone() };
    for group in ["and", "or"] {
        if let Some(items) = obj.get(group).and_then(Value::as_array) {
            return json!({ group: items.iter().map(|i| canonical_filter(fields, i)).collect::<Vec<_>>() });
        }
    }
    let mut cond = node.clone();
    let field = obj.get("field").and_then(Value::as_str).and_then(|k| field_by_key(fields, k));
    canonical_key(fields, &mut cond);
    if let (Some(field), Some(value)) = (field, cond.get_mut("value")) {
        if matches!(field.field_type, "select" | "multi_select") {
            let to_id = |v: &Value| match v.as_str().and_then(|s| option_id(field, s)) {
                Some(id) => Value::String(id),
                None => v.clone(),
            };
            let converted = match &*value {
                Value::Array(items) => Value::Array(items.iter().map(to_id).collect()),
                other => to_id(other),
            };
            *value = converted;
        }
    }
    cond
}

/// Rewrites the `field` key of a filter condition or sort key to a field ID.
fn canonical_key(fields: &[FieldOut], item: &mut Value) {
    let Some(key) = item.get("field").and_then(Value::as_str) else { return };
    if key.starts_with('$') {
        return;
    }
    if let Some(field) = field_by_key(fields, key) {
        item["field"] = Value::String(field.id.clone());
    }
}

/// Finds a view of the table by ID, or else by name (case-insensitive).
fn find(conn: &Connection, table_id: &str, key: &str) -> Result<Row, AppError> {
    let by_id = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM views WHERE table_id = ?1 AND id = ?2"),
            params![table_id, key],
            row,
        )
        .optional()?;
    if let Some(found) = by_id {
        return Ok(found);
    }
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM views WHERE table_id = ?1 AND lower(name) = lower(?2)"),
        params![table_id, key],
        row,
    )
    .optional()?
    .ok_or(AppError::NotFound)
}

fn name_taken(conn: &Connection, table_id: &str, name: &str, except: Option<&str>) -> Result<bool, AppError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM views WHERE table_id = ?1 AND lower(name) = lower(?2) AND id != ?3",
        params![table_id, name, except.unwrap_or("")],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

pub fn list(conn: &Connection, table_key: &str) -> Result<Vec<ViewOut>, AppError> {
    let table = catalog::resolve_table(conn, table_key)?;
    let fields = catalog::read_fields(conn, &table.id)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM views WHERE table_id = ?1 ORDER BY position, created_at, id"
    ))?;
    let rows = stmt.query_map([&table.id], row)?.collect::<Result<Vec<_>, _>>()?;
    rows.into_iter().map(|r| out(&fields, r)).collect()
}

pub fn get(conn: &Connection, table_key: &str, view_key: &str) -> Result<ViewOut, AppError> {
    let table = catalog::resolve_table(conn, table_key)?;
    let fields = catalog::read_fields(conn, &table.id)?;
    out(&fields, find(conn, &table.id, view_key)?)
}

pub fn create(conn: &mut Connection, user_id: &str, table_key: &str, new: NewView) -> Result<ViewOut, AppError> {
    let name = catalog::validate_name(&new.name, "view name")?;
    let kind = Kind::parse(&new.kind)?;
    let tx = conn.transaction()?;
    let table = catalog::resolve_table(&tx, table_key)?;
    let fields = catalog::read_fields(&tx, &table.id)?;
    let config = normalize(&fields, kind, &new.config)?;

    if name_taken(&tx, &table.id, &name, None)? {
        return Err(AppError::Conflict(format!("this table already has a view named `{name}`")));
    }
    let (count, next): (i64, i64) = tx.query_row(
        "SELECT COUNT(*), COALESCE(MAX(position), -1) + 1 FROM views WHERE table_id = ?1",
        [&table.id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if count >= MAX_VIEWS_PER_TABLE {
        return Err(AppError::Conflict(format!("a table can have at most {MAX_VIEWS_PER_TABLE} views")));
    }
    let id = new_id();
    tx.execute(
        &format!(
            "INSERT INTO views (id, table_id, name, kind, config_json, position, created_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, {NOW}, ?7)"
        ),
        params![id, table.id, name, kind.as_str(), config.to_string(), next, user_id],
    )?;
    let view = out(&fields, find(&tx, &table.id, &id)?)?;
    tx.commit()?;
    Ok(view)
}

pub fn update(conn: &mut Connection, table_key: &str, view_key: &str, patch: ViewPatch) -> Result<ViewOut, AppError> {
    let tx = conn.transaction()?;
    let table = catalog::resolve_table(&tx, table_key)?;
    let fields = catalog::read_fields(&tx, &table.id)?;
    let current = find(&tx, &table.id, view_key)?;
    let kind = Kind::parse(&current.kind)?;

    if let Some(name) = patch.name {
        let name = catalog::validate_name(&name, "view name")?;
        if name_taken(&tx, &table.id, &name, Some(&current.id))? {
            return Err(AppError::Conflict(format!("this table already has a view named `{name}`")));
        }
        tx.execute("UPDATE views SET name = ?1 WHERE id = ?2", params![name, current.id])?;
    }
    if let Some(config) = patch.config {
        let config = normalize(&fields, kind, &config)?;
        tx.execute("UPDATE views SET config_json = ?1 WHERE id = ?2", params![config.to_string(), current.id])?;
    }
    if let Some(position) = patch.position {
        if position < 0 {
            return Err(AppError::BadRequest("`position` must not be negative".into()));
        }
        tx.execute("UPDATE views SET position = ?1 WHERE id = ?2", params![position, current.id])?;
    }
    let view = out(&fields, find(&tx, &table.id, &current.id)?)?;
    tx.commit()?;
    Ok(view)
}

/// Views are plain configuration, so deleting one removes the row for good.
pub fn delete(conn: &mut Connection, table_key: &str, view_key: &str) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let table = catalog::resolve_table(&tx, table_key)?;
    let current = find(&tx, &table.id, view_key)?;
    tx.execute("DELETE FROM views WHERE id = ?1", [&current.id])?;
    tx.commit()?;
    Ok(())
}
