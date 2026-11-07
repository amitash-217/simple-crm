use axum::http::{Method, StatusCode};
use serde_json::{json, Value};

use super::auth_tests::{app, call, register_admin};

fn find<'a>(items: &'a Value, name: &str) -> &'a Value {
    items
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap_or_else(|| panic!("no item named {name}"))
}

#[tokio::test]
async fn schema_endpoints_require_auth() {
    let (app, _dir) = app();
    for (method, uri) in [
        (Method::GET, "/api/tables"),
        (Method::POST, "/api/tables"),
        (Method::GET, "/api/tables/x"),
        (Method::GET, "/api/tables/x/fields"),
    ] {
        let r = call(&app, method, uri, Some(json!({ "name": "x" })), None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn table_lifecycle() {
    let (app, _dir) = app();
    let cookie = register_admin(&app).await;
    let auth = Some(("cookie", cookie.as_str()));

    let created = call(&app, Method::POST, "/api/tables", Some(json!({ "name": "Deals" })), auth).await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.body["name"], "Deals");
    assert_eq!(created.body["fields"][0]["name"], "Name");
    assert_eq!(created.body["fields"][0]["type"], "text");
    assert_eq!(created.body["fields"][0]["is_primary"], true);
    assert!(created.body.get("sql_name").is_none(), "internal identifiers stay internal");
    let id = created.body["id"].as_str().unwrap().to_string();

    let dup = call(&app, Method::POST, "/api/tables", Some(json!({ "name": "deals" })), auth).await;
    assert_eq!(dup.status, StatusCode::CONFLICT);

    let list = call(&app, Method::GET, "/api/tables", None, auth).await;
    assert_eq!(list.body.as_array().unwrap().len(), 1);

    // By name (any case) and by ID.
    for key in ["deals", id.as_str()] {
        let r = call(&app, Method::GET, &format!("/api/tables/{key}"), None, auth).await;
        assert_eq!(r.status, StatusCode::OK, "{key}");
        assert_eq!(r.body["id"], id.as_str());
    }

    let renamed = call(&app, Method::PATCH, "/api/tables/Deals", Some(json!({ "name": "Opportunities" })), auth).await;
    assert_eq!(renamed.status, StatusCode::OK);
    assert_eq!(renamed.body["id"], id.as_str());
    let gone = call(&app, Method::GET, "/api/tables/Deals", None, auth).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);

    let deleted = call(&app, Method::DELETE, "/api/tables/Opportunities", None, auth).await;
    assert_eq!(deleted.status, StatusCode::OK);
    let gone = call(&app, Method::GET, &format!("/api/tables/{id}"), None, auth).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
    let list = call(&app, Method::GET, "/api/tables", None, auth).await;
    assert!(list.body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn bad_bodies_get_json_errors() {
    let (app, _dir) = app();
    let cookie = register_admin(&app).await;
    let auth = Some(("cookie", cookie.as_str()));

    let empty = call(&app, Method::POST, "/api/tables", Some(json!({ "name": "   " })), auth).await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    assert_eq!(empty.body["error"]["code"], "bad_request");

    let missing = call(&app, Method::POST, "/api/tables", Some(json!({})), auth).await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
    assert_eq!(missing.body["error"]["code"], "bad_request");

    let extra = call(&app, Method::POST, "/api/tables", Some(json!({ "name": "A", "sql_name": "x" })), auth).await;
    assert_eq!(extra.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn field_lifecycle() {
    let (app, _dir) = app();
    let cookie = register_admin(&app).await;
    let auth = Some(("cookie", cookie.as_str()));
    call(&app, Method::POST, "/api/tables", Some(json!({ "name": "Deals" })), auth).await;

    let amount = call(
        &app,
        Method::POST,
        "/api/tables/Deals/fields",
        Some(json!({ "name": "Amount", "type": "number", "config": { "integer": true } })),
        auth,
    )
    .await;
    assert_eq!(amount.status, StatusCode::CREATED);
    assert_eq!(amount.body["type"], "number");
    assert_eq!(amount.body["config"]["integer"], true);
    assert_eq!(amount.body["is_primary"], false);
    assert_eq!(amount.body["position"], 1);
    assert!(amount.body.get("sql_name").is_none());

    let stage = call(
        &app,
        Method::POST,
        "/api/tables/Deals/fields",
        Some(json!({
            "name": "Stage",
            "type": "select",
            "options": [{ "label": "Open", "color": "#3b82f6" }, { "label": "Won" }]
        })),
        auth,
    )
    .await;
    assert_eq!(stage.status, StatusCode::CREATED);
    assert_eq!(stage.body["options"][0]["label"], "Open");
    assert_eq!(stage.body["options"][0]["color"], "#3b82f6");
    assert_eq!(stage.body["options"][1]["position"], 1);

    // The table detail carries the fields in order.
    let table = call(&app, Method::GET, "/api/tables/Deals", None, auth).await;
    let names: Vec<&str> = table.body["fields"].as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Name", "Amount", "Stage"]);

    // Rename, then replace the options: keep Open, drop Won, add Lost.
    let open_id = stage.body["options"][0]["id"].as_str().unwrap();
    let patched = call(
        &app,
        Method::PATCH,
        "/api/tables/Deals/fields/Stage",
        Some(json!({ "name": "Pipeline stage", "options": [{ "id": open_id, "label": "Open" }, { "label": "Lost" }] })),
        auth,
    )
    .await;
    assert_eq!(patched.status, StatusCode::OK);
    assert_eq!(patched.body["name"], "Pipeline stage");
    assert_eq!(patched.body["options"].as_array().unwrap().len(), 2);
    assert_eq!(patched.body["options"][0]["id"], open_id);
    assert_eq!(patched.body["options"][1]["label"], "Lost");

    let fields = call(&app, Method::GET, "/api/tables/Deals/fields", None, auth).await;
    find(&fields.body, "Pipeline stage");

    let deleted = call(&app, Method::DELETE, "/api/tables/Deals/fields/Amount", None, auth).await;
    assert_eq!(deleted.status, StatusCode::OK);
    let fields = call(&app, Method::GET, "/api/tables/Deals/fields", None, auth).await;
    assert_eq!(fields.body.as_array().unwrap().len(), 2);
    let gone = call(&app, Method::DELETE, "/api/tables/Deals/fields/Amount", None, auth).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn field_errors() {
    let (app, _dir) = app();
    let cookie = register_admin(&app).await;
    let auth = Some(("cookie", cookie.as_str()));
    call(&app, Method::POST, "/api/tables", Some(json!({ "name": "Deals" })), auth).await;
    let add = |body: Value| {
        let app = app.clone();
        let cookie = cookie.clone();
        async move {
            call(&app, Method::POST, "/api/tables/Deals/fields", Some(body), Some(("cookie", cookie.as_str()))).await
        }
    };

    assert_eq!(add(json!({ "name": "name", "type": "text" })).await.status, StatusCode::CONFLICT);
    // A relation without a target table is invalid too.
    for ty in ["relation", "bogus"] {
        let r = add(json!({ "name": "X", "type": ty })).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{ty}");
    }
    assert_eq!(add(json!({ "name": "X" })).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        add(json!({ "name": "X", "type": "text", "options": [{ "label": "a" }] })).await.status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        add(json!({ "name": "X", "type": "select", "options": [{ "label": "a" }, { "label": "A" }] })).await.status,
        StatusCode::BAD_REQUEST
    );

    // Changing a field's type is not supported.
    let r = call(&app, Method::PATCH, "/api/tables/Deals/fields/Name", Some(json!({ "type": "number" })), auth).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    let primary = call(&app, Method::DELETE, "/api/tables/Deals/fields/Name", None, auth).await;
    assert_eq!(primary.status, StatusCode::BAD_REQUEST);

    let missing = call(&app, Method::GET, "/api/tables/Nope/fields", None, auth).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn api_keys_can_manage_schema() {
    let (app, _dir) = app();
    let cookie = register_admin(&app).await;
    let key = call(&app, Method::POST, "/api/api-keys", Some(json!({ "name": "ci" })), Some(("cookie", &cookie))).await;
    let bearer = format!("Bearer {}", key.body["key"].as_str().unwrap());

    let created = call(&app, Method::POST, "/api/tables", Some(json!({ "name": "Leads" })), Some(("authorization", &bearer))).await;
    assert_eq!(created.status, StatusCode::CREATED);
}
