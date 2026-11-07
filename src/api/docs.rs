//! Interactive API documentation: the OpenAPI spec and an embedded Swagger UI. Both are public, like `/api/health`.
use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;

use super::AppState;

/// Written by hand in `docs/openapi.json`. The `openapi_covers_every_route` test keeps it in step with the router.
const SPEC: &str = include_str!("../../docs/openapi.json");

#[derive(RustEmbed)]
#[folder = "docs/swagger"]
struct SwaggerAssets;

const PAGE: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<title>CRM API</title>
<link rel="stylesheet" href="/api/docs/assets/swagger-ui.css">
<style>
  html, body { background: #0c0c0e; margin: 0; }
  /* Swagger UI has no dark theme; invert it and rotate the hue back so colours keep their meaning. */
  .swagger-ui { filter: invert(0.9) hue-rotate(180deg); }
  .swagger-ui .topbar { display: none; }
</style>
</head>
<body>
<div id="ui"></div>
<script src="/api/docs/assets/swagger-ui-bundle.js"></script>
<script>
  SwaggerUIBundle({
    url: "/api/openapi.json",
    dom_id: "#ui",
    deepLinking: true,
    persistAuthorization: true,
    docExpansion: "list",
    tryItOutEnabled: true,
  });
</script>
</body>
</html>
"##;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/openapi.json", get(spec))
        .route("/api/docs", get(page))
        .route("/api/docs/assets/{file}", get(asset))
}

async fn spec() -> Response {
    ([(header::CONTENT_TYPE, "application/json")], SPEC).into_response()
}

async fn page() -> Html<&'static str> {
    Html(PAGE)
}

async fn asset(Path(file): Path<String>) -> Response {
    match SwaggerAssets::get(&file) {
        Some(f) => {
            let mime = mime_guess::from_path(&file).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref().to_string())], f.data).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Every route registered by an API module, as `METHOD /path`. Keep in step with the routers.
    const ROUTES: &[&str] = &[
        "get /api/health",
        "get /api/auth/status",
        "post /api/auth/register",
        "post /api/auth/login",
        "post /api/auth/logout",
        "get /api/auth/me",
        "get /api/users",
        "post /api/users",
        "patch /api/users/{id}",
        "delete /api/users/{id}",
        "get /api/api-keys",
        "post /api/api-keys",
        "delete /api/api-keys/{id}",
        "get /api/tables",
        "post /api/tables",
        "get /api/tables/{table}",
        "patch /api/tables/{table}",
        "delete /api/tables/{table}",
        "get /api/tables/{table}/fields",
        "post /api/tables/{table}/fields",
        "patch /api/tables/{table}/fields/{field}",
        "delete /api/tables/{table}/fields/{field}",
        "get /api/tables/{table}/records",
        "post /api/tables/{table}/records",
        "get /api/tables/{table}/records/{id}",
        "patch /api/tables/{table}/records/{id}",
        "delete /api/tables/{table}/records/{id}",
        "get /api/tables/{table}/views",
        "post /api/tables/{table}/views",
        "get /api/tables/{table}/views/{view}",
        "patch /api/tables/{table}/views/{view}",
        "delete /api/tables/{table}/views/{view}",
        "get /api/search",
    ];

    #[test]
    fn openapi_covers_every_route() {
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let mut documented = BTreeSet::new();
        for (path, item) in spec["paths"].as_object().unwrap() {
            for method in item.as_object().unwrap().keys() {
                documented.insert(format!("{method} {path}"));
            }
        }
        let routes: BTreeSet<String> = ROUTES.iter().map(|s| s.to_string()).collect();
        assert_eq!(documented, routes);
    }

    #[test]
    fn swagger_assets_are_embedded() {
        assert!(SwaggerAssets::get("swagger-ui-bundle.js").is_some());
        assert!(SwaggerAssets::get("swagger-ui.css").is_some());
    }
}
