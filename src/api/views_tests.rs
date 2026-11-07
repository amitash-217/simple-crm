//! Saved views, end to end through the HTTP API.

use axum::{http::{Method, StatusCode}, Router};
use serde_json::{json, Value};

use super::auth_tests::{app, call, register_admin, Reply};

struct Env {
    app: Router,
    cookie: String,
    _dir: tempfile::TempDir,
}

impl Env {
    async fn new() -> Env {
        let (app, dir) = app();
        let cookie = register_admin(&app).await;
        Env { app, cookie, _dir: dir }
    }

    async fn call(&self, method: Method, uri: &str, body: Option<Value>) -> Reply {
        call(&self.app, method, uri, body, Some(("cookie", self.cookie.as_str()))).await
    }

    async fn ok(&self, method: Method, uri: &str, body: Option<Value>) -> Value {
        let r = self.call(method, uri, body).await;
        assert!(r.status.is_success(), "{uri}: {} {}", r.status, r.body);
        r.body
    }

    /// A `Deals` table with a Stage select (Open, Won, Lost) and an Amount number.
    async fn deals(&self) -> Value {
        let table = self.ok(Method::POST, "/api/tables", Some(json!({ "name": "Deals" }))).await;
        let t = table["id"].as_str().unwrap();
        let stage = self
            .ok(
                Method::POST,
                &format!("/api/tables/{t}/fields"),
                Some(json!({ "name": "Stage", "type": "select", "options": [
                    { "label": "Open" }, { "label": "Won" }, { "label": "Lost" } ] })),
            )
            .await;
        let amount = self
            .ok(Method::POST, &format!("/api/tables/{t}/fields"), Some(json!({ "name": "Amount", "type": "number" })))
            .await;
        let fields = self.ok(Method::GET, &format!("/api/tables/{t}/fields"), None).await;
        let name = fields.as_array().unwrap().iter().find(|f| f["is_primary"] == true).unwrap().clone();
        json!({ "table": t, "stage": stage, "amount": amount, "name": name })
    }
}

fn id(v: &Value) -> &str {
    v["id"].as_str().unwrap()
}

#[tokio::test]
async fn requires_login() {
    let (app, _dir) = app();
    let r = call(&app, Method::GET, "/api/tables/x/views", None, None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_stores_ids_and_get_by_id_or_name() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let r = env
        .call(
            Method::POST,
            &format!("/api/tables/{t}/views"),
            Some(json!({ "name": "Big open deals", "kind": "grid", "config": {
                "filter": { "and": [
                    { "field": "amount", "op": "gte", "value": 1000 },
                    { "field": "Stage", "op": "eq", "value": "open" },
                    { "field": "$created_at", "op": "gte", "value": "2020-01-01" },
                ] },
                "sort": [{ "field": "Amount", "dir": "desc" }],
                "hidden_fields": ["Amount", "Amount"],
            } })),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let cfg = &r.body["config"];
    let conds = cfg["filter"]["and"].as_array().unwrap();
    assert_eq!(conds[0]["field"], id(&d["amount"]));
    assert_eq!(conds[1]["field"], id(&d["stage"]));
    let open = d["stage"]["options"][0]["id"].clone();
    assert_eq!(conds[1]["value"], open, "label becomes option id");
    assert_eq!(conds[2]["field"], "$created_at");
    assert_eq!(cfg["sort"][0]["field"], id(&d["amount"]));
    assert_eq!(cfg["hidden_fields"], json!([id(&d["amount"])]));
    assert_eq!(r.body["stale"], false);
    assert_eq!(r.body["kind"], "grid");
    assert!(r.body["created_by"].is_string());

    let vid = id(&r.body).to_string();
    let by_id = env.ok(Method::GET, &format!("/api/tables/{t}/views/{vid}"), None).await;
    let by_name = env.ok(Method::GET, &format!("/api/tables/{t}/views/big%20OPEN%20deals"), None).await;
    assert_eq!(by_id, by_name);
    assert_eq!(by_id, r.body);
}

#[tokio::test]
async fn stored_filter_runs_against_records() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    for (n, stage, amount) in [("a", "Open", 5), ("b", "Won", 9), ("c", "Open", 20)] {
        env.ok(
            Method::POST,
            &format!("/api/tables/{t}/records"),
            Some(json!({ "values": { "Name": n, "Stage": stage, "Amount": amount } })),
        )
        .await;
    }
    let v = env
        .ok(
            Method::POST,
            &format!("/api/tables/{t}/views"),
            Some(json!({ "name": "Open", "kind": "grid", "config": {
                "filter": { "field": "Stage", "op": "eq", "value": "Open" },
                "sort": [{ "field": "Amount", "dir": "desc" }] } })),
        )
        .await;
    let enc = |v: &Value| {
        v.to_string()
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect::<String>()
    };
    let uri = format!(
        "/api/tables/{t}/records?filter={}&sort={}",
        enc(&v["config"]["filter"]),
        enc(&v["config"]["sort"])
    );
    let list = env.ok(Method::GET, &uri, None).await;
    let names: Vec<&str> = list["records"]
        .as_array()
        .or_else(|| list["data"].as_array())
        .unwrap()
        .iter()
        .map(|r| r["values"]["Name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["c", "a"]);
}

#[tokio::test]
async fn kanban_needs_a_select_group_by() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let url = format!("/api/tables/{t}/views");
    let post = |config: Value| env.call(Method::POST, &url, Some(json!({ "name": "K", "kind": "kanban", "config": config })));

    assert_eq!(post(json!({})).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(post(json!({ "group_by": "Amount" })).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(post(json!({ "group_by": "Nope" })).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(post(json!({ "group_by": ["Stage"] })).await.status, StatusCode::BAD_REQUEST);
    let ok = post(json!({ "group_by": "Stage" })).await;
    assert_eq!(ok.status, StatusCode::CREATED, "{}", ok.body);
    assert_eq!(ok.body["config"]["group_by"], id(&d["stage"]));

    let grid = env
        .call(Method::POST, &url, Some(json!({ "name": "G", "kind": "grid", "config": { "group_by": "Stage" } })))
        .await;
    assert_eq!(grid.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejects_bad_input() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let url = format!("/api/tables/{t}/views");
    let cases = [
        json!({ "name": "x", "kind": "calendar" }),
        json!({ "name": "  ", "kind": "grid" }),
        json!({ "name": "x", "kind": "grid", "config": [] }),
        json!({ "name": "x", "kind": "grid", "config": { "colour": 1 } }),
        json!({ "name": "x", "kind": "grid", "config": { "filter": { "field": "Nope", "op": "eq", "value": 1 } } }),
        json!({ "name": "x", "kind": "grid", "config": { "filter": { "field": "Amount", "op": "contains", "value": 1 } } }),
        json!({ "name": "x", "kind": "grid", "config": { "filter": { "field": "Stage", "op": "eq", "value": "Maybe" } } }),
        json!({ "name": "x", "kind": "grid", "config": { "sort": [{ "field": "Nope" }] } }),
        json!({ "name": "x", "kind": "grid", "config": { "sort": "Amount" } }),
        json!({ "name": "x", "kind": "grid", "config": { "hidden_fields": ["Nope"] } }),
        json!({ "name": "x", "kind": "grid", "config": { "hidden_fields": ["Name"] } }),
        json!({ "name": "x", "kind": "grid", "config": { "hidden_fields": "Amount" } }),
        json!({ "name": "x", "kind": "grid", "extra": 1 }),
    ];
    for body in cases {
        let r = env.call(Method::POST, &url, Some(body.clone())).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body} -> {}", r.body);
    }
    let none = env.ok(Method::GET, &url, None).await;
    assert_eq!(none, json!([]));
}

#[tokio::test]
async fn names_are_unique_per_table_ignoring_case() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let other = env.ok(Method::POST, "/api/tables", Some(json!({ "name": "People" }))).await;
    let url = format!("/api/tables/{t}/views");
    let make = |n: &str| env.call(Method::POST, &url, Some(json!({ "name": n, "kind": "grid" })));

    assert_eq!(make("All").await.status, StatusCode::CREATED);
    assert_eq!(make("all").await.status, StatusCode::CONFLICT);
    let second = make("Mine").await;
    assert_eq!(second.status, StatusCode::CREATED);

    let rename = env
        .call(Method::PATCH, &format!("{url}/{}", id(&second.body)), Some(json!({ "name": "ALL" })))
        .await;
    assert_eq!(rename.status, StatusCode::CONFLICT);
    let same = env
        .call(Method::PATCH, &format!("{url}/{}", id(&second.body)), Some(json!({ "name": "MINE" })))
        .await;
    assert_eq!(same.status, StatusCode::OK, "renaming to own name in another case is fine");

    let o = id(&other);
    let elsewhere = env
        .call(Method::POST, &format!("/api/tables/{o}/views"), Some(json!({ "name": "All", "kind": "grid" })))
        .await;
    assert_eq!(elsewhere.status, StatusCode::CREATED, "same name in another table is fine");
}

#[tokio::test]
async fn views_are_scoped_to_their_table() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let other = env.ok(Method::POST, "/api/tables", Some(json!({ "name": "People" }))).await;
    let o = id(&other);
    let v = env
        .ok(Method::POST, &format!("/api/tables/{t}/views"), Some(json!({ "name": "All", "kind": "grid" })))
        .await;
    let vid = id(&v);

    for method in [Method::GET, Method::DELETE] {
        let r = env.call(method, &format!("/api/tables/{o}/views/{vid}"), None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND);
    }
    let r = env.call(Method::PATCH, &format!("/api/tables/{o}/views/{vid}"), Some(json!({ "name": "X" }))).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(env.ok(Method::GET, &format!("/api/tables/{o}/views"), None).await, json!([]));
}

#[tokio::test]
async fn patch_updates_name_config_and_position_but_not_kind() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let url = format!("/api/tables/{t}/views");
    let a = env.ok(Method::POST, &url, Some(json!({ "name": "A", "kind": "grid" }))).await;
    let b = env.ok(Method::POST, &url, Some(json!({ "name": "B", "kind": "grid" }))).await;
    assert_eq!((a["position"].as_i64(), b["position"].as_i64()), (Some(0), Some(1)));

    let p = env
        .ok(
            Method::PATCH,
            &format!("{url}/{}", id(&b)),
            Some(json!({ "name": "B2", "position": 0, "config": { "hidden_fields": ["Amount"] } })),
        )
        .await;
    assert_eq!(p["name"], "B2");
    assert_eq!(p["config"]["hidden_fields"], json!([id(&d["amount"])]));

    // `config` replaces the old config; it does not merge into it.
    let p = env.ok(Method::PATCH, &format!("{url}/{}", id(&b)), Some(json!({ "config": {} }))).await;
    assert_eq!(p["config"], json!({}));

    let order: Vec<String> = env
        .ok(Method::GET, &url, None)
        .await
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(order, ["A", "B2"], "ties on position fall back to creation order");

    let kind = env.call(Method::PATCH, &format!("{url}/{}", id(&a)), Some(json!({ "kind": "kanban" }))).await;
    assert_eq!(kind.status, StatusCode::BAD_REQUEST);
    let neg = env.call(Method::PATCH, &format!("{url}/{}", id(&a)), Some(json!({ "position": -1 }))).await;
    assert_eq!(neg.status, StatusCode::BAD_REQUEST);
    // A rejected patch changes nothing, even the parts that were valid.
    let mixed = env
        .call(Method::PATCH, &format!("{url}/{}", id(&a)), Some(json!({ "name": "Z", "config": { "x": 1 } })))
        .await;
    assert_eq!(mixed.status, StatusCode::BAD_REQUEST);
    let a2 = env.ok(Method::GET, &format!("{url}/{}", id(&a)), None).await;
    assert_eq!(a2["name"], "A");
}

#[tokio::test]
async fn delete_removes_only_that_view() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let url = format!("/api/tables/{t}/views");
    let a = env.ok(Method::POST, &url, Some(json!({ "name": "A", "kind": "grid" }))).await;
    env.ok(Method::POST, &url, Some(json!({ "name": "B", "kind": "grid" }))).await;
    env.ok(Method::DELETE, &format!("{url}/{}", id(&a)), None).await;
    assert_eq!(env.call(Method::GET, &format!("{url}/{}", id(&a)), None).await.status, StatusCode::NOT_FOUND);
    assert_eq!(env.call(Method::DELETE, &format!("{url}/{}", id(&a)), None).await.status, StatusCode::NOT_FOUND);
    assert_eq!(env.ok(Method::GET, &url, None).await.as_array().unwrap().len(), 1);
    // The name is free again.
    let again = env.call(Method::POST, &url, Some(json!({ "name": "A", "kind": "grid" }))).await;
    assert_eq!(again.status, StatusCode::CREATED);
}

#[tokio::test]
async fn renames_keep_views_working() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let url = format!("/api/tables/{t}/views");
    let v = env
        .ok(
            Method::POST,
            &url,
            Some(json!({ "name": "Open", "kind": "kanban", "config": {
                "group_by": "Stage",
                "filter": { "field": "Stage", "op": "in", "value": ["Open", "Won"] } } })),
        )
        .await;
    let stage = id(&d["stage"]);
    let opt = |i: usize| d["stage"]["options"][i]["id"].as_str().unwrap().to_string();
    // Rename the field and one option. Options are matched by ID, so all three keep their IDs.
    env.ok(
        Method::PATCH,
        &format!("/api/tables/{t}/fields/{stage}"),
        Some(json!({ "name": "Phase", "options": [
            { "id": opt(0), "label": "In progress" }, { "id": opt(1), "label": "Won" }, { "id": opt(2), "label": "Lost" } ] })),
    )
    .await;
    let after = env.ok(Method::GET, &format!("{url}/{}", id(&v)), None).await;
    assert_eq!(after["stale"], false, "{after}");
    assert_eq!(after["config"], v["config"]);
}

#[tokio::test]
async fn deleting_a_field_marks_views_stale_and_editing_fixes_them() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let url = format!("/api/tables/{t}/views");
    let by_amount = env
        .ok(Method::POST, &url, Some(json!({ "name": "By amount", "kind": "grid", "config": { "sort": [{ "field": "Amount" }] } })))
        .await;
    let board = env
        .ok(Method::POST, &url, Some(json!({ "name": "Board", "kind": "kanban", "config": { "group_by": "Stage" } })))
        .await;
    let plain = env.ok(Method::POST, &url, Some(json!({ "name": "Plain", "kind": "grid" }))).await;

    env.ok(Method::DELETE, &format!("/api/tables/{t}/fields/{}", id(&d["amount"])), None).await;
    env.ok(Method::DELETE, &format!("/api/tables/{t}/fields/{}", id(&d["stage"])), None).await;

    let list = env.ok(Method::GET, &url, None).await;
    let stale = |name: &str| {
        list.as_array().unwrap().iter().find(|v| v["name"] == name).unwrap()["stale"].as_bool().unwrap()
    };
    assert!(stale("By amount"));
    assert!(stale("Board"));
    assert!(!stale("Plain"));
    // The saved config is kept as it was, so nothing is lost silently.
    let kept = env.ok(Method::GET, &format!("{url}/{}", id(&by_amount)), None).await;
    assert_eq!(kept["config"], by_amount["config"]);

    let fixed = env
        .ok(Method::PATCH, &format!("{url}/{}", id(&by_amount)), Some(json!({ "config": {} })))
        .await;
    assert_eq!(fixed["stale"], false);
    let _ = (board, plain);
}

#[tokio::test]
async fn deleted_table_hides_its_views() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();
    let url = format!("/api/tables/{t}/views");
    env.ok(Method::POST, &url, Some(json!({ "name": "A", "kind": "grid" }))).await;
    env.ok(Method::DELETE, &format!("/api/tables/{t}"), None).await;
    assert_eq!(env.call(Method::GET, &url, None).await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        env.call(Method::POST, &url, Some(json!({ "name": "B", "kind": "grid" }))).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn deleting_a_user_keeps_their_views() {
    let env = Env::new().await;
    let d = env.deals().await;
    let t = d["table"].as_str().unwrap();

    let user = env
        .ok(Method::POST, "/api/users", Some(json!({ "email": "bob@example.com", "password": "correct horse" })))
        .await;
    let login = call(
        &env.app,
        Method::POST,
        "/api/auth/login",
        Some(json!({ "email": "bob@example.com", "password": "correct horse" })),
        None,
    )
    .await;
    assert_eq!(login.status, StatusCode::OK, "{}", login.body);
    let bob = login.cookie.unwrap();
    let made = call(
        &env.app,
        Method::POST,
        &format!("/api/tables/{t}/views"),
        Some(json!({ "name": "Bob's", "kind": "grid" })),
        Some(("cookie", bob.as_str())),
    )
    .await;
    assert_eq!(made.status, StatusCode::CREATED, "{}", made.body);
    assert_eq!(made.body["created_by"], id(&user));

    env.ok(Method::DELETE, &format!("/api/users/{}", id(&user)), None).await;
    let v = env.ok(Method::GET, &format!("/api/tables/{t}/views/{}", id(&made.body)), None).await;
    assert!(v["created_by"].is_null());
}

#[tokio::test]
async fn deleting_a_user_who_made_a_table_works() {
    let env = Env::new().await;
    let user = env
        .ok(Method::POST, "/api/users", Some(json!({ "email": "bob@example.com", "password": "correct horse" })))
        .await;
    let login = call(
        &env.app,
        Method::POST,
        "/api/auth/login",
        Some(json!({ "email": "bob@example.com", "password": "correct horse" })),
        None,
    )
    .await;
    let bob = login.cookie.unwrap();
    let t = call(&env.app, Method::POST, "/api/tables", Some(json!({ "name": "Bobs" })), Some(("cookie", bob.as_str()))).await;
    assert_eq!(t.status, StatusCode::CREATED);
    let r = env.call(Method::DELETE, &format!("/api/users/{}", id(&user)), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
}
