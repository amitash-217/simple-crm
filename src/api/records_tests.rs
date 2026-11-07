use axum::{http::{Method, StatusCode}, Router};
use serde_json::{json, Value};

use super::auth_tests::{app, call, register_admin, Reply};

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

struct Env {
    app: Router,
    cookie: String,
    _dir: tempfile::TempDir,
}

impl Env {
    async fn call(&self, method: Method, uri: &str, body: Option<Value>) -> Reply {
        call(&self.app, method, uri, body, Some(("cookie", self.cookie.as_str()))).await
    }

    async fn post(&self, uri: &str, body: Value) -> Reply {
        self.call(Method::POST, uri, Some(body)).await
    }

    /// GET /records with optional filter, sort, limit and cursor.
    async fn list(&self, filter: Option<Value>, sort: Option<Value>, limit: Option<usize>, cursor: Option<&str>) -> Reply {
        let mut parts = Vec::new();
        if let Some(f) = filter {
            parts.push(format!("filter={}", enc(&f.to_string())));
        }
        if let Some(s) = sort {
            parts.push(format!("sort={}", enc(&s.to_string())));
        }
        if let Some(l) = limit {
            parts.push(format!("limit={l}"));
        }
        if let Some(c) = cursor {
            parts.push(format!("cursor={c}"));
        }
        self.call(Method::GET, &format!("/api/tables/Deals/records?{}", parts.join("&")), None).await
    }

    async fn names(&self, filter: Value) -> Vec<String> {
        let r = self.list(Some(filter), None, None, None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        names_of(&r)
    }

    /// Follows `next_cursor` to the end and returns every name, in order.
    async fn paged_names(&self, sort: Value, limit: usize) -> Vec<String> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..20 {
            let r = self.list(None, Some(sort.clone()), Some(limit), cursor.as_deref()).await;
            assert_eq!(r.status, StatusCode::OK, "{}", r.body);
            all.extend(names_of(&r));
            match r.body["next_cursor"].as_str() {
                Some(c) => cursor = Some(c.to_string()),
                None => return all,
            }
        }
        panic!("pagination did not finish");
    }
}

fn names_of(r: &Reply) -> Vec<String> {
    r.body["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rec| rec["values"]["Name"].as_str().unwrap().to_string())
        .collect()
}

/// A `Deals` table with typed fields, and no records.
async fn env() -> Env {
    let (app, dir) = app();
    let cookie = register_admin(&app).await;
    let env = Env { app, cookie, _dir: dir };
    assert_eq!(env.post("/api/tables", json!({ "name": "Deals" })).await.status, StatusCode::CREATED);
    for field in [
        json!({ "name": "Amount", "type": "number" }),
        json!({ "name": "Stage", "type": "select", "options": [{ "label": "Open" }, { "label": "Won" }, { "label": "Lost" }] }),
        json!({ "name": "Mobile", "type": "phone" }),
        json!({ "name": "Closes", "type": "date" }),
        json!({ "name": "Priority", "type": "boolean" }),
        json!({ "name": "Seats", "type": "number", "config": { "integer": true } }),
    ] {
        let r = env.post("/api/tables/Deals/fields", field).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    }
    env
}

/// Six records. Names sort in creation order, which is the default order.
async fn seeded() -> Env {
    let env = env().await;
    for values in [
        json!({ "Name": "Acme", "Amount": 100, "Stage": "Open", "Priority": true }),
        json!({ "Name": "Beta", "Amount": 250.5, "Stage": "Won", "Priority": false }),
        json!({ "Name": "Cyan", "Stage": "Won" }),
        json!({ "Name": "Delta 100%", "Amount": 100, "Stage": "Lost", "Closes": "2026-03-01" }),
        json!({ "Name": "Echo_1", "Amount": 40, "Closes": "2026-01-15" }),
        json!({ "Name": "Foxtrot", "Amount": 250.5, "Stage": "Open" }),
    ] {
        let r = env.post("/api/tables/Deals/records", json!({ "values": values })).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    }
    env
}

fn cond(field: &str, op: &str, value: Value) -> Value {
    json!({ "field": field, "op": op, "value": value })
}

#[tokio::test]
async fn record_endpoints_require_auth() {
    let (app, _dir) = app();
    for (method, uri) in [
        (Method::GET, "/api/tables/Deals/records"),
        (Method::POST, "/api/tables/Deals/records"),
        (Method::GET, "/api/tables/Deals/records/x"),
        (Method::PATCH, "/api/tables/Deals/records/x"),
        (Method::DELETE, "/api/tables/Deals/records/x"),
    ] {
        let r = call(&app, method, uri, Some(json!({})), None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn create_read_update_delete() {
    let env = env().await;

    let created = env
        .post(
            "/api/tables/Deals/records",
            json!({ "values": {
                "Name": "Acme", "Amount": 99.5, "Stage": "won", "Mobile": "+1 (415) 555-2671",
                "Closes": "2026-03-01", "Priority": true, "Seats": 12
            } }),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let rec = &created.body;
    let id = rec["id"].as_str().unwrap().to_string();
    assert_eq!(rec["values"]["Name"], "Acme");
    assert_eq!(rec["values"]["Amount"], 99.5);
    assert_eq!(rec["values"]["Stage"], "Won", "select comes back as its label, matched case-insensitively on write");
    assert_eq!(rec["values"]["Mobile"], "+14155552671");
    assert_eq!(rec["values"]["Priority"], true);
    assert_eq!(rec["values"]["Seats"], 12);
    assert!(rec["created_by"].is_string());
    assert_eq!(rec["created_at"], rec["updated_at"]);
    let names: Vec<&str> = rec["values"].as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(names, ["Name", "Amount", "Stage", "Mobile", "Closes", "Priority", "Seats"], "values follow field order");

    let got = env.call(Method::GET, &format!("/api/tables/Deals/records/{id}"), None).await;
    assert_eq!(got.status, StatusCode::OK);
    assert_eq!(got.body, *rec);

    // PATCH changes only the keys it names; null clears.
    let patched = env
        .call(
            Method::PATCH,
            &format!("/api/tables/Deals/records/{id}"),
            Some(json!({ "values": { "Amount": 120, "Stage": null } })),
        )
        .await;
    assert_eq!(patched.status, StatusCode::OK, "{}", patched.body);
    assert_eq!(patched.body["values"]["Amount"], 120.0);
    assert_eq!(patched.body["values"]["Stage"], Value::Null);
    assert_eq!(patched.body["values"]["Name"], "Acme");
    assert_eq!(patched.body["values"]["Mobile"], "+14155552671");
    assert_eq!(patched.body["created_at"], rec["created_at"]);

    let deleted = env.call(Method::DELETE, &format!("/api/tables/Deals/records/{id}"), None).await;
    assert_eq!(deleted.status, StatusCode::OK);
    for method in [Method::GET, Method::DELETE] {
        let gone = env.call(method, &format!("/api/tables/Deals/records/{id}"), None).await;
        assert_eq!(gone.status, StatusCode::NOT_FOUND);
    }
    let gone = env
        .call(Method::PATCH, &format!("/api/tables/Deals/records/{id}"), Some(json!({ "values": {} })))
        .await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn empty_record_and_keys_by_field_id() {
    let env = env().await;
    let empty = env.post("/api/tables/Deals/records", json!({})).await;
    assert_eq!(empty.status, StatusCode::CREATED);
    assert!(empty.body["values"].as_object().unwrap().values().all(Value::is_null));

    let fields = env.call(Method::GET, "/api/tables/Deals/fields", None).await;
    let amount_id = fields.body.as_array().unwrap().iter().find(|f| f["name"] == "Amount").unwrap()["id"].clone();
    let r = env.post("/api/tables/Deals/records", json!({ "values": { amount_id.as_str().unwrap(): 7 } })).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["values"]["Amount"], 7.0);
}

#[tokio::test]
async fn invalid_values_are_rejected_without_writing() {
    let env = env().await;
    let cases = [
        json!({ "Nope": 1 }),
        json!({ "Name": 5 }),
        json!({ "Amount": "ten" }),
        json!({ "Seats": 1.5 }),
        json!({ "Priority": "yes" }),
        json!({ "Stage": "Nonsense" }),
        json!({ "Mobile": "4155552671" }),
        json!({ "Closes": "2026-02-30" }),
        json!({ "Name": "a", "name": "b" }),
    ];
    for values in cases {
        let r = env.post("/api/tables/Deals/records", json!({ "values": values })).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{values}");
        assert_eq!(r.body["error"]["code"], "bad_request");
    }
    let r = env.post("/api/tables/Deals/records", json!({ "values": {}, "id": "x" })).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "unknown top-level keys are rejected");

    let all = env.list(None, None, None, None).await;
    assert!(all.body["records"].as_array().unwrap().is_empty(), "failed writes leave nothing behind");
}

#[tokio::test]
async fn unknown_table_and_hostile_values() {
    let env = env().await;
    let r = env.post("/api/tables/Nope/records", json!({})).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    let hostile = "x'); DROP TABLE users; --";
    let created = env.post("/api/tables/Deals/records", json!({ "values": { "Name": hostile } })).await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.body["values"]["Name"], hostile);
    let found = env.names(cond("Name", "eq", json!(hostile))).await;
    assert_eq!(found, [hostile]);
    assert_eq!(env.call(Method::GET, "/api/users", None).await.status, StatusCode::OK, "users table is intact");
}

#[tokio::test]
async fn deleted_fields_and_tables_disappear_from_records() {
    let env = seeded().await;
    env.call(Method::DELETE, "/api/tables/Deals/fields/Amount", None).await;
    let r = env.list(None, None, None, None).await;
    assert!(r.body["records"][0]["values"].get("Amount").is_none());
    let write = env.post("/api/tables/Deals/records", json!({ "values": { "Amount": 1 } })).await;
    assert_eq!(write.status, StatusCode::BAD_REQUEST);

    env.call(Method::DELETE, "/api/tables/Deals", None).await;
    assert_eq!(env.list(None, None, None, None).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn default_order_is_creation_order() {
    let env = seeded().await;
    let r = env.list(None, None, None, None).await;
    assert_eq!(names_of(&r), ["Acme", "Beta", "Cyan", "Delta 100%", "Echo_1", "Foxtrot"]);
    assert!(r.body["next_cursor"].is_null());
}

#[tokio::test]
async fn filter_operators() {
    let env = seeded().await;
    let id = |names: Vec<&str>| names.into_iter().map(String::from).collect::<Vec<_>>();

    assert_eq!(env.names(cond("Amount", "eq", json!(100))).await, id(vec!["Acme", "Delta 100%"]));
    assert_eq!(env.names(cond("Amount", "gt", json!(100))).await, id(vec!["Beta", "Foxtrot"]));
    assert_eq!(env.names(cond("Amount", "gte", json!(100))).await, id(vec!["Acme", "Beta", "Delta 100%", "Foxtrot"]));
    assert_eq!(env.names(cond("Amount", "lt", json!(100))).await, id(vec!["Echo_1"]));
    assert_eq!(env.names(cond("Amount", "lte", json!(100))).await, id(vec!["Acme", "Delta 100%", "Echo_1"]));
    assert_eq!(env.names(cond("amount", "in", json!([40, 250.5]))).await, id(vec!["Beta", "Echo_1", "Foxtrot"]));

    // neq keeps records where the field is empty.
    assert_eq!(
        env.names(cond("Stage", "neq", json!("Won"))).await,
        id(vec!["Acme", "Delta 100%", "Echo_1", "Foxtrot"])
    );
    assert_eq!(env.names(cond("Stage", "eq", json!("open"))).await, id(vec!["Acme", "Foxtrot"]));
    assert_eq!(env.names(cond("Stage", "in", json!(["Open", "Lost"]))).await, id(vec!["Acme", "Delta 100%", "Foxtrot"]));
    assert_eq!(env.names(cond("Stage", "eq", Value::Null)).await, id(vec!["Echo_1"]));

    assert_eq!(env.names(json!({ "field": "Amount", "op": "is_empty" })).await, id(vec!["Cyan"]));
    assert_eq!(env.names(json!({ "field": "Closes", "op": "is_not_empty" })).await, id(vec!["Delta 100%", "Echo_1"]));
    assert_eq!(env.names(cond("Priority", "eq", json!(true))).await, id(vec!["Acme"]));
    assert_eq!(env.names(cond("Closes", "lt", json!("2026-02-01"))).await, id(vec!["Echo_1"]));
    assert_eq!(env.names(cond("Closes", "gte", json!("2026-03-01"))).await, id(vec!["Delta 100%"]));

    // LIKE wildcards in the value are literal characters.
    assert_eq!(env.names(cond("Name", "contains", json!("%"))).await, id(vec!["Delta 100%"]));
    assert_eq!(env.names(cond("Name", "contains", json!("_"))).await, id(vec!["Echo_1"]));
    assert_eq!(env.names(cond("Name", "starts_with", json!("de"))).await, id(vec!["Delta 100%"]));
    let with_a = env.names(cond("Name", "contains", json!("A"))).await;
    assert_eq!(with_a, id(vec!["Acme", "Beta", "Cyan", "Delta 100%"]), "case-insensitive");

    assert_eq!(env.names(cond("$created_at", "gt", json!("2000-01-01"))).await.len(), 6);
    assert_eq!(env.names(cond("$created_at", "lt", json!("2000-01-01"))).await.len(), 0);
    assert_eq!(env.names(cond("Amount", "in", json!([]))).await.len(), 0);
}

#[tokio::test]
async fn filter_groups_nest() {
    let env = seeded().await;
    let filter = json!({ "and": [
        cond("Stage", "eq", json!("Open")),
        { "or": [cond("Amount", "lt", json!(150)), cond("Name", "eq", json!("Foxtrot"))] }
    ] });
    assert_eq!(env.names(filter).await, ["Acme", "Foxtrot"]);

    assert_eq!(env.names(json!({ "and": [] })).await.len(), 6, "an empty and matches everything");
    assert_eq!(env.names(json!({ "or": [] })).await.len(), 0, "an empty or matches nothing");
    let either = json!({ "or": [cond("Name", "eq", json!("Acme")), cond("Name", "eq", json!("Beta"))] });
    assert_eq!(env.names(either).await, ["Acme", "Beta"]);
}

#[tokio::test]
async fn bad_filters_are_rejected() {
    let env = seeded().await;
    let bad = [
        cond("Nope", "eq", json!(1)),
        cond("Amount", "contains", json!("1")),
        cond("Name", "gt", json!("a")),
        cond("Priority", "gt", json!(true)),
        cond("Amount", "bogus", json!(1)),
        cond("Amount", "gt", Value::Null),
        cond("Amount", "eq", json!("ten")),
        cond("Stage", "eq", json!("Nonsense")),
        cond("Amount", "in", json!(5)),
        cond("Closes", "eq", json!("tomorrow")),
        cond("$bogus", "eq", json!("x")),
        json!({ "field": "Amount", "op": "eq" }),
        json!({ "field": "Amount", "value": 1 }),
        json!({ "field": "Amount", "op": "eq", "value": 1, "extra": true }),
        json!({ "and": [], "or": [] }),
        json!({ "and": "no" }),
        json!([]),
        json!("text"),
    ];
    for filter in bad {
        let r = env.list(Some(filter.clone()), None, None, None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{filter}");
        assert_eq!(r.body["error"]["code"], "bad_request");
    }

    let not_json = env.call(Method::GET, "/api/tables/Deals/records?filter=%7Bnope", None).await;
    assert_eq!(not_json.status, StatusCode::BAD_REQUEST);

    let mut deep = cond("Amount", "eq", json!(1));
    for _ in 0..10 {
        deep = json!({ "and": [deep] });
    }
    assert_eq!(env.list(Some(deep), None, None, None).await.status, StatusCode::BAD_REQUEST);
    let many: Vec<Value> = (0..51).map(|_| cond("Amount", "eq", json!(1))).collect();
    assert_eq!(env.list(Some(json!({ "or": many })), None, None, None).await.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sorting() {
    let env = seeded().await;
    let sort = |v: Value| env.list(None, Some(v), None, None);

    // NULLs come first ascending and last descending. Ties fall back to creation order.
    let r = sort(json!([{ "field": "Amount" }])).await;
    assert_eq!(names_of(&r), ["Cyan", "Echo_1", "Acme", "Delta 100%", "Beta", "Foxtrot"]);
    let r = sort(json!([{ "field": "Amount", "dir": "desc" }])).await;
    assert_eq!(names_of(&r), ["Beta", "Foxtrot", "Acme", "Delta 100%", "Echo_1", "Cyan"]);

    // Select fields sort by option order: Open, Won, Lost.
    let r = sort(json!([{ "field": "Stage" }])).await;
    assert_eq!(names_of(&r), ["Echo_1", "Acme", "Foxtrot", "Beta", "Cyan", "Delta 100%"]);

    let r = sort(json!([{ "field": "Stage" }, { "field": "Amount", "dir": "desc" }])).await;
    assert_eq!(names_of(&r), ["Echo_1", "Foxtrot", "Acme", "Beta", "Cyan", "Delta 100%"]);

    let r = sort(json!([{ "field": "$created_at", "dir": "desc" }, { "field": "$id", "dir": "desc" }])).await;
    assert_eq!(names_of(&r), ["Foxtrot", "Echo_1", "Delta 100%", "Cyan", "Beta", "Acme"]);

    for bad in [
        json!([{ "field": "Nope" }]),
        json!([{ "field": "Amount", "dir": "up" }]),
        json!([{ "field": "Amount" }, { "field": "amount" }]),
        json!([{ "field": "Amount", "extra": 1 }]),
        json!({ "field": "Amount" }),
        json!([1]),
    ] {
        assert_eq!(sort(bad.clone()).await.status, StatusCode::BAD_REQUEST, "{bad}");
    }
}

#[tokio::test]
async fn pagination_matches_the_unpaged_order() {
    let env = seeded().await;
    let sorts = [
        json!([]),
        json!([{ "field": "Amount" }]),
        json!([{ "field": "Amount", "dir": "desc" }]),
        json!([{ "field": "Stage" }, { "field": "Amount", "dir": "desc" }]),
        json!([{ "field": "Closes", "dir": "desc" }, { "field": "Name" }]),
        json!([{ "field": "$updated_at", "dir": "desc" }]),
    ];
    for sort in sorts {
        let whole = names_of(&env.list(None, Some(sort.clone()), Some(200), None).await);
        assert_eq!(whole.len(), 6);
        for limit in [1, 2, 4, 5] {
            let paged = env.paged_names(sort.clone(), limit).await;
            assert_eq!(paged, whole, "sort {sort}, limit {limit}");
        }
    }
}

#[tokio::test]
async fn pagination_combines_with_filters_and_survives_inserts() {
    let env = seeded().await;
    let filter = cond("Amount", "gte", json!(100));
    let first = env.list(Some(filter.clone()), None, Some(2), None).await;
    assert_eq!(names_of(&first), ["Acme", "Beta"]);
    let cursor = first.body["next_cursor"].as_str().unwrap().to_string();

    // A record inserted before the cursor does not shift the next page.
    let sorted = json!([{ "field": "Amount" }]);
    let page1 = env.list(Some(filter.clone()), Some(sorted.clone()), Some(2), None).await;
    assert_eq!(names_of(&page1), ["Acme", "Delta 100%"]);
    env.post("/api/tables/Deals/records", json!({ "values": { "Name": "Early", "Amount": 100 } })).await;
    env.post("/api/tables/Deals/records", json!({ "values": { "Name": "Zero", "Amount": 1000 } })).await;
    let page2 = env
        .list(Some(filter.clone()), Some(sorted), Some(2), page1.body["next_cursor"].as_str())
        .await;
    assert_eq!(names_of(&page2), ["Early", "Beta"], "rows already served are not repeated");

    // The first cursor still works with its own sort.
    let next = env.list(Some(filter), None, Some(2), Some(&cursor)).await;
    assert_eq!(next.status, StatusCode::OK);
    assert_eq!(names_of(&next)[0], "Delta 100%");
}

#[tokio::test]
async fn bad_paging_parameters() {
    let env = seeded().await;
    for query in ["limit=0", "limit=201", "limit=abc", "cursor=zz", "cursor=abc", "cursor=5b5d", "bogus=1"] {
        let r = env.call(Method::GET, &format!("/api/tables/Deals/records?{query}"), None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(r.body["error"]["code"], "bad_request", "{query}");
    }
    // A cursor from one sort is refused by another.
    let page = env.list(None, Some(json!([{ "field": "Amount" }])), Some(2), None).await;
    let cursor = page.body["next_cursor"].as_str().unwrap();
    let r = env.list(None, None, Some(2), Some(cursor)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn api_keys_can_use_records() {
    let env = env().await;
    let key = env.post("/api/api-keys", json!({ "name": "ci" })).await;
    let bearer = format!("Bearer {}", key.body["key"].as_str().unwrap());
    let created = call(
        &env.app,
        Method::POST,
        "/api/tables/Deals/records",
        Some(json!({ "values": { "Name": "Via key" } })),
        Some(("authorization", &bearer)),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert!(created.body["created_by"].is_string());
}
