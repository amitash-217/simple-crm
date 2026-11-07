//! Full-text search through the HTTP API: the `q` parameter on record lists, and `GET /api/search`.

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

    async fn add(&self, table: &str, values: Value) -> String {
        let r = self
            .call(Method::POST, &format!("/api/tables/{table}/records"), Some(json!({ "values": values })))
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        r.body["id"].as_str().unwrap().to_string()
    }

    async fn patch(&self, table: &str, id: &str, values: Value) {
        let r = self
            .call(Method::PATCH, &format!("/api/tables/{table}/records/{id}"), Some(json!({ "values": values })))
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    }

    async fn field(&self, table: &str, name: &str, ty: &str) {
        let r = self
            .call(Method::POST, &format!("/api/tables/{table}/fields"), Some(json!({ "name": name, "type": ty })))
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    }

    /// GET /records?q=... with optional extras already encoded as `&key=value` pairs.
    async fn find(&self, table: &str, q: &str, extra: &str) -> Reply {
        self.call(Method::GET, &format!("/api/tables/{table}/records?q={}{extra}", enc(q)), None).await
    }

    /// Names of the Companies matching `q`.
    async fn companies(&self, q: &str) -> Vec<String> {
        let r = self.find("Companies", q, "").await;
        assert_eq!(r.status, StatusCode::OK, "{q}: {}", r.body);
        names(&r)
    }

    async fn search(&self, query: &str) -> Reply {
        self.call(Method::GET, &format!("/api/search?{query}"), None).await
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

/// A `Companies` table with three records.
async fn env() -> Env {
    let (app, dir) = app();
    let cookie = register_admin(&app).await;
    let env = Env { app, cookie, _dir: dir };
    let r = env.call(Method::POST, "/api/tables", Some(json!({ "name": "Companies" }))).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    env.field("Companies", "Notes", "text").await;
    env.field("Companies", "Mobile", "phone").await;
    env.field("Companies", "Amount", "number").await;
    env.add("Companies", json!({ "Name": "Acme Corporation", "Notes": "Loves pineapple", "Mobile": "+1 (415) 555-2671", "Amount": 500 })).await;
    env.add("Companies", json!({ "Name": "Beta Labs", "Notes": "pineapple and rust", "Amount": 50 })).await;
    env.add("Companies", json!({ "Name": "Gamma", "Notes": "Café Münster" })).await;
    env
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[tokio::test]
async fn search_requires_auth() {
    let (app, _dir) = app();
    for uri in ["/api/search?q=x", "/api/tables/Companies/records?q=x"] {
        let r = call(&app, Method::GET, uri, None, None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn words_prefixes_case_and_accents() {
    let env = env().await;
    assert_eq!(env.companies("acme").await, strs(&["Acme Corporation"]));
    assert_eq!(env.companies("ACME").await, strs(&["Acme Corporation"]));
    assert_eq!(env.companies("acm").await, strs(&["Acme Corporation"]), "the last word matches as a prefix");
    assert_eq!(env.companies("corporation acme").await, strs(&["Acme Corporation"]), "word order does not matter");
    assert_eq!(env.companies("pineapple").await, strs(&["Acme Corporation", "Beta Labs"]));
    assert_eq!(env.companies("pineapple rust").await, strs(&["Beta Labs"]), "all words are required");
    assert_eq!(env.companies("cafe").await, strs(&["Gamma"]), "accents are ignored");
    assert_eq!(env.companies("münster").await, strs(&["Gamma"]));
    assert_eq!(env.companies("nothing").await, Vec::<String>::new());
    assert_eq!(env.companies("ine").await, Vec::<String>::new(), "only prefixes match, not the middle of a word");
}

#[tokio::test]
async fn phone_numbers_are_searchable_by_digits() {
    let env = env().await;
    assert_eq!(env.companies("14155552671").await, strs(&["Acme Corporation"]));
    assert_eq!(env.companies("1415").await, strs(&["Acme Corporation"]));
    assert_eq!(env.companies("+1 (415) 555-2671").await, strs(&["Acme Corporation"]));
}

#[tokio::test]
async fn only_text_and_phone_fields_are_indexed() {
    let env = env().await;
    // 500 is Acme's Amount, a number.
    assert_eq!(env.companies("500").await, Vec::<String>::new());
}

#[tokio::test]
async fn search_combines_with_filter_sort_and_paging() {
    let env = env().await;
    let filter = enc(&json!({ "field": "Amount", "op": "gt", "value": 100 }).to_string());
    let r = env.find("Companies", "pineapple", &format!("&filter={filter}")).await;
    assert_eq!(names(&r), strs(&["Acme Corporation"]));

    let sort = enc(&json!([{ "field": "Name", "dir": "desc" }]).to_string());
    let first = env.find("Companies", "pineapple", &format!("&sort={sort}&limit=1")).await;
    assert_eq!(names(&first), strs(&["Beta Labs"]));
    let cursor = first.body["next_cursor"].as_str().unwrap();
    let second = env.find("Companies", "pineapple", &format!("&sort={sort}&limit=1&cursor={cursor}")).await;
    assert_eq!(names(&second), strs(&["Acme Corporation"]));
    assert!(second.body["next_cursor"].is_null());
}

#[tokio::test]
async fn hostile_search_text_is_harmless() {
    let env = env().await;
    // Text with words in it is searched as plain words, whatever the punctuation.
    for q in [
        "acme\" OR *",
        "NEAR(a b)",
        "Notes:rust",
        "-rust",
        "rust AND",
        "'; DROP TABLE users; --",
        "acme*",
        "{Notes} : acme",
    ] {
        let r = env.find("Companies", q, "").await;
        assert_eq!(r.status, StatusCode::OK, "{q}: {}", r.body);
    }
    assert_eq!(env.companies("rust OR acme").await, Vec::<String>::new(), "OR is an ordinary word here");
    assert_eq!(env.companies("notes:acme").await, Vec::<String>::new(), "column filters do not work");
    assert_eq!(env.call(Method::GET, "/api/users", None).await.status, StatusCode::OK);
}

#[tokio::test]
async fn unusable_search_text_is_rejected() {
    let env = env().await;
    for q in ["", "   ", "***", "\"\"", "()-:", "a b c d e f g h i j k"] {
        let r = env.find("Companies", q, "").await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{q:?}");
        assert_eq!(r.body["error"]["code"], "bad_request");
    }
    let long = "x".repeat(101);
    assert_eq!(env.find("Companies", &long, "").await.status, StatusCode::BAD_REQUEST);
    let global = env.search("q=%2A%2A").await;
    assert_eq!(global.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_index_follows_record_changes() {
    let env = env().await;
    let list = env.call(Method::GET, "/api/tables/Companies/records", None).await;
    let acme = list.body["records"][0]["id"].as_str().unwrap().to_string();
    let beta = list.body["records"][1]["id"].as_str().unwrap().to_string();

    env.patch("Companies", &acme, json!({ "Notes": "zeppelin enthusiast" })).await;
    assert_eq!(env.companies("pineapple").await, strs(&["Beta Labs"]), "old text no longer matches");
    assert_eq!(env.companies("zeppelin").await, strs(&["Acme Corporation"]));

    env.patch("Companies", &acme, json!({ "Name": "Acme Holdings" })).await;
    assert_eq!(env.companies("holdings").await, strs(&["Acme Holdings"]));
    assert_eq!(env.companies("corporation").await, Vec::<String>::new());

    // Changing an unindexed field leaves search alone.
    env.patch("Companies", &acme, json!({ "Amount": 1 })).await;
    assert_eq!(env.companies("zeppelin").await, strs(&["Acme Holdings"]));

    // Clearing a value removes it.
    env.patch("Companies", &beta, json!({ "Notes": null })).await;
    assert_eq!(env.companies("pineapple").await, Vec::<String>::new());

    let r = env.call(Method::DELETE, &format!("/api/tables/Companies/records/{acme}"), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(env.companies("zeppelin").await, Vec::<String>::new());

    env.add("Companies", json!({ "Name": "Delta", "Notes": "brand new" })).await;
    assert_eq!(env.companies("brand").await, strs(&["Delta"]));
}

#[tokio::test]
async fn the_index_follows_field_changes() {
    let env = env().await;
    let list = env.call(Method::GET, "/api/tables/Companies/records", None).await;
    let gamma = list.body["records"][2]["id"].as_str().unwrap().to_string();

    // A text field added after the records exist is indexed.
    env.field("Companies", "Summary", "text").await;
    env.patch("Companies", &gamma, json!({ "Summary": "airship" })).await;
    assert_eq!(env.companies("airship").await, strs(&["Gamma"]));

    // Deleting the field hides its text from search, while the other fields still match.
    let r = env.call(Method::DELETE, "/api/tables/Companies/fields/Summary", None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(env.companies("airship").await, Vec::<String>::new());
    assert_eq!(env.companies("gamma").await, strs(&["Gamma"]));

    // The name can be reused, and the new field is indexed on its own.
    env.field("Companies", "Summary", "text").await;
    assert_eq!(env.companies("airship").await, Vec::<String>::new(), "the old column's data stays out of the index");
    env.patch("Companies", &gamma, json!({ "Summary": "dirigible" })).await;
    assert_eq!(env.companies("dirigible").await, strs(&["Gamma"]));

    // A new phone field is indexed too.
    env.field("Companies", "Fax", "phone").await;
    env.patch("Companies", &gamma, json!({ "Fax": "+442079460958" })).await;
    assert_eq!(env.companies("442079460958").await, strs(&["Gamma"]));
}

#[tokio::test]
async fn global_search_groups_matches_by_table() {
    let env = env().await;
    let r = env.call(Method::POST, "/api/tables", Some(json!({ "name": "People" }))).await;
    assert_eq!(r.status, StatusCode::CREATED);
    env.field("People", "Bio", "text").await;
    let pete = env.add("People", json!({ "Name": "Pineapple Pete", "Bio": "grows pineapple for fun" })).await;
    env.add("People", json!({ "Name": "Quinn", "Bio": "likes pears" })).await;
    env.call(Method::POST, "/api/tables", Some(json!({ "name": "Empty" }))).await;

    let r = env.search("q=pineapple").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let results = r.body["results"].as_array().unwrap();
    let tables: Vec<&str> = results.iter().map(|t| t["table"]["name"].as_str().unwrap()).collect();
    assert_eq!(tables, ["Companies", "People"], "tables without matches are left out");
    assert!(results[0]["table"]["id"].is_string());

    let companies: Vec<&str> = results[0]["matches"].as_array().unwrap().iter().map(|m| m["title"].as_str().unwrap()).collect();
    assert_eq!(companies.len(), 2);
    assert!(companies.contains(&"Acme Corporation") && companies.contains(&"Beta Labs"));

    let hit = &results[1]["matches"][0];
    assert_eq!(hit["id"], pete.as_str());
    assert_eq!(hit["title"], "Pineapple Pete");
    let snippet = hit["snippet"].as_str().unwrap();
    assert!(snippet.contains('«') && snippet.contains('»'), "matched words are marked: {snippet}");

    // limit applies per table.
    let r = env.search("q=pineapple&limit=1").await;
    for table in r.body["results"].as_array().unwrap() {
        assert_eq!(table["matches"].as_array().unwrap().len(), 1);
    }

    // The record the match points to can be fetched.
    let got = env.call(Method::GET, &format!("/api/tables/People/records/{pete}"), None).await;
    assert_eq!(got.status, StatusCode::OK);

    assert_eq!(env.search("q=nothing").await.body["results"], json!([]));
}

#[tokio::test]
async fn global_search_skips_deleted_tables_and_validates_input() {
    let env = env().await;
    let r = env.call(Method::DELETE, "/api/tables/Companies", None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(env.search("q=pineapple").await.body["results"], json!([]));

    for query in ["", "q=", "limit=5", "q=x&limit=0", "q=x&limit=51", "q=x&bogus=1"] {
        let r = env.search(query).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{query}: {}", r.body);
    }
}

#[tokio::test]
async fn a_table_created_before_any_fields_is_searchable_by_name() {
    let (app, dir) = app();
    let cookie = register_admin(&app).await;
    let env = Env { app, cookie, _dir: dir };
    env.call(Method::POST, "/api/tables", Some(json!({ "name": "Leads" }))).await;
    env.add("Leads", json!({ "Name": "Solo prospect" })).await;
    let r = env.find("Leads", "prospect", "").await;
    assert_eq!(names(&r), strs(&["Solo prospect"]));
}
