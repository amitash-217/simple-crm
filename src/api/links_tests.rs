//! Relation and multi-select fields, end to end through the HTTP API.

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

    async fn table(&self, name: &str) -> String {
        let r = self.call(Method::POST, "/api/tables", Some(json!({ "name": name }))).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        r.body["id"].as_str().unwrap().to_string()
    }

    async fn field(&self, table: &str, body: Value) -> Reply {
        self.call(Method::POST, &format!("/api/tables/{table}/fields"), Some(body)).await
    }

    async fn record(&self, table: &str, values: Value) -> Reply {
        self.call(Method::POST, &format!("/api/tables/{table}/records"), Some(json!({ "values": values }))).await
    }

    /// Creates a record, expects success, and returns its ID.
    async fn add(&self, table: &str, values: Value) -> String {
        let r = self.record(table, values).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        r.body["id"].as_str().unwrap().to_string()
    }

    async fn patch(&self, table: &str, id: &str, values: Value) -> Reply {
        self.call(Method::PATCH, &format!("/api/tables/{table}/records/{id}"), Some(json!({ "values": values }))).await
    }

    async fn get(&self, table: &str, id: &str) -> Value {
        let r = self.call(Method::GET, &format!("/api/tables/{table}/records/{id}"), None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        r.body
    }

    async fn delete(&self, table: &str, id: &str) {
        let r = self.call(Method::DELETE, &format!("/api/tables/{table}/records/{id}"), None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }

    async fn list(&self, table: &str, filter: Option<Value>, sort: Option<Value>, limit: Option<usize>, cursor: Option<&str>) -> Reply {
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
        self.call(Method::GET, &format!("/api/tables/{table}/records?{}", parts.join("&")), None).await
    }

    /// Names of the People matching `filter`.
    async fn people(&self, filter: Value) -> Vec<String> {
        let r = self.list("People", Some(filter), None, None, None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        names(&r)
    }
}

fn names(r: &Reply) -> Vec<String> {
    r.body["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rec| rec["values"]["Name"].as_str().unwrap().to_string())
        .collect()
}

fn cond(field: &str, op: &str, value: Value) -> Value {
    json!({ "field": field, "op": op, "value": value })
}

struct Crm {
    env: Env,
    companies: String,
    projects: String,
    /// Two companies and three projects.
    a: String,
    b: String,
    p: [String; 3],
}

/// Companies, Projects, and People with a to-one `Company`, a multi-select `Skills`, and a to-many `Projects`.
async fn crm() -> Crm {
    let (app, dir) = app();
    let cookie = register_admin(&app).await;
    let env = Env { app, cookie, _dir: dir };
    let companies = env.table("Companies").await;
    let projects = env.table("Projects").await;
    env.table("People").await;
    for body in [
        json!({ "name": "Company", "type": "relation", "config": { "table": "Companies" } }),
        json!({ "name": "Skills", "type": "multi_select", "options": [{ "label": "Rust" }, { "label": "Go" }, { "label": "SQL" }] }),
        json!({ "name": "Projects", "type": "relation", "config": { "table": "projects", "many": true } }),
    ] {
        let r = env.field("People", body).await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    }
    let a = env.add("Companies", json!({ "Name": "Acme" })).await;
    let b = env.add("Companies", json!({ "Name": "Beta" })).await;
    let p = [
        env.add("Projects", json!({ "Name": "P1" })).await,
        env.add("Projects", json!({ "Name": "P2" })).await,
        env.add("Projects", json!({ "Name": "P3" })).await,
    ];
    Crm { env, companies, projects, a, b, p }
}

#[tokio::test]
async fn relation_and_multi_select_fields_are_described_in_the_catalog() {
    let crm = crm().await;
    let table = crm.env.call(Method::GET, "/api/tables/People", None).await;
    let fields = table.body["fields"].as_array().unwrap();
    let get = |name: &str| fields.iter().find(|f| f["name"] == name).unwrap();

    assert_eq!(get("Company")["type"], "relation");
    assert_eq!(get("Company")["config"]["table"], crm.companies.as_str(), "stores the target's ID, not the name given");
    assert_eq!(get("Company")["config"]["many"], false);
    assert_eq!(get("Projects")["config"]["table"], crm.projects.as_str());
    assert_eq!(get("Projects")["config"]["many"], true);
    assert_eq!(get("Skills")["type"], "multi_select");
    assert_eq!(get("Skills")["options"].as_array().unwrap().len(), 3);
    assert!(get("Skills").get("sql_name").is_none());
}

#[tokio::test]
async fn invalid_link_fields_are_rejected() {
    let crm = crm().await;
    let env = &crm.env;
    for body in [
        json!({ "name": "X", "type": "relation" }),
        json!({ "name": "X", "type": "relation", "config": { "table": "Nope" } }),
        json!({ "name": "X", "type": "relation", "config": { "table": "Companies", "bogus": 1 } }),
        json!({ "name": "X", "type": "text", "config": { "table": "Companies" } }),
        json!({ "name": "X", "type": "text", "config": { "many": true } }),
        json!({ "name": "X", "type": "relation", "config": { "table": "Companies" }, "options": [{ "label": "a" }] }),
        json!({ "name": "X", "type": "multi_select", "options": [{ "label": "a" }, { "label": "A" }] }),
    ] {
        let r = env.field("People", body.clone()).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body}: {}", r.body);
    }
    // A failed field leaves nothing behind: the same name is still free.
    let r = env.field("People", json!({ "name": "X", "type": "multi_select" })).await;
    assert_eq!(r.status, StatusCode::CREATED);
}

#[tokio::test]
async fn links_round_trip() {
    let crm = crm().await;
    let env = &crm.env;
    let [p1, _, p3] = &crm.p;

    let created = env
        .record("People", json!({ "Name": "Ann", "Company": crm.a, "Skills": ["rust", "SQL"], "Projects": [p3, p1] }))
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let v = &created.body["values"];
    assert_eq!(v["Company"], crm.a.as_str());
    assert_eq!(v["Skills"], json!(["Rust", "SQL"]), "labels come back as defined, in the order sent");
    assert_eq!(v["Projects"], json!([p3, p1]), "link order is kept");
    let id = created.body["id"].as_str().unwrap();
    assert_eq!(env.get("People", id).await, created.body);

    // A record that sets none of them has empty values.
    let bare = env.record("People", json!({ "Name": "Bob" })).await;
    assert_eq!(bare.body["values"]["Company"], Value::Null);
    assert_eq!(bare.body["values"]["Skills"], json!([]));
    assert_eq!(bare.body["values"]["Projects"], json!([]));

    // PATCH replaces only the fields it names.
    let patched = env.patch("People", id, json!({ "Skills": ["Go"] })).await;
    assert_eq!(patched.status, StatusCode::OK, "{}", patched.body);
    assert_eq!(patched.body["values"]["Skills"], json!(["Go"]));
    assert_eq!(patched.body["values"]["Projects"], json!([p3, p1]));
    assert_eq!(patched.body["values"]["Company"], crm.a.as_str());

    let cleared = env.patch("People", id, json!({ "Company": null, "Projects": [], "Skills": null })).await;
    assert_eq!(cleared.body["values"]["Company"], Value::Null);
    assert_eq!(cleared.body["values"]["Projects"], json!([]));
    assert_eq!(cleared.body["values"]["Skills"], json!([]));

    // The list endpoint carries the same shapes.
    let listed = env.list("People", None, None, None, None).await;
    assert_eq!(listed.body["records"][1]["values"]["Skills"], json!([]));
}

#[tokio::test]
async fn invalid_links_are_rejected_without_writing() {
    let crm = crm().await;
    let env = &crm.env;
    let p1 = &crm.p[0];
    for values in [
        json!({ "Company": "no-such-record" }),
        json!({ "Company": 5 }),
        json!({ "Company": [crm.a] }),
        json!({ "Company": "" }),
        json!({ "Projects": p1 }),
        json!({ "Projects": ["no-such-record"] }),
        json!({ "Projects": [p1, "no-such-record"] }),
        json!({ "Projects": [p1, p1] }),
        json!({ "Projects": [1] }),
        json!({ "Projects": [""] }),
        json!({ "Skills": ["Cobol"] }),
        json!({ "Skills": ["Rust", "rust"] }),
        json!({ "Skills": "Rust" }),
        // A record ID from the wrong table is not found in the target table.
        json!({ "Company": p1 }),
    ] {
        let r = env.record("People", values.clone()).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{values}: {}", r.body);
        assert_eq!(r.body["error"]["code"], "bad_request");
    }

    let all = env.list("People", None, None, None, None).await;
    assert!(all.body["records"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn deleting_records_unlinks_them() {
    let crm = crm().await;
    let env = &crm.env;
    let [p1, p2, _] = &crm.p;
    let ann = env.add("People", json!({ "Name": "Ann", "Company": crm.a, "Projects": [p1, p2] })).await;
    let bob = env.add("People", json!({ "Name": "Bob", "Company": crm.a, "Projects": [p2] })).await;

    // Deleting a company empties the to-one link on everyone who had it.
    env.delete("Companies", &crm.a).await;
    assert_eq!(env.get("People", &ann).await["values"]["Company"], Value::Null);
    assert_eq!(env.get("People", &bob).await["values"]["Company"], Value::Null);

    // Deleting a project removes it from the to-many lists.
    env.delete("Projects", p2).await;
    assert_eq!(env.get("People", &ann).await["values"]["Projects"], json!([p1]));
    assert_eq!(env.get("People", &bob).await["values"]["Projects"], json!([]));

    // Deleting a person leaves the projects alone.
    env.delete("People", &ann).await;
    let projects = env.list("Projects", None, None, None, None).await;
    assert_eq!(names(&projects), ["P1", "P3"]);
    assert_eq!(env.get("People", &bob).await["values"]["Name"], "Bob");
}

#[tokio::test]
async fn removing_a_multi_select_option_unlinks_it() {
    let crm = crm().await;
    let env = &crm.env;
    let ann = env.add("People", json!({ "Name": "Ann", "Skills": ["Rust", "Go"] })).await;
    let bob = env.add("People", json!({ "Name": "Bob", "Skills": ["Go"] })).await;

    let field = env.call(Method::GET, "/api/tables/People/fields", None).await;
    let skills = field.body.as_array().unwrap().iter().find(|f| f["name"] == "Skills").unwrap();
    let rust = skills["options"][0]["id"].as_str().unwrap();
    let sql = skills["options"][2]["id"].as_str().unwrap();

    // Keep Rust and SQL, drop Go.
    let r = env
        .call(
            Method::PATCH,
            "/api/tables/People/fields/Skills",
            Some(json!({ "options": [{ "id": rust, "label": "Rust" }, { "id": sql, "label": "SQL" }] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(env.get("People", &ann).await["values"]["Skills"], json!(["Rust"]));
    assert_eq!(env.get("People", &bob).await["values"]["Skills"], json!([]));
}

#[tokio::test]
async fn filters_on_links() {
    let crm = crm().await;
    let env = &crm.env;
    let [p1, p2, _] = &crm.p;
    env.add("People", json!({ "Name": "P1", "Company": crm.a, "Skills": ["Rust"], "Projects": [p1] })).await;
    env.add("People", json!({ "Name": "P2", "Company": crm.b, "Skills": ["Rust", "Go"], "Projects": [p1, p2] })).await;
    env.add("People", json!({ "Name": "P3" })).await;
    env.add("People", json!({ "Name": "P4", "Company": crm.a, "Skills": ["SQL"], "Projects": [p2] })).await;
    let ids = |v: Vec<&str>| v.into_iter().map(String::from).collect::<Vec<_>>();

    // To-one relation.
    assert_eq!(env.people(cond("Company", "eq", json!(crm.a))).await, ids(vec!["P1", "P4"]));
    assert_eq!(env.people(cond("Company", "neq", json!(crm.a))).await, ids(vec!["P2", "P3"]));
    assert_eq!(env.people(cond("Company", "in", json!([crm.a, crm.b]))).await, ids(vec!["P1", "P2", "P4"]));
    assert_eq!(env.people(json!({ "field": "Company", "op": "is_empty" })).await, ids(vec!["P3"]));
    assert_eq!(env.people(json!({ "field": "Company", "op": "is_not_empty" })).await, ids(vec!["P1", "P2", "P4"]));

    // Multi-select.
    assert_eq!(env.people(cond("Skills", "has", json!("rust"))).await, ids(vec!["P1", "P2"]));
    assert_eq!(env.people(cond("Skills", "has_any", json!(["Go", "SQL"]))).await, ids(vec!["P2", "P4"]));
    assert_eq!(env.people(cond("Skills", "has_any", json!([]))).await, Vec::<String>::new());
    assert_eq!(env.people(json!({ "field": "Skills", "op": "is_empty" })).await, ids(vec!["P3"]));
    assert_eq!(env.people(json!({ "field": "Skills", "op": "is_not_empty" })).await, ids(vec!["P1", "P2", "P4"]));

    // To-many relation.
    assert_eq!(env.people(cond("Projects", "has", json!(p1))).await, ids(vec!["P1", "P2"]));
    assert_eq!(env.people(cond("Projects", "has_any", json!([p2]))).await, ids(vec!["P2", "P4"]));
    assert_eq!(env.people(json!({ "field": "Projects", "op": "is_empty" })).await, ids(vec!["P3"]));

    // Combined with other conditions.
    let both = json!({ "and": [cond("Skills", "has", json!("Rust")), cond("Company", "eq", json!(crm.a))] });
    assert_eq!(env.people(both).await, ids(vec!["P1"]));
    let either = json!({ "or": [cond("Skills", "has", json!("SQL")), { "field": "Projects", "op": "is_empty" }] });
    assert_eq!(env.people(either).await, ids(vec!["P3", "P4"]));
}

#[tokio::test]
async fn bad_filters_on_links_are_rejected() {
    let crm = crm().await;
    let env = &crm.env;
    for filter in [
        cond("Skills", "contains", json!("R")),
        cond("Skills", "eq", json!("Rust")),
        cond("Skills", "has", json!("Cobol")),
        cond("Skills", "has", json!(1)),
        cond("Skills", "has_any", json!("Rust")),
        json!({ "field": "Skills", "op": "has" }),
        cond("Projects", "has", Value::Null),
        cond("Company", "contains", json!("x")),
        cond("Company", "gt", json!("x")),
        cond("Company", "eq", json!(5)),
        cond("Name", "has", json!("x")),
    ] {
        let r = env.list("People", Some(filter.clone()), None, None, None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{filter}: {}", r.body);
    }
}

#[tokio::test]
async fn link_fields_cannot_be_sorted() {
    let crm = crm().await;
    for field in ["Company", "Skills", "Projects"] {
        let r = crm.env.list("People", None, Some(json!([{ "field": field }])), None, None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{field}");
    }
}

#[tokio::test]
async fn pagination_works_with_link_filters() {
    let crm = crm().await;
    let env = &crm.env;
    for name in ["A", "B", "C", "D", "E"] {
        let skills = if name == "C" { json!(["Go"]) } else { json!(["Rust"]) };
        env.add("People", json!({ "Name": name, "Skills": skills })).await;
    }
    let filter = cond("Skills", "has", json!("Rust"));
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..5 {
        let r = env.list("People", Some(filter.clone()), None, Some(3), cursor.as_deref()).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
        seen.extend(names(&r));
        match r.body["next_cursor"].as_str() {
            Some(c) => cursor = Some(c.to_string()),
            None => break,
        }
    }
    assert_eq!(seen, ["A", "B", "D", "E"]);
}

#[tokio::test]
async fn a_table_can_relate_to_itself() {
    let crm = crm().await;
    let env = &crm.env;
    let r = env.field("People", json!({ "name": "Manager", "type": "relation", "config": { "table": "People" } })).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let r = env.field("People", json!({ "name": "Reports", "type": "relation", "config": { "table": "People", "many": true } })).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);

    let boss = env.add("People", json!({ "Name": "Boss" })).await;
    let ann = env.add("People", json!({ "Name": "Ann", "Manager": boss })).await;
    env.patch("People", &boss, json!({ "Reports": [ann] })).await;
    assert_eq!(env.get("People", &boss).await["values"]["Reports"], json!([ann]));

    // Removing the manager clears the link.
    env.delete("People", &boss).await;
    assert_eq!(env.get("People", &ann).await["values"]["Manager"], Value::Null);
}

#[tokio::test]
async fn deleting_the_target_table_keeps_reads_working_and_blocks_new_links() {
    let crm = crm().await;
    let env = &crm.env;
    let ann = env.add("People", json!({ "Name": "Ann", "Company": crm.a })).await;

    let r = env.call(Method::DELETE, "/api/tables/Companies", None).await;
    assert_eq!(r.status, StatusCode::OK);

    assert_eq!(env.get("People", &ann).await["values"]["Company"], crm.a.as_str());
    let blocked = env.patch("People", &ann, json!({ "Company": crm.b })).await;
    assert_eq!(blocked.status, StatusCode::BAD_REQUEST, "{}", blocked.body);
    // Clearing the link is still fine.
    let cleared = env.patch("People", &ann, json!({ "Company": null })).await;
    assert_eq!(cleared.status, StatusCode::OK, "{}", cleared.body);
}

#[tokio::test]
async fn deleting_a_link_field_hides_it_and_frees_the_name() {
    let crm = crm().await;
    let env = &crm.env;
    let ann = env.add("People", json!({ "Name": "Ann", "Skills": ["Rust"], "Projects": [crm.p[0]] })).await;

    for field in ["Skills", "Projects"] {
        let r = env.call(Method::DELETE, &format!("/api/tables/People/fields/{field}"), None).await;
        assert_eq!(r.status, StatusCode::OK);
    }
    let rec = env.get("People", &ann).await;
    assert!(rec["values"].get("Skills").is_none() && rec["values"].get("Projects").is_none());
    let r = env.field("People", json!({ "name": "Skills", "type": "text" })).await;
    assert_eq!(r.status, StatusCode::CREATED);
}

#[tokio::test]
async fn a_to_many_relation_adds_a_field_on_the_other_table_that_stays_in_sync() {
    let crm = crm().await;
    let env = &crm.env;
    let ann = env.add("People", json!({ "Name": "Ann", "Projects": [crm.p[0], crm.p[1]] })).await;

    // The partner is named after the table the relation was made on.
    let back = env.get("Projects", &crm.p[0]).await;
    assert_eq!(back["values"]["People"], json!([ann]));

    // Setting links from the other side shows up on the first table.
    let bob = env.add("People", json!({ "Name": "Bob" })).await;
    let r = env.patch("Projects", &crm.p[2], json!({ "People": [bob, ann] })).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(env.get("People", &bob).await["values"]["Projects"], json!([crm.p[2]]));
    assert_eq!(env.get("People", &ann).await["values"]["Projects"], json!([crm.p[0], crm.p[1], crm.p[2]]));

    // Filters work on the partner too.
    let hit = env.list("Projects", Some(cond("People", "has", json!(bob))), None, None, None).await;
    assert_eq!(names(&hit), ["P3"]);

    // Deleting a record unlinks it on both sides.
    env.delete("People", &ann).await;
    assert_eq!(env.get("Projects", &crm.p[0]).await["values"]["People"], json!([]));

    // Deleting one field deletes its partner.
    let r = env.call(Method::DELETE, "/api/tables/People/fields/Projects", None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(env.get("Projects", &crm.p[2]).await["values"].get("People").is_none());
}

#[tokio::test]
async fn the_partner_field_is_optional_and_never_made_for_self_links() {
    let crm = crm().await;
    let env = &crm.env;
    let r = env
        .field("Companies", json!({ "name": "Deals", "type": "relation", "config": { "table": "Projects", "many": true, "create_related": false } }))
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let id = env.add("Companies", json!({ "Name": "C" })).await;
    let _ = id;
    assert!(env.get("Projects", &crm.p[0]).await["values"].get("Companies").is_none());

    let r = env.field("Companies", json!({ "name": "Parent", "type": "relation", "config": { "table": "Companies", "many": true } })).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let fields = env.call(Method::GET, "/api/tables/Companies/fields", None).await;
    assert_eq!(fields.body.as_array().unwrap().len(), 3);

    for config in [
        json!({ "table": "Projects", "create_related": true }),
        json!({ "table": "Companies", "many": true, "create_related": true }),
        json!({ "table": "Projects", "many": true, "related": "x" }),
    ] {
        let r = env.field("Companies", json!({ "name": "Bad", "type": "relation", "config": config })).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    }
}
