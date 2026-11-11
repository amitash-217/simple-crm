//! Compiles the JSON filter tree and sort list into SQL fragments with bound parameters.
//!
//! Column expressions come only from generated identifiers (`ident`) or fixed system columns. Every
//! user-supplied value is bound as a parameter, so no request data is ever spliced into SQL text.

use std::collections::HashSet;

use rusqlite::types::Value as Sql;
use serde_json::{Map, Value};

use super::{field_by_key, value};
use crate::{
    catalog::{ident, FieldOut},
    error::AppError,
};

const MAX_DEPTH: usize = 8;
const MAX_CONDITIONS: usize = 50;
const MAX_IN: usize = 100;
const MAX_SORTS: usize = 5;
const ID_EXPR: &str = "\"id\"";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    Number,
    Bool,
    Date,
    Select,
    /// A to-one relation: a column holding one record ID.
    Link,
    /// Multi-select or a to-many relation: values live in a join table.
    Multi,
}

impl Kind {
    fn ops(self) -> &'static [&'static str] {
        match self {
            Kind::Text => &["eq", "neq", "contains", "starts_with", "in", "is_empty", "is_not_empty"],
            Kind::Number | Kind::Date => &["eq", "neq", "gt", "gte", "lt", "lte", "in", "is_empty", "is_not_empty"],
            Kind::Bool => &["eq", "neq", "is_empty", "is_not_empty"],
            Kind::Select | Kind::Link => &["eq", "neq", "in", "is_empty", "is_not_empty"],
            Kind::Multi => &["has", "has_any", "is_empty", "is_not_empty"],
        }
    }
}

/// What a filter or sort `field` refers to: a user field, or a system column such as `$created_at`.
struct Target<'a> {
    expr: String,
    kind: Kind,
    name: String,
    field: Option<&'a FieldOut>,
}

fn target<'a>(fields: &'a [FieldOut], key: &str) -> Result<Target<'a>, AppError> {
    let system = match key {
        "$id" => Some(("id", Kind::Text)),
        "$created_by" => Some(("created_by", Kind::Text)),
        "$updated_by" => Some(("updated_by", Kind::Text)),
        "$created_at" => Some(("created_at", Kind::Date)),
        "$updated_at" => Some(("updated_at", Kind::Date)),
        _ => None,
    };
    if let Some((column, kind)) = system {
        return Ok(Target { expr: format!("\"{column}\""), kind, name: key.to_string(), field: None });
    }
    let field = field_by_key(fields, key)
        .ok_or_else(|| AppError::BadRequest(format!("unknown field `{key}`")))?;
    let kind = match field.field_type {
        "text" | "long_text" | "email" | "phone" => Kind::Text,
        "number" => Kind::Number,
        "boolean" => Kind::Bool,
        "date" => Kind::Date,
        "select" => Kind::Select,
        "multi_select" => Kind::Multi,
        "relation" if field.in_join_table() => Kind::Multi,
        "relation" => Kind::Link,
        other => return Err(AppError::Internal(format!("unsupported field type `{other}`"))),
    };
    // For join-table fields, `expr` is the join table, not a column.
    Ok(Target { expr: ident(&field.sql_name)?, kind, name: field.name.clone(), field: Some(field) })
}

/// `EXISTS (...)` over a join table, for the record being filtered (aliased `rec` in the list query).
fn linked(join: &str, own: &str, extra: &str) -> String {
    format!("EXISTS (SELECT 1 FROM {join} WHERE {own} = rec.\"id\"{extra})")
}

fn operand(t: &Target, v: &Value) -> Result<Sql, AppError> {
    if let Some(field) = t.field {
        return value::to_sql(field, v);
    }
    match v {
        Value::Null => Ok(Sql::Null),
        Value::String(s) if t.kind != Kind::Date || value::valid_date(s) => Ok(Sql::Text(s.clone())),
        _ => Err(AppError::BadRequest(format!("`{}` expects a string value", t.name))),
    }
}

/// A compiled `WHERE` fragment and its parameters, in the order the `?` placeholders appear.
pub struct Compiled {
    pub sql: String,
    pub params: Vec<Sql>,
}

pub fn compile_filter(fields: &[FieldOut], filter: &Value) -> Result<Compiled, AppError> {
    let mut walk = Walk { fields, params: Vec::new(), conditions: 0 };
    let sql = walk.node(filter, 0)?;
    Ok(Compiled { sql, params: walk.params })
}

struct Walk<'a> {
    fields: &'a [FieldOut],
    params: Vec<Sql>,
    conditions: usize,
}

impl Walk<'_> {
    fn node(&mut self, v: &Value, depth: usize) -> Result<String, AppError> {
        if depth > MAX_DEPTH {
            return Err(AppError::BadRequest(format!("filter nests deeper than {MAX_DEPTH} levels")));
        }
        let obj = v
            .as_object()
            .ok_or_else(|| AppError::BadRequest("each filter node must be a JSON object".into()))?;
        let Some(group) = ["and", "or"].into_iter().find(|g| obj.contains_key(*g)) else {
            return self.condition(obj);
        };
        if obj.len() != 1 {
            return Err(AppError::BadRequest(format!("a `{group}` group must have no other keys")));
        }
        let items = obj[group]
            .as_array()
            .ok_or_else(|| AppError::BadRequest(format!("`{group}` must be an array")))?;
        if items.is_empty() {
            return Ok(if group == "and" { "1" } else { "0" }.to_string());
        }
        let parts = items
            .iter()
            .map(|item| self.node(item, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!("({})", parts.join(if group == "and" { " AND " } else { " OR " })))
    }

    fn condition(&mut self, obj: &Map<String, Value>) -> Result<String, AppError> {
        if let Some(extra) = obj.keys().find(|k| !matches!(k.as_str(), "field" | "op" | "value")) {
            return Err(AppError::BadRequest(format!("unknown key `{extra}` in filter condition")));
        }
        let key = obj
            .get("field")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::BadRequest("a filter condition needs a string `field`".into()))?;
        let op = obj
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::BadRequest("a filter condition needs a string `op`".into()))?;
        let t = target(self.fields, key)?;
        if !t.kind.ops().contains(&op) {
            return Err(AppError::BadRequest(format!(
                "operator `{op}` does not apply to `{}`; valid operators: {}",
                t.name,
                t.kind.ops().join(", ")
            )));
        }
        self.conditions += 1;
        if self.conditions > MAX_CONDITIONS {
            return Err(AppError::BadRequest(format!("filter has more than {MAX_CONDITIONS} conditions")));
        }

        let col = &t.expr;
        if t.kind == Kind::Multi {
            return self.multi_condition(&t, op, obj.get("value"));
        }
        let text = t.kind == Kind::Text;
        match op {
            "is_empty" if text => return Ok(format!("({col} IS NULL OR {col} = '')")),
            "is_empty" => return Ok(format!("{col} IS NULL")),
            "is_not_empty" if text => return Ok(format!("({col} IS NOT NULL AND {col} != '')")),
            "is_not_empty" => return Ok(format!("{col} IS NOT NULL")),
            _ => {}
        }

        let v = obj
            .get("value")
            .ok_or_else(|| AppError::BadRequest(format!("operator `{op}` needs a `value`")))?;
        match op {
            "eq" | "neq" => {
                let not = if op == "neq" { "NOT " } else { "" };
                match operand(&t, v)? {
                    Sql::Null => Ok(format!("{col} IS {not}NULL")),
                    other => {
                        self.params.push(other);
                        Ok(format!("{col} IS {not}?"))
                    }
                }
            }
            "gt" | "gte" | "lt" | "lte" => {
                let symbol = match op {
                    "gt" => ">",
                    "gte" => ">=",
                    "lt" => "<",
                    _ => "<=",
                };
                match operand(&t, v)? {
                    Sql::Null => Err(AppError::BadRequest(format!("operator `{op}` needs a non-null value"))),
                    other => {
                        self.params.push(other);
                        Ok(format!("{col} {symbol} ?"))
                    }
                }
            }
            "contains" | "starts_with" => {
                let s = v.as_str().ok_or_else(|| {
                    AppError::BadRequest(format!("operator `{op}` needs a string value"))
                })?;
                let escaped = s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
                let pattern = if op == "contains" { format!("%{escaped}%") } else { format!("{escaped}%") };
                self.params.push(Sql::Text(pattern));
                Ok(format!("{col} LIKE ? ESCAPE '\\'"))
            }
            "in" => {
                let items = v
                    .as_array()
                    .ok_or_else(|| AppError::BadRequest("operator `in` needs an array value".into()))?;
                if items.len() > MAX_IN {
                    return Err(AppError::BadRequest(format!("`in` accepts at most {MAX_IN} values")));
                }
                if items.is_empty() {
                    return Ok("0".to_string());
                }
                for item in items {
                    match operand(&t, item)? {
                        Sql::Null => return Err(AppError::BadRequest("`in` values must not be null".into())),
                        other => self.params.push(other),
                    }
                }
                Ok(format!("{col} IN ({})", vec!["?"; items.len()].join(", ")))
            }
            _ => Err(AppError::Internal(format!("unhandled operator `{op}`"))),
        }
    }
}

impl Walk<'_> {
    /// Conditions on multi-select and to-many relation fields.
    fn multi_condition(&mut self, t: &Target, op: &str, value: Option<&Value>) -> Result<String, AppError> {
        let field = t.field.ok_or_else(|| AppError::Internal("join field without a catalog entry".into()))?;
        let join = &t.expr;
        let (own, column) = field.link_columns();
        match op {
            "is_empty" => return Ok(format!("NOT {}", linked(join, own, ""))),
            "is_not_empty" => return Ok(linked(join, own, "")),
            _ => {}
        }
        let value = value.ok_or_else(|| AppError::BadRequest(format!("operator `{op}` needs a `value`")))?;
        let bad = |msg: &str| AppError::BadRequest(format!("field `{}`: {msg}", t.name));
        let one = |item: &Value| -> Result<String, AppError> {
            let s = item.as_str().filter(|s| !s.is_empty()).ok_or_else(|| bad("expected a string"))?;
            if field.field_type == "multi_select" {
                value::option_id(field, s).ok_or_else(|| bad(&format!("`{s}` is not one of this field's options")))
            } else {
                Ok(s.to_string())
            }
        };
        match op {
            "has" => {
                self.params.push(Sql::Text(one(value)?));
                Ok(linked(join, own, &format!(" AND {column} = ?")))
            }
            "has_any" => {
                let items = value.as_array().ok_or_else(|| bad("`has_any` needs an array value"))?;
                if items.len() > MAX_IN {
                    return Err(bad(&format!("`has_any` accepts at most {MAX_IN} values")));
                }
                if items.is_empty() {
                    return Ok("0".to_string());
                }
                for item in items {
                    self.params.push(Sql::Text(one(item)?));
                }
                let marks = vec!["?"; items.len()].join(", ");
                Ok(linked(join, own, &format!(" AND {column} IN ({marks})")))
            }
            _ => Err(AppError::Internal(format!("unhandled operator `{op}`"))),
        }
    }
}

/// One `ORDER BY` key. `expr` is also what the keyset cursor compares against.
pub struct Sort {
    pub expr: String,
    pub desc: bool,
}

/// Parses `[{"field": "Amount", "dir": "desc"}, ...]`. A final `id` key is always present, so order is total.
pub fn compile_sort(fields: &[FieldOut], sort: Option<&Value>) -> Result<Vec<Sort>, AppError> {
    let mut keys = Vec::new();
    let mut seen = HashSet::new();
    if let Some(sort) = sort {
        let items = sort
            .as_array()
            .ok_or_else(|| AppError::BadRequest("sort must be a JSON array".into()))?;
        if items.len() > MAX_SORTS {
            return Err(AppError::BadRequest(format!("sort accepts at most {MAX_SORTS} keys")));
        }
        for item in items {
            let obj = item
                .as_object()
                .ok_or_else(|| AppError::BadRequest("each sort key must be a JSON object".into()))?;
            if let Some(extra) = obj.keys().find(|k| !matches!(k.as_str(), "field" | "dir")) {
                return Err(AppError::BadRequest(format!("unknown key `{extra}` in sort key")));
            }
            let key = obj
                .get("field")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::BadRequest("a sort key needs a string `field`".into()))?;
            let desc = match obj.get("dir").map(Value::as_str) {
                None | Some(Some("asc")) => false,
                Some(Some("desc")) => true,
                _ => return Err(AppError::BadRequest("`dir` must be \"asc\" or \"desc\"".into())),
            };
            let t = target(fields, key)?;
            if matches!(t.kind, Kind::Link | Kind::Multi) {
                return Err(AppError::BadRequest(format!(
                    "cannot sort by `{}`: relation and multi-select fields are not sortable",
                    t.name
                )));
            }
            // Select fields sort by the option's position, which is the order users arranged them in.
            let expr = match t.field {
                Some(f) if f.field_type == "select" => {
                    format!("(SELECT position FROM field_options WHERE id = {})", t.expr)
                }
                _ => t.expr,
            };
            if !seen.insert(expr.clone()) {
                return Err(AppError::BadRequest(format!("`{key}` appears twice in sort")));
            }
            keys.push(Sort { expr, desc });
        }
    }
    if !seen.contains(ID_EXPR) {
        keys.push(Sort { expr: ID_EXPR.to_string(), desc: false });
    }
    Ok(keys)
}

pub fn order_by(keys: &[Sort]) -> String {
    keys.iter()
        .map(|k| format!("{} {}", k.expr, if k.desc { "DESC" } else { "ASC" }))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Builds the condition for "rows strictly after the row whose sort values are `vals`".
///
/// SQLite orders NULL first when ascending and last when descending, and this follows that rule,
/// so a cursor stays correct across NULLs, and when rows are inserted or removed between pages.
pub fn after_cursor(keys: &[Sort], vals: &[Sql]) -> Compiled {
    let mut branches = Vec::new();
    let mut params = Vec::new();
    for (i, (key, val)) in keys.iter().zip(vals).enumerate() {
        let mut parts: Vec<String> = Vec::new();
        let mut branch_params: Vec<Sql> = Vec::new();
        for (earlier, earlier_val) in keys[..i].iter().zip(vals) {
            parts.push(format!("{} IS ?", earlier.expr));
            branch_params.push(earlier_val.clone());
        }
        let after = match (key.desc, val) {
            (false, Sql::Null) => format!("{} IS NOT NULL", key.expr),
            (false, v) => {
                branch_params.push(v.clone());
                format!("{} > ?", key.expr)
            }
            // Nothing sorts after NULL when descending.
            (true, Sql::Null) => continue,
            (true, v) => {
                branch_params.push(v.clone());
                format!("({0} < ? OR {0} IS NULL)", key.expr)
            }
        };
        parts.push(after);
        branches.push(format!("({})", parts.join(" AND ")));
        params.extend(branch_params);
    }
    let sql = if branches.is_empty() { "0".to_string() } else { format!("({})", branches.join(" OR ")) };
    Compiled { sql, params }
}
