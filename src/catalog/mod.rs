//! Catalog of user-defined tables and fields, and the runtime DDL that backs them.
//!
//! Every SQL identifier for a user table or field is generated here from a UUID (`t_<hex>`, `f_<hex>`).
//! User-chosen names live only in catalog rows, so renames never touch DDL and user input never reaches SQL text.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    auth::{new_id, NOW},
    error::AppError,
    search,
};

const MAX_NAME: usize = 100;
const MAX_COLOR: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldType {
    Text,
    Number,
    Boolean,
    Date,
    Phone,
    Email,
    LongText,
    Select,
    MultiSelect,
    Relation,
}

impl FieldType {
    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "boolean" => Ok(Self::Boolean),
            "date" => Ok(Self::Date),
            "phone" => Ok(Self::Phone),
            "email" => Ok(Self::Email),
            "long_text" => Ok(Self::LongText),
            "select" => Ok(Self::Select),
            "multi_select" => Ok(Self::MultiSelect),
            "relation" => Ok(Self::Relation),
            _ => Err(AppError::BadRequest(format!(
                "unknown field type `{s}`; expected text, number, boolean, date, phone, email, long_text, select, multi_select or relation"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::Phone => "phone",
            Self::Email => "email",
            Self::LongText => "long_text",
            Self::Select => "select",
            Self::MultiSelect => "multi_select",
            Self::Relation => "relation",
        }
    }

    /// SQLite type of the column. Multi-select and to-many relations have no column, only a join table.
    fn column_type(self, config: &FieldConfig) -> &'static str {
        match self {
            Self::Number if config.integer == Some(true) => "INTEGER",
            Self::Number => "REAL",
            Self::Boolean => "INTEGER",
            Self::Text | Self::LongText | Self::Email | Self::Date | Self::Phone | Self::Select | Self::MultiSelect | Self::Relation => "TEXT",
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldConfig {
    /// Number fields only: store whole numbers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integer: Option<bool>,
    /// Relation fields only: the target table. Requests give a name or ID; the catalog stores the ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    /// Relation fields only: link to many records instead of one. Defaults to false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub many: Option<bool>,
    /// Date fields only: also store a time of day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<bool>,
    /// Relation fields, on create only: also add a field to the target table that shows the links from the other side.
    /// Defaults to true for to-many relations to another table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_related: Option<bool>,
    /// Stored on relation fields that have a partner on the other table: the partner field's ID. Never accepted in requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related: Option<String>,
    /// Stored on the auto-created partner: it reads the partner's join table from the other end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
    /// Relation fields, on create only: whether the partner field holds many records (true, the default) or one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_many: Option<bool>,
    /// Stored on relation fields: each record on the linked table can be linked from only one record of this field,
    /// because the partner holds at most one. Setting links here moves a record away from its previous owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionInput {
    /// Present when keeping or editing an existing option; absent when adding one.
    pub id: Option<String>,
    pub label: String,
    pub color: Option<String>,
}

#[derive(Serialize)]
pub struct OptionOut {
    pub id: String,
    pub label: String,
    pub color: Option<String>,
    pub position: i64,
}

#[derive(Clone, Serialize)]
pub struct Table {
    pub id: String,
    pub name: String,
    #[serde(skip)]
    pub sql_name: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct TableDetail {
    #[serde(flatten)]
    pub table: Table,
    pub fields: Vec<FieldOut>,
}

#[derive(Serialize)]
pub struct FieldOut {
    pub id: String,
    pub name: String,
    #[serde(skip)]
    pub sql_name: String,
    #[serde(rename = "type")]
    pub field_type: &'static str,
    pub is_primary: bool,
    pub position: i64,
    pub config: Value,
    pub options: Vec<OptionOut>,
}

impl FieldOut {
    pub fn many_relation(&self) -> bool {
        self.field_type == "relation" && self.config.get("many").and_then(Value::as_bool) == Some(true)
    }

    /// For a join-table field: the columns that hold this field's own record and the linked value.
    /// A partner field created for the other side of a relation shares the join table, with the columns swapped.
    pub fn link_columns(&self) -> (&'static str, &'static str) {
        if self.field_type == "multi_select" {
            ("record_id", "option_id")
        } else if self.config.get("reverse").and_then(Value::as_bool) == Some(true) {
            ("target_id", "record_id")
        } else {
            ("record_id", "target_id")
        }
    }

    /// The partner field on the other table, if this relation has one.
    pub fn related_id(&self) -> Option<&str> {
        self.config.get("related").and_then(Value::as_str)
    }

    /// True for fields stored in a join table: multi-select, to-many relations, and any relation that has a partner.
    pub fn in_join_table(&self) -> bool {
        self.field_type == "multi_select"
            || (self.field_type == "relation" && (self.many_relation() || self.related_id().is_some()))
    }

    /// A relation in a join table that holds at most one record: its value is an ID or `null`, not an array.
    pub fn single_link(&self) -> bool {
        self.field_type == "relation" && !self.many_relation() && self.in_join_table()
    }

    /// Each linked record can belong to only one record of this field.
    pub fn exclusive(&self) -> bool {
        self.config.get("exclusive").and_then(Value::as_bool) == Some(true)
    }

    /// False for fields stored in a join table instead of a column.
    pub fn has_column(&self) -> bool {
        !self.in_join_table()
    }
}

pub struct NewField {
    pub name: String,
    pub field_type: String,
    pub config: FieldConfig,
    pub options: Option<Vec<OptionInput>>,
}

#[derive(Default)]
pub struct FieldPatch {
    pub name: Option<String>,
    pub options: Option<Vec<OptionInput>>,
    /// A new type, or the same type with new `integer` / `time` settings.
    pub field_type: Option<String>,
    pub config: Option<FieldConfig>,
}

/// Returns `sql_name` double-quoted for use in SQL text, after checking it is one of our generated identifiers.
pub fn ident(sql_name: &str) -> Result<String, AppError> {
    let ok = sql_name.is_ascii()
        && sql_name.len() == 34
        && matches!(&sql_name[..2], "t_" | "f_" | "m_" | "r_" | "i_" | "s_" | "a_" | "d_" | "u_")
        && sql_name[2..].bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if ok {
        Ok(format!("\"{sql_name}\""))
    } else {
        Err(AppError::Internal(format!("invalid generated identifier `{sql_name}`")))
    }
}

fn sql_name(prefix: &str, id: &str) -> String {
    format!("{prefix}_{}", id.replace('-', ""))
}

pub(crate) fn validate_name(raw: &str, what: &str) -> Result<String, AppError> {
    let name = raw.trim();
    let len = name.chars().count();
    if len == 0 || len > MAX_NAME || name.chars().any(char::is_control) {
        return Err(AppError::BadRequest(format!(
            "{what} must be 1 to {MAX_NAME} characters with no control characters"
        )));
    }
    Ok(name.to_string())
}

fn table_from_row(r: &Row) -> rusqlite::Result<Table> {
    Ok(Table { id: r.get(0)?, name: r.get(1)?, sql_name: r.get(2)?, created_at: r.get(3)? })
}

/// Finds a live table by ID, or else by name (case-insensitive).
pub fn resolve_table(conn: &Connection, key: &str) -> Result<Table, AppError> {
    let by_id = conn
        .query_row(
            "SELECT id, name, sql_name, created_at FROM tables WHERE deleted_at IS NULL AND id = ?1",
            [key],
            table_from_row,
        )
        .optional()?;
    if let Some(table) = by_id {
        return Ok(table);
    }
    conn.query_row(
        "SELECT id, name, sql_name, created_at FROM tables
         WHERE deleted_at IS NULL AND lower(name) = lower(?1)",
        [key],
        table_from_row,
    )
    .optional()?
    .ok_or(AppError::NotFound)
}

pub fn list_tables(conn: &Connection) -> Result<Vec<Table>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, sql_name, created_at FROM tables WHERE deleted_at IS NULL ORDER BY created_at, id",
    )?;
    let rows = stmt.query_map([], table_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_table(conn: &Connection, key: &str) -> Result<TableDetail, AppError> {
    let table = resolve_table(conn, key)?;
    let fields = read_fields(conn, &table.id)?;
    Ok(TableDetail { table, fields })
}

fn table_name_taken(conn: &Connection, name: &str, except_id: &str) -> Result<bool, AppError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tables WHERE deleted_at IS NULL AND lower(name) = lower(?1) AND id != ?2",
        params![name, except_id],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

fn field_name_taken(conn: &Connection, table_id: &str, name: &str, except_id: &str) -> Result<bool, AppError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM fields
         WHERE table_id = ?1 AND deleted_at IS NULL AND lower(name) = lower(?2) AND id != ?3",
        params![table_id, name, except_id],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

/// Creates the catalog row and the backing SQLite table, with a primary text field named `Name`.
pub fn create_table(conn: &mut Connection, user_id: &str, name: &str) -> Result<TableDetail, AppError> {
    let name = validate_name(name, "table name")?;
    let tx = conn.transaction()?;
    if table_name_taken(&tx, &name, "")? {
        return Err(AppError::Conflict(format!("a table named `{name}` already exists")));
    }

    let id = new_id();
    let table_sql = sql_name("t", &id);
    let field_id = new_id();
    let field_sql = sql_name("f", &field_id);

    tx.execute(
        &format!("INSERT INTO tables (id, name, sql_name, created_at, created_by) VALUES (?1, ?2, ?3, {NOW}, ?4)"),
        params![id, name, table_sql, user_id],
    )?;
    // created_by and updated_by are plain TEXT, not foreign keys, so deleting a user never blocks on their records.
    // `seq` is an explicit integer key for the full-text index, which needs a rowid that never changes.
    tx.execute_batch(&format!(
        "CREATE TABLE {} (
            id         TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            created_by TEXT,
            updated_by TEXT,
            {} TEXT,
            seq        INTEGER PRIMARY KEY
        )",
        ident(&table_sql)?,
        ident(&field_sql)?,
    ))?;
    tx.execute(
        &format!(
            "INSERT INTO fields (id, table_id, name, sql_name, type, config_json, position, is_primary, created_at)
             VALUES (?1, ?2, 'Name', ?3, 'text', '{{}}', 0, 1, {NOW})"
        ),
        params![field_id, id, field_sql],
    )?;

    let detail = get_table(&tx, &id)?;
    search::rebuild_index(&tx, &detail.table, &detail.fields)?;
    tx.commit()?;
    Ok(detail)
}

/// Renames only the catalog row. The SQL table keeps its generated name.
pub fn rename_table(conn: &mut Connection, key: &str, name: &str) -> Result<TableDetail, AppError> {
    let name = validate_name(name, "table name")?;
    let tx = conn.transaction()?;
    let table = resolve_table(&tx, key)?;
    if table_name_taken(&tx, &name, &table.id)? {
        return Err(AppError::Conflict(format!("a table named `{name}` already exists")));
    }
    tx.execute("UPDATE tables SET name = ?1 WHERE id = ?2", params![name, table.id])?;
    let detail = get_table(&tx, &table.id)?;
    tx.commit()?;
    Ok(detail)
}

/// Soft delete: the SQL table and its data stay in the file.
pub fn delete_table(conn: &mut Connection, key: &str) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let table = resolve_table(&tx, key)?;
    tx.execute(&format!("UPDATE tables SET deleted_at = {NOW} WHERE id = ?1"), [&table.id])?;
    // Partners of this table's relation fields would point at a deleted table.
    tx.execute(
        &format!(
            "UPDATE fields SET deleted_at = {NOW} WHERE deleted_at IS NULL AND id IN
             (SELECT json_extract(config_json, '$.related') FROM fields WHERE table_id = ?1 AND deleted_at IS NULL)"
        ),
        [&table.id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn read_fields(conn: &Connection, table_id: &str) -> Result<Vec<FieldOut>, AppError> {
    let mut options: HashMap<String, Vec<OptionOut>> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT o.field_id, o.id, o.label, o.color, o.position
         FROM field_options o JOIN fields f ON f.id = o.field_id
         WHERE f.table_id = ?1 AND f.deleted_at IS NULL ORDER BY o.position",
    )?;
    let rows = stmt.query_map([table_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            OptionOut { id: r.get(1)?, label: r.get(2)?, color: r.get(3)?, position: r.get(4)? },
        ))
    })?;
    for row in rows {
        let (field_id, option) = row?;
        options.entry(field_id).or_default().push(option);
    }

    let mut stmt = conn.prepare(
        "SELECT id, name, sql_name, type, config_json, position, is_primary
         FROM fields WHERE table_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows = stmt.query_map([table_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, i64>(6)?,
        ))
    })?;
    let mut fields = Vec::new();
    for row in rows {
        let (id, name, sql_name, ty, config, position, is_primary) = row?;
        let field_type = FieldType::parse(&ty).map_err(|e| AppError::Internal(e.to_string()))?.as_str();
        let config = serde_json::from_str(&config).map_err(|e| AppError::Internal(e.to_string()))?;
        fields.push(FieldOut {
            options: options.remove(&id).unwrap_or_default(),
            id,
            name,
            sql_name,
            field_type,
            is_primary: is_primary != 0,
            position,
            config,
        });
    }
    Ok(fields)
}

/// The live table a relation field points to.
pub fn relation_target(conn: &Connection, field: &FieldOut) -> Result<Table, AppError> {
    let id = field
        .config
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Internal(format!("relation field `{}` has no target", field.name)))?;
    conn.query_row(
        "SELECT id, name, sql_name, created_at FROM tables WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        table_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::BadRequest(format!("field `{}`: the linked table no longer exists", field.name)))
}

fn resolve_field(conn: &Connection, table_id: &str, key: &str) -> Result<FieldOut, AppError> {
    let mut fields = read_fields(conn, table_id)?;
    let by_id = fields.iter().position(|f| f.id == key);
    let index = by_id
        .or_else(|| fields.iter().position(|f| f.name.to_lowercase() == key.to_lowercase()))
        .ok_or(AppError::NotFound)?;
    Ok(fields.swap_remove(index))
}

pub fn list_fields(conn: &Connection, table_key: &str) -> Result<Vec<FieldOut>, AppError> {
    let table = resolve_table(conn, table_key)?;
    read_fields(conn, &table.id)
}

/// Checks labels and colors, and returns the cleaned options.
fn validate_options(input: &[OptionInput]) -> Result<Vec<(Option<&str>, String, Option<&str>)>, AppError> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(input.len());
    for option in input {
        let label = validate_name(&option.label, "option label")?;
        if !seen.insert(label.to_lowercase()) {
            return Err(AppError::BadRequest(format!("duplicate option label `{label}`")));
        }
        if let Some(color) = &option.color {
            if color.chars().count() > MAX_COLOR || color.chars().any(char::is_control) {
                return Err(AppError::BadRequest(format!(
                    "option color must be at most {MAX_COLOR} characters"
                )));
            }
        }
        out.push((option.id.as_deref(), label, option.color.as_deref()));
    }
    Ok(out)
}

/// Makes the stored options match `input`: updates listed IDs, adds entries without an ID, and removes the rest.
/// For a select field (`clears_column`), records that used a removed option get that field set to NULL.
/// For a multi-select field, the foreign key on its join table drops the links.
fn sync_options(
    tx: &Transaction,
    table_sql: &str,
    field_id: &str,
    field_sql: &str,
    clears_column: bool,
    existing: &[OptionOut],
    input: &[OptionInput],
) -> Result<(), AppError> {
    let cleaned = validate_options(input)?;
    let known: HashSet<&str> = existing.iter().map(|o| o.id.as_str()).collect();
    let mut kept = HashSet::new();

    for (position, (id, label, color)) in cleaned.into_iter().enumerate() {
        match id {
            Some(id) => {
                if !known.contains(id) {
                    return Err(AppError::BadRequest(format!("unknown option id `{id}`")));
                }
                if !kept.insert(id) {
                    return Err(AppError::BadRequest(format!("option id `{id}` listed twice")));
                }
                tx.execute(
                    "UPDATE field_options SET label = ?1, color = ?2, position = ?3 WHERE id = ?4",
                    params![label, color, position as i64, id],
                )?;
            }
            None => {
                tx.execute(
                    "INSERT INTO field_options (id, field_id, label, color, position) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![new_id(), field_id, label, color, position as i64],
                )?;
            }
        }
    }

    for old in existing.iter().filter(|o| !kept.contains(o.id.as_str())) {
        if clears_column {
            let (table, column) = (ident(table_sql)?, ident(field_sql)?);
            tx.execute(&format!("UPDATE {table} SET {column} = NULL WHERE {column} = ?1"), [&old.id])?;
        }
        tx.execute("DELETE FROM field_options WHERE id = ?1", [&old.id])?;
    }
    Ok(())
}

/// Creates what stores the field's values: a column, or a join table for multi-select and to-many relations.
struct Links {
    /// A join table, not a column.
    joined: bool,
    /// Each record of this table links at most one record.
    one_per_record: bool,
    /// Each linked record is linked from at most one record of this table.
    one_per_target: bool,
}

fn create_storage(
    tx: &Transaction,
    table: &Table,
    field_sql: &str,
    field_type: FieldType,
    config: &FieldConfig,
    target: Option<&Table>,
    links: &Links,
) -> Result<(), AppError> {
    let (t, f) = (ident(&table.sql_name)?, ident(field_sql)?);
    let join = |other_column: &str, other_ref: &str, unique_other: bool| -> Result<String, AppError> {
        // The index names reuse the field's hex ID, so they are as safe as the other identifiers.
        let index = ident(&format!("i_{}", &field_sql[2..]))?;
        let unique = if unique_other { "UNIQUE " } else { "" };
        let mut sql = format!(
            "CREATE TABLE {f} (
                record_id TEXT NOT NULL REFERENCES {t}(id) ON DELETE CASCADE,
                {other_column} TEXT NOT NULL REFERENCES {other_ref} ON DELETE CASCADE,
                position INTEGER NOT NULL,
                PRIMARY KEY (record_id, {other_column})
            ) WITHOUT ROWID;
            CREATE {unique}INDEX {index} ON {f}({other_column});"
        );
        if links.one_per_record && other_column == "target_id" {
            let own = ident(&format!("u_{}", &field_sql[2..]))?;
            sql.push_str(&format!("CREATE UNIQUE INDEX {own} ON {f}(record_id);"));
        }
        Ok(sql)
    };
    match (field_type, target) {
        (FieldType::MultiSelect, _) => tx.execute_batch(&join("option_id", "field_options(id)", false)?)?,
        (FieldType::Relation, Some(target)) => {
            let target_ref = format!("{}(id)", ident(&target.sql_name)?);
            if links.joined {
                tx.execute_batch(&join("target_id", &target_ref, links.one_per_target)?)?;
            } else {
                // A new column with a REFERENCES clause must default to NULL, which is the default here.
                tx.execute_batch(&format!(
                    "ALTER TABLE {t} ADD COLUMN {f} TEXT REFERENCES {target_ref} ON DELETE SET NULL"
                ))?;
            }
        }
        (FieldType::Relation, None) => {
            return Err(AppError::Internal("relation field without a target table".into()))
        }
        _ => tx.execute_batch(&format!("ALTER TABLE {t} ADD COLUMN {f} {}", field_type.column_type(config)))?,
    }
    Ok(())
}

/// Adds the catalog row and the SQL column in one transaction.
pub fn add_field(conn: &mut Connection, table_key: &str, new: NewField) -> Result<FieldOut, AppError> {
    let name = validate_name(&new.name, "field name")?;
    let field_type = FieldType::parse(&new.field_type)?;
    if new.config.integer.is_some() && field_type != FieldType::Number {
        return Err(AppError::BadRequest("`integer` applies only to number fields".into()));
    }
    if new.config.related.is_some() || new.config.reverse.is_some() || new.config.exclusive.is_some() {
        return Err(AppError::BadRequest("`related`, `reverse` and `exclusive` are set by the server".into()));
    }
    if (new.config.create_related.is_some() || new.config.related_many.is_some()) && field_type != FieldType::Relation {
        return Err(AppError::BadRequest("`create_related` and `related_many` apply only to relation fields".into()));
    }
    if new.config.time.is_some() && field_type != FieldType::Date {
        return Err(AppError::BadRequest("`time` applies only to date fields".into()));
    }
    if (new.config.table.is_some() || new.config.many.is_some()) && field_type != FieldType::Relation {
        return Err(AppError::BadRequest("`table` and `many` apply only to relation fields".into()));
    }
    if new.options.is_some() && !matches!(field_type, FieldType::Select | FieldType::MultiSelect) {
        return Err(AppError::BadRequest("`options` apply only to select and multi_select fields".into()));
    }

    let tx = conn.transaction()?;
    let table = resolve_table(&tx, table_key)?;
    if field_name_taken(&tx, &table.id, &name, "")? {
        return Err(AppError::Conflict(format!("a field named `{name}` already exists in this table")));
    }

    let id = new_id();
    let many = new.config.many == Some(true);
    // For a relation, store the target's ID, so renaming the target table does not break the link.
    let mut stored = FieldConfig { integer: new.config.integer, time: new.config.time, ..Default::default() };
    let mut partner: Option<(String, Table, bool)> = None;
    let mut target = None;
    if field_type == FieldType::Relation {
        let key = new
            .config
            .table
            .as_deref()
            .ok_or_else(|| AppError::BadRequest("relation fields need `config.table`".into()))?;
        let found = resolve_table(&tx, key).map_err(|e| match e {
            AppError::NotFound => AppError::BadRequest(format!("relation target table `{key}` does not exist")),
            other => other,
        })?;
        let wants_partner = match new.config.create_related {
            Some(true) if found.id == table.id => {
                return Err(AppError::BadRequest("`create_related` needs a relation to a different table".into()))
            }
            Some(want) => want,
            None => (many || new.config.related_many.is_some()) && found.id != table.id,
        };
        if !wants_partner && new.config.related_many.is_some() {
            return Err(AppError::BadRequest("`related_many` needs a related field".into()));
        }
        stored.table = Some(found.id.clone());
        stored.many = Some(many);
        if wants_partner {
            let partner_id = new_id();
            let related_many = new.config.related_many.unwrap_or(true);
            stored.related = Some(partner_id.clone());
            // If the partner holds at most one record, each record here can be linked from only one of its records.
            stored.exclusive = (!related_many).then_some(true);
            partner = Some((partner_id, found.clone(), related_many));
        }
        target = Some(found);
    }

    // The SQL name says what stores the values: a column (f), a multi-select join table (m), or a relation join table (r).
    let joined = field_type == FieldType::Relation && (many || partner.is_some());
    let prefix = match field_type {
        FieldType::MultiSelect => "m",
        FieldType::Relation if joined => "r",
        _ => "f",
    };
    let field_sql = sql_name(prefix, &id);
    let links = Links { joined, one_per_record: joined && !many, one_per_target: stored.exclusive == Some(true) };
    create_storage(&tx, &table, &field_sql, field_type, &new.config, target.as_ref(), &links)?;
    let config = serde_json::to_string(&stored).map_err(|e| AppError::Internal(e.to_string()))?;
    tx.execute(
        &format!(
            "INSERT INTO fields (id, table_id, name, sql_name, type, config_json, position, is_primary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6,
                     (SELECT COALESCE(MAX(position), -1) + 1 FROM fields WHERE table_id = ?2), 0, {NOW})"
        ),
        params![id, table.id, name, field_sql, field_type.as_str(), config],
    )?;
    if let Some((partner_id, other, related_many)) = &partner {
        // The partner has no storage of its own: it reads this field's join table from the other end.
        let mut label = table.name.clone();
        let mut n = 1;
        while field_name_taken(&tx, &other.id, &label, "")? {
            n += 1;
            label = format!("{} {n}", table.name);
        }
        let back = FieldConfig {
            table: Some(table.id.clone()),
            many: Some(*related_many),
            related: Some(id.clone()),
            reverse: Some(true),
            exclusive: (!many).then_some(true),
            ..Default::default()
        };
        tx.execute(
            &format!(
                "INSERT INTO fields (id, table_id, name, sql_name, type, config_json, position, is_primary, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'relation', ?5,
                         (SELECT COALESCE(MAX(position), -1) + 1 FROM fields WHERE table_id = ?2), 0, {NOW})"
            ),
            params![
                partner_id,
                other.id,
                label,
                field_sql,
                serde_json::to_string(&back).map_err(|e| AppError::Internal(e.to_string()))?
            ],
        )?;
    }
    if let Some(options) = &new.options {
        let clears_column = field_type == FieldType::Select;
        sync_options(&tx, &table.sql_name, &id, &field_sql, clears_column, &[], options)?;
    }

    // A new text, long text, email or phone field changes what the search index covers.
    if matches!(field_type, FieldType::Text | FieldType::LongText | FieldType::Email | FieldType::Phone) {
        search::rebuild_index(&tx, &table, &read_fields(&tx, &table.id)?)?;
    }

    let field = resolve_field(&tx, &table.id, &id)?;
    tx.commit()?;
    Ok(field)
}

/// Renames a field and/or replaces its select options. Neither runs DDL.
pub fn update_field(
    conn: &mut Connection,
    table_key: &str,
    field_key: &str,
    patch: FieldPatch,
) -> Result<FieldOut, AppError> {
    let tx = conn.transaction()?;
    let table = resolve_table(&tx, table_key)?;
    let field = resolve_field(&tx, &table.id, field_key)?;

    if let Some(name) = &patch.name {
        let name = validate_name(name, "field name")?;
        if field_name_taken(&tx, &table.id, &name, &field.id)? {
            return Err(AppError::Conflict(format!("a field named `{name}` already exists in this table")));
        }
        tx.execute("UPDATE fields SET name = ?1 WHERE id = ?2", params![name, field.id])?;
    }
    if let Some(options) = &patch.options {
        if field.field_type != "select" && field.field_type != "multi_select" {
            return Err(AppError::BadRequest("`options` apply only to select and multi_select fields".into()));
        }
        let clears_column = field.field_type == "select";
        sync_options(&tx, &table.sql_name, &field.id, &field.sql_name, clears_column, &field.options, options)?;
    }

    if patch.field_type.is_some() || patch.config.is_some() {
        change_type(&tx, &table, &field, patch.field_type.as_deref(), patch.config.as_ref())?;
    }

    let updated = resolve_field(&tx, &table.id, &field.id)?;
    tx.commit()?;
    Ok(updated)
}

const TEXT_LIKE: [&str; 4] = ["text", "long_text", "email", "phone"];

/// Converts one stored value for a new field type. Everything goes through its text form.
fn convert(from: &FieldOut, to: FieldType, config: &FieldConfig, v: rusqlite::types::Value) -> Result<rusqlite::types::Value, String> {
    use rusqlite::types::Value as Sql;
    let text = match v {
        Sql::Null | Sql::Blob(_) => return Ok(Sql::Null),
        Sql::Integer(i) => i.to_string(),
        Sql::Real(f) => f.to_string(),
        Sql::Text(s) if from.field_type == "select" => match from.options.iter().find(|o| o.id == s) {
            Some(o) => o.label.clone(),
            None => return Ok(Sql::Null),
        },
        Sql::Text(s) => s,
    };
    use crate::records::value as v;
    match to {
        FieldType::Text if text.chars().count() > 100_000 => Err("text is longer than 100000 characters".into()),
        FieldType::Text | FieldType::LongText => Ok(Sql::Text(text)),
        FieldType::Email if v::valid_email(&text) => Ok(Sql::Text(text.trim().to_string())),
        FieldType::Email => Err(format!("`{text}` is not an email address")),
        FieldType::Phone => v::normalize_phone(&text).map(Sql::Text).map_err(|m| format!("`{text}`: {m}")),
        FieldType::Date if v::valid_date(&text) => Ok(Sql::Text(text)),
        FieldType::Date => Err(format!("`{text}` is not an ISO 8601 date")),
        FieldType::Number => {
            let f: f64 = text.trim().parse().map_err(|_| format!("`{text}` is not a number"))?;
            if !f.is_finite() {
                return Err(format!("`{text}` is not a number"));
            }
            if config.integer == Some(true) {
                if f.fract() != 0.0 || f.abs() >= 9.0e18 {
                    return Err(format!("`{text}` is not a whole number"));
                }
                Ok(Sql::Integer(f as i64))
            } else {
                Ok(Sql::Real(f))
            }
        }
        _ => Err("unsupported conversion".into()),
    }
}

/// Changes a field's type, or its `integer` / `time` setting. The values move into a new column, so nothing
/// is lost if a value does not convert: the whole change is refused and rolled back instead.
fn change_type(
    tx: &Transaction,
    table: &Table,
    field: &FieldOut,
    new_type: Option<&str>,
    new_config: Option<&FieldConfig>,
) -> Result<(), AppError> {
    let to = match new_type {
        Some(s) => FieldType::parse(s)?,
        None => FieldType::parse(field.field_type)?,
    };
    let mut cfg = FieldConfig::default();
    if let Some(given) = new_config {
        if given.table.is_some() || given.many.is_some() || given.create_related.is_some() || given.related_many.is_some()
            || given.related.is_some() || given.reverse.is_some() || given.exclusive.is_some()
        {
            return Err(AppError::BadRequest("only `integer` and `time` can be changed in a field's config".into()));
        }
        if given.integer.is_some() && to != FieldType::Number {
            return Err(AppError::BadRequest("`integer` applies only to number fields".into()));
        }
        if given.time.is_some() && to != FieldType::Date {
            return Err(AppError::BadRequest("`time` applies only to date fields".into()));
        }
        cfg.integer = given.integer;
        cfg.time = given.time;
    } else {
        // Keep the settings that still make sense for the type.
        cfg.integer = (to == FieldType::Number).then(|| field.config.get("integer").and_then(Value::as_bool)).flatten();
        cfg.time = (to == FieldType::Date).then(|| field.config.get("time").and_then(Value::as_bool)).flatten();
    }
    let same_type = to.as_str() == field.field_type;
    let old_integer = field.config.get("integer").and_then(Value::as_bool) == Some(true);
    let old_time = field.config.get("time").and_then(Value::as_bool) == Some(true);
    if same_type && (cfg.integer == Some(true)) == old_integer && (cfg.time == Some(true)) == old_time {
        return Ok(());
    }
    let json = serde_json::to_string(&cfg).map_err(|e| AppError::Internal(e.to_string()))?;

    // Showing or hiding the time of day changes only how the value is displayed.
    if same_type && to == FieldType::Date {
        tx.execute("UPDATE fields SET config_json = ?1 WHERE id = ?2", params![json, field.id])?;
        return Ok(());
    }

    let from_ok = TEXT_LIKE.contains(&field.field_type) || matches!(field.field_type, "number" | "date" | "select");
    let to_ok = TEXT_LIKE.contains(&to.as_str()) || matches!(to, FieldType::Number | FieldType::Date);
    if !from_ok || !to_ok {
        return Err(AppError::BadRequest(format!(
            "cannot change a {} field to {}",
            field.field_type.replace('_', " "),
            to.as_str().replace('_', " ")
        )));
    }
    if field.is_primary && !TEXT_LIKE.contains(&to.as_str()) {
        return Err(AppError::BadRequest("the primary field must stay a text, long text, email or phone field".into()));
    }

    let (t, old_col) = (ident(&table.sql_name)?, ident(&field.sql_name)?);
    let new_sql = sql_name("f", &new_id());
    let new_col = ident(&new_sql)?;
    tx.execute_batch(&format!("ALTER TABLE {t} ADD COLUMN {new_col} {}", to.column_type(&cfg)))?;

    let rows: Vec<(String, rusqlite::types::Value)> = {
        let mut stmt = tx.prepare(&format!("SELECT id, {old_col} FROM {t}"))?;
        let mapped = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        mapped.collect::<Result<_, _>>()?
    };
    let (mut bad, mut first) = (0usize, String::new());
    let mut converted = Vec::new();
    for (id, v) in rows {
        match convert(field, to, &cfg, v) {
            Ok(rusqlite::types::Value::Null) => {}
            Ok(v) => converted.push((id, v)),
            Err(e) => {
                bad += 1;
                if first.is_empty() {
                    first = e;
                }
            }
        }
    }
    if bad > 0 {
        return Err(AppError::BadRequest(format!(
            "{bad} value{} cannot be converted to {}: {first}. Fix or clear them first.",
            if bad == 1 { "" } else { "s" },
            to.as_str().replace('_', " ")
        )));
    }
    for (id, v) in converted {
        tx.execute(&format!("UPDATE {t} SET {new_col} = ?1 WHERE id = ?2"), params![v, id])?;
    }
    tx.execute(
        "UPDATE fields SET type = ?1, sql_name = ?2, config_json = ?3 WHERE id = ?4",
        params![to.as_str(), new_sql, json, field.id],
    )?;
    if field.field_type == "select" {
        tx.execute("DELETE FROM field_options WHERE field_id = ?1", [&field.id])?;
    }
    if TEXT_LIKE.contains(&field.field_type) || TEXT_LIKE.contains(&to.as_str()) {
        search::rebuild_index(tx, table, &read_fields(tx, &table.id)?)?;
    }
    Ok(())
}

/// Soft delete: the SQL column and its data stay in the file. The primary field cannot be deleted.
pub fn delete_field(conn: &mut Connection, table_key: &str, field_key: &str) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let table = resolve_table(&tx, table_key)?;
    let field = resolve_field(&tx, &table.id, field_key)?;
    if field.is_primary {
        return Err(AppError::BadRequest("the primary field cannot be deleted".into()));
    }
    tx.execute(&format!("UPDATE fields SET deleted_at = {NOW} WHERE id = ?1"), [&field.id])?;
    // The partner on the other table has nothing left to show, so it goes too.
    if let Some(partner) = field.config.get("related").and_then(Value::as_str) {
        tx.execute(&format!("UPDATE fields SET deleted_at = {NOW} WHERE id = ?1 AND deleted_at IS NULL"), [partner])?;
    }
    // The column stays in the table, but its text must stop matching searches.
    if matches!(field.field_type, "text" | "long_text" | "email" | "phone") {
        search::rebuild_index(&tx, &table, &read_fields(&tx, &table.id)?)?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::db::Db;

    /// A database holding the user `u1`, because `tables.created_by` is a foreign key.
    fn db() -> (Arc<Db>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("test.db")).unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
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

    fn option(label: &str) -> OptionInput {
        OptionInput { id: None, label: label.into(), color: None }
    }

    /// Column names of an SQL table, as SQLite itself reports them.
    fn columns(conn: &Connection, table_sql: &str) -> Vec<(String, String)> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", ident(table_sql).unwrap())).unwrap();
        let rows = stmt.query_map([], |r| Ok((r.get(1)?, r.get(2)?))).unwrap();
        rows.map(Result::unwrap).collect()
    }

    #[tokio::test]
    async fn create_table_builds_sql_table_with_primary_field() {
        let (db, _dir) = db();
        let detail = db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();

        assert_eq!(detail.table.name, "Deals");
        assert_eq!(detail.fields.len(), 1);
        assert_eq!(detail.fields[0].name, "Name");
        assert!(detail.fields[0].is_primary);

        let (table_sql, field_sql) = (detail.table.sql_name.clone(), detail.fields[0].sql_name.clone());
        let cols = db.read(move |c| Ok(columns(c, &table_sql))).await.unwrap();
        let names: Vec<&str> = cols.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names[..5], ["id", "created_at", "updated_at", "created_by", "updated_by"]);
        assert_eq!(names[5], field_sql);
    }

    #[tokio::test]
    async fn table_names_are_unique_case_insensitively_among_live_tables() {
        let (db, _dir) = db();
        db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
        let dup = db.write(|c| create_table(c, "u1", "  deals ")).await;
        assert!(matches!(dup, Err(AppError::Conflict(_))));

        db.write(|c| delete_table(c, "deals")).await.unwrap();
        db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
    }

    #[tokio::test]
    async fn rename_changes_catalog_only() {
        let (db, _dir) = db();
        let before = db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
        let after = db.write(|c| rename_table(c, "Deals", "Opportunities")).await.unwrap();

        assert_eq!(after.table.id, before.table.id);
        assert_eq!(after.table.name, "Opportunities");
        assert_eq!(after.table.sql_name, before.table.sql_name);
        assert!(matches!(db.read(|c| resolve_table(c, "Deals")).await, Err(AppError::NotFound)));
    }

    #[tokio::test]
    async fn delete_table_is_soft() {
        let (db, _dir) = db();
        let detail = db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
        db.write(|c| delete_table(c, "Deals")).await.unwrap();

        assert!(matches!(db.read(|c| resolve_table(c, "Deals")).await, Err(AppError::NotFound)));
        assert!(db.read(|c| list_tables(c)).await.unwrap().is_empty());

        let table_sql = detail.table.sql_name.clone();
        let cols = db.read(move |c| Ok(columns(c, &table_sql))).await.unwrap();
        assert!(!cols.is_empty(), "SQL table must survive a soft delete");
    }

    #[tokio::test]
    async fn add_field_runs_alter_table_with_the_right_column_type() {
        let (db, _dir) = db();
        let table = db.write(|c| create_table(c, "u1", "Deals")).await.unwrap().table;
        let mut integer = new_field("Count", "number");
        integer.config.integer = Some(true);
        let fields = db
            .write(|c| {
                Ok(vec![
                    add_field(c, "Deals", new_field("Amount", "number"))?,
                    add_field(c, "Deals", integer)?,
                    add_field(c, "Deals", new_field("Won", "boolean"))?,
                    add_field(c, "Deals", new_field("Closes", "date"))?,
                    add_field(c, "Deals", new_field("Mobile", "phone"))?,
                    add_field(c, "Deals", new_field("Notes", "text"))?,
                ])
            })
            .await
            .unwrap();

        let table_sql = table.sql_name.clone();
        let cols = db.read(move |c| Ok(columns(c, &table_sql))).await.unwrap();
        let type_of = |f: &FieldOut| cols.iter().find(|(n, _)| *n == f.sql_name).unwrap().1.clone();
        assert_eq!(type_of(&fields[0]), "REAL");
        assert_eq!(type_of(&fields[1]), "INTEGER");
        assert_eq!(type_of(&fields[2]), "INTEGER");
        assert_eq!(type_of(&fields[3]), "TEXT");
        assert_eq!(type_of(&fields[5]), "TEXT");
        assert_eq!(fields[1].config["integer"], true);

        let listed = db.read(|c| list_fields(c, "Deals")).await.unwrap();
        let positions: Vec<i64> = listed.iter().map(|f| f.position).collect();
        assert_eq!(positions, [0, 1, 2, 3, 4, 5, 6]);
    }

    #[tokio::test]
    async fn add_field_rejects_bad_input() {
        let (db, _dir) = db();
        db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();

        let dup = db.write(|c| add_field(c, "Deals", new_field("name", "text"))).await;
        assert!(matches!(dup, Err(AppError::Conflict(_))));
        // A relation without a target table is invalid too.
        for ty in ["relation", "bogus"] {
            let r = db.write(move |c| add_field(c, "Deals", new_field("X", ty))).await;
            assert!(matches!(r, Err(AppError::BadRequest(_))), "type {ty}");
        }
        let mut with_integer = new_field("X", "text");
        with_integer.config.integer = Some(true);
        assert!(matches!(db.write(|c| add_field(c, "Deals", with_integer)).await, Err(AppError::BadRequest(_))));
        let mut with_options = new_field("X", "text");
        with_options.options = Some(vec![option("a")]);
        assert!(matches!(db.write(|c| add_field(c, "Deals", with_options)).await, Err(AppError::BadRequest(_))));
        assert!(matches!(
            db.write(|c| add_field(c, "Missing", new_field("X", "text"))).await,
            Err(AppError::NotFound)
        ));
    }

    #[tokio::test]
    async fn failed_add_field_leaves_no_trace() {
        let (db, _dir) = db();
        let table = db.write(|c| create_table(c, "u1", "Deals")).await.unwrap().table;
        let mut bad = new_field("Stage", "select");
        bad.options = Some(vec![option("Won"), option("won")]);
        assert!(db.write(|c| add_field(c, "Deals", bad)).await.is_err());

        let table_sql = table.sql_name.clone();
        let cols = db.read(move |c| Ok(columns(c, &table_sql))).await.unwrap();
        // 5 fixed columns, Name, and `seq`.
        assert_eq!(cols.len(), 7, "ALTER TABLE must roll back with the catalog insert");
        assert_eq!(db.read(|c| list_fields(c, "Deals")).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn select_options_can_be_added_renamed_and_removed() {
        let (db, _dir) = db();
        let table = db.write(|c| create_table(c, "u1", "Deals")).await.unwrap().table;
        let mut stage = new_field("Stage", "select");
        stage.options = Some(vec![option("Open"), option("Won"), option("Lost")]);
        let field = db.write(|c| add_field(c, "Deals", stage)).await.unwrap();
        let labels: Vec<&str> = field.options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, ["Open", "Won", "Lost"]);

        // A record holds the "Lost" option.
        let (open, won, lost) = (field.options[0].id.clone(), field.options[1].id.clone(), field.options[2].id.clone());
        let (t, f, lost_id) = (ident(&table.sql_name).unwrap(), ident(&field.sql_name).unwrap(), lost.clone());
        db.write(move |c| {
            c.execute(
                &format!("INSERT INTO {t} (id, created_at, updated_at, {f}) VALUES ('r1', 'x', 'x', ?1)"),
                [&lost_id],
            )?;
            Ok(())
        })
        .await
        .unwrap();

        // Keep Open, rename Won, drop Lost, add Stalled.
        let patch = FieldPatch {
            name: None,
            options: Some(vec![
                OptionInput { id: Some(open.clone()), label: "Open".into(), color: Some("#00ff00".into()) },
                OptionInput { id: Some(won.clone()), label: "Closed won".into(), color: None },
                option("Stalled"),
            ]),
           ..Default::default()
        };
        let updated = db.write(move |c| update_field(c, "Deals", "Stage", patch)).await.unwrap();
        let labels: Vec<&str> = updated.options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, ["Open", "Closed won", "Stalled"]);
        assert_eq!(updated.options[0].id, open);
        assert_eq!(updated.options[1].id, won);
        assert_eq!(updated.options[0].color.as_deref(), Some("#00ff00"));

        let (t, f) = (ident(&table.sql_name).unwrap(), ident(&field.sql_name).unwrap());
        let value: Option<String> = db
            .read(move |c| Ok(c.query_row(&format!("SELECT {f} FROM {t} WHERE id = 'r1'"), [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(value, None, "removing an option clears it from records");
    }

    #[tokio::test]
    async fn option_updates_reject_unknown_and_repeated_ids() {
        let (db, _dir) = db();
        db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
        let mut stage = new_field("Stage", "select");
        stage.options = Some(vec![option("Open")]);
        let field = db.write(|c| add_field(c, "Deals", stage)).await.unwrap();
        let id = field.options[0].id.clone();

        let unknown = FieldPatch {
            name: None,
            options: Some(vec![OptionInput { id: Some("nope".into()), label: "x".into(), color: None }]),

            ..Default::default()
        };
        assert!(matches!(
            db.write(|c| update_field(c, "Deals", "Stage", unknown)).await,
            Err(AppError::BadRequest(_))
        ));
        let repeated = FieldPatch {
            name: None,
            options: Some(vec![
                OptionInput { id: Some(id.clone()), label: "a".into(), color: None },
                OptionInput { id: Some(id), label: "b".into(), color: None },
            ]),

            ..Default::default()
        };
        assert!(matches!(
            db.write(|c| update_field(c, "Deals", "Stage", repeated)).await,
            Err(AppError::BadRequest(_))
        ));
    }

    #[tokio::test]
    async fn rename_field_and_soft_delete_field() {
        let (db, _dir) = db();
        let table = db.write(|c| create_table(c, "u1", "Deals")).await.unwrap().table;
        let amount = db.write(|c| add_field(c, "Deals", new_field("Amount", "number"))).await.unwrap();

        let renamed = db
            .write(|c| update_field(c, "Deals", "Amount", FieldPatch { name: Some("Value".into()), ..Default::default() }))
            .await
            .unwrap();
        assert_eq!(renamed.name, "Value");
        assert_eq!(renamed.sql_name, amount.sql_name);

        db.write(|c| delete_field(c, "Deals", "Value")).await.unwrap();
        assert_eq!(db.read(|c| list_fields(c, "Deals")).await.unwrap().len(), 1);

        // The name is free again, and the old column is still in the table.
        db.write(|c| add_field(c, "Deals", new_field("Value", "number"))).await.unwrap();
        let table_sql = table.sql_name.clone();
        let cols = db.read(move |c| Ok(columns(c, &table_sql))).await.unwrap();
        assert!(cols.iter().any(|(n, _)| *n == amount.sql_name));
        // 5 fixed columns + Name + `seq` + the soft-deleted Amount + the new Value.
        assert_eq!(cols.len(), 9);
    }

    #[tokio::test]
    async fn primary_field_cannot_be_deleted() {
        let (db, _dir) = db();
        db.write(|c| create_table(c, "u1", "Deals")).await.unwrap();
        let r = db.write(|c| delete_field(c, "Deals", "Name")).await;
        assert!(matches!(r, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn hostile_names_are_stored_as_data_and_never_reach_sql() {
        let (db, _dir) = db();
        let hostile = "x\"; DROP TABLE users; --";
        let detail = db.write(move |c| create_table(c, "u1", hostile)).await.unwrap();
        assert_eq!(detail.table.name, hostile);
        let field = db.write(move |c| add_field(c, hostile, new_field(hostile, "text"))).await.unwrap();
        assert_eq!(field.name, hostile);

        let users: i64 = db
            .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name = 'users'", [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(users, 1);
    }

    #[tokio::test]
    async fn link_fields_use_a_column_or_a_join_table() {
        let (db, _dir) = db();
        let companies = db.write(|c| create_table(c, "u1", "Companies")).await.unwrap().table;
        let people = db.write(|c| create_table(c, "u1", "People")).await.unwrap().table;
        let relation = |name: &str, many: bool| NewField {
            name: name.into(),
            field_type: "relation".into(),
            config: FieldConfig { integer: None, table: Some("Companies".into()), many: Some(many), ..Default::default() },
            options: None,
        };
        let (one_input, many_input) = (relation("Company", false), relation("Employers", true));
        let one = db.write(move |c| add_field(c, "People", one_input)).await.unwrap();
        let many = db.write(move |c| add_field(c, "People", many_input)).await.unwrap();
        let mut skills = new_field("Skills", "multi_select");
        skills.options = Some(vec![option("Rust")]);
        let multi = db.write(|c| add_field(c, "People", skills)).await.unwrap();

        assert!(one.has_column() && !many.has_column() && !multi.has_column());
        assert!(one.sql_name.starts_with("f_") && many.sql_name.starts_with("r_") && multi.sql_name.starts_with("m_"));
        assert_eq!(one.config["table"], companies.id.as_str());

        // Only the to-one field added a column: the 5 fixed ones, Name, `seq`, and Company.
        let people_sql = people.sql_name.clone();
        let cols = db.read(move |c| Ok(columns(c, &people_sql))).await.unwrap();
        assert_eq!(cols.len(), 8);

        // That column is a foreign key to the target table that clears itself when the target record goes away.
        // Read from sqlite_master: PRAGMA foreign_key_list does not open a transaction, so a pooled reader can serve a stale schema.
        let people_name = people.sql_name.clone();
        let ddl: String = db
            .read(move |c| {
                Ok(c.query_row("SELECT sql FROM sqlite_master WHERE name = ?1", [&people_name], |r| r.get(0))?)
            })
            .await
            .unwrap();
        let clause = format!("REFERENCES \"{}\"(id) ON DELETE SET NULL", companies.sql_name);
        assert!(ddl.contains(&clause), "missing `{clause}` in {ddl}");

        // The other two live in join tables.
        let (r_name, m_name) = (many.sql_name.clone(), multi.sql_name.clone());
        let joins: i64 = db
            .read(move |c| {
                Ok(c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (?1, ?2)",
                    [&r_name, &m_name],
                    |r| r.get(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(joins, 2);
    }

    #[test]
    fn ident_accepts_only_generated_names() {
        let ok = format!("t_{}", "a".repeat(32));
        assert_eq!(ident(&ok).unwrap(), format!("\"{ok}\""));
        for bad in ["users", "t_short", &format!("t_{}", "A".repeat(32)), &format!("x_{}", "a".repeat(32)), "\"; DROP"] {
            assert!(ident(bad).is_err(), "{bad}");
        }
    }
}
