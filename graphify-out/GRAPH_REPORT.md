# Graph Report - crm-p  (2026-11-11)

## Corpus Check
- Corpus is ~43,780 words - fits in a single context window. You may not need a graph.

## Summary
- 797 nodes · 2407 edges · 23 communities (19 shown, 4 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 97 edges (avg confidence: 0.84)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- API Entrypoint & Routing
- Web Frontend API Client
- Records & Link Queries
- Catalog Schema Management
- Auth & API Keys
- DESIGN.md Architecture
- Link Tests
- Search Index & Fields
- Records Tests
- Saved Views
- Search Tests
- Web Dependencies
- Views Tests
- DB Connection Pool
- Auth Tests
- OpenAPI Docs Route
- Router & Health
- TypeScript Config
- Tables Tests
- Config
- Server Startup
- Community 22

## God Nodes (most connected - your core abstractions)
1. `AppError` - 133 edges
2. `AppState` - 58 edges
3. `FieldOut` - 48 edges
4. `AuthUser` - 35 edges
5. `create_table()` - 29 edges
6. `Icon()` - 29 edges
7. `register_admin()` - 26 edges
8. `resolve_table()` - 26 edges
9. `ident()` - 25 edges
10. `add_field()` - 25 edges

## Surprising Connections (you probably didn't know these)
- `Cross-compile from Mac (zigbuild, musl)` --semantically_similar_to--> `Docker musl static build`  [INFERRED] [semantically similar]
  docs/LINUX.md → README.md
- `session.txt (claude resume command)` --conceptually_related_to--> `DESIGN.md (CRM Design)`  [AMBIGUOUS]
  session.txt → DESIGN.md
- `Backups (sqlite3 .backup)` --conceptually_related_to--> `Concurrency model (WAL, single writer, read pool)`  [INFERRED]
  docs/LINUX.md → DESIGN.md
- `web/index.html (SPA entry)` --implements--> `React SPA frontend`  [INFERRED]
  web/index.html → DESIGN.md
- `app()` --calls--> `router()`  [INFERRED]
  src/api/auth_tests.rs → src/api/mod.rs

## Import Cycles
- 1-file cycle: `src/api/search.rs -> src/api/search.rs`

## Hyperedges (group relationships)
- **Release and deployment pipeline** — scripts_release, readme_docker_build, docs_linux_cross_compile, docs_linux_native_build, design_release_build [INFERRED 0.85]
- **Runtime-defined schema storage** — design_catalog_tables, design_internal_id_naming, design_schema_changes, design_field_types, design_relations [INFERRED 0.85]

## Communities (23 total, 4 thin omitted)

### Community 0 - "API Entrypoint & Routing"
Cohesion: 0.05
Nodes (57): PLACEHOLDER, create(), CreateKey, KeyOut, list(), remove(), routes(), ApiJson (+49 more)

### Community 1 - "Web Frontend API Client"
Cohesion: 0.06
Nodes (91): react, api, ApiError, errorMessage(), ListArgs, listRecords(), request(), App() (+83 more)

### Community 2 - "Records & Link Queries"
Cohesion: 0.06
Nodes (74): FieldOut, ident(), AppError, BadRequest, Conflict, Db, Forbidden, Internal (+66 more)

### Community 3 - "Catalog Schema Management"
Cohesion: 0.09
Nodes (58): new_id(), add_field(), add_field_rejects_bad_input(), add_field_runs_alter_table_with_the_right_column_type(), columns(), create_storage(), create_table(), create_table_builds_sql_table_with_primary_field() (+50 more)

### Community 4 - "Auth & API Keys"
Cohesion: 0.08
Nodes (23): Credentials, login(), logout(), me(), register(), routes(), session_cookie(), status() (+15 more)

### Community 5 - "DESIGN.md Architecture"
Cohesion: 0.07
Nodes (32): DESIGN.md (CRM Design), Auth (sessions, API keys, first-user admin), Build order, Catalog tables (tables, fields, field_options, views), Concurrency model (WAL, single writer, read pool), Field types and SQLite storage, Filter JSON tree, React SPA frontend (+24 more)

### Community 6 - "Link Tests"
Cohesion: 0.18
Nodes (20): Reply, a_table_can_relate_to_itself(), bad_filters_on_links_are_rejected(), cond(), Crm, crm(), deleting_a_link_field_hides_it_and_frees_the_name(), deleting_records_unlinks_them() (+12 more)

### Community 7 - "Search Index & Fields"
Cohesion: 0.12
Nodes (24): assert_index_matches_content(), db(), DEFAULT_LIMIT, every_table_has_an_index_and_triggers(), fts_ident(), global(), hex(), is_indexed() (+16 more)

### Community 8 - "Records Tests"
Cohesion: 0.19
Nodes (21): api_keys_can_use_records(), bad_filters_are_rejected(), bad_paging_parameters(), cond(), create_read_update_delete(), default_order_is_creation_order(), deleted_fields_and_tables_disappear_from_records(), empty_record_and_keys_by_field_id() (+13 more)

### Community 9 - "Saved Views"
Cohesion: 0.19
Nodes (23): field_by_key(), canonical_filter(), canonical_key(), COLUMNS, create(), delete(), find(), get() (+15 more)

### Community 10 - "Search Tests"
Cohesion: 0.21
Nodes (17): a_table_created_before_any_fields_is_searchable_by_name(), enc(), Env, env(), global_search_groups_matches_by_table(), global_search_skips_deleted_tables_and_validates_input(), hostile_search_text_is_harmless(), names() (+9 more)

### Community 11 - "Web Dependencies"
Cohesion: 0.10
Nodes (23): react-dom, @types/react, @types/react-dom, typescript, vite, @vitejs/plugin-react, dependencies, react@18.3.1 (+15 more)

### Community 12 - "Views Tests"
Cohesion: 0.27
Nodes (16): create_stores_ids_and_get_by_id_or_name(), delete_removes_only_that_view(), deleted_table_hides_its_views(), deleting_a_field_marks_views_stale_and_editing_fixes_them(), deleting_a_user_keeps_their_views(), deleting_a_user_who_made_a_table_works(), Env, id() (+8 more)

### Community 13 - "DB Connection Pool"
Cohesion: 0.18
Nodes (9): connect(), Db, migrate(), migrates_and_is_idempotent(), MIGRATIONS, poisoned(), READERS, run_blocking() (+1 more)

### Community 14 - "Auth Tests"
Cohesion: 0.21
Nodes (12): admin_creates_users_and_members_cannot_manage_them(), api_key_authenticates_until_deleted(), app(), bearer_token_without_key_prefix_is_rejected(), call(), first_user_registers_as_admin_and_registration_then_closes(), last_admin_cannot_be_demoted_or_deleted(), login_checks_password_and_logout_ends_session() (+4 more)

### Community 15 - "OpenAPI Docs Route"
Cohesion: 0.14
Nodes (8): asset(), PAGE, page(), routes(), ROUTES, spec(), SPEC, SwaggerAssets

### Community 16 - "Router & Health"
Cohesion: 0.19
Nodes (8): app(), Assets, health(), health_returns_ok(), router(), static_files(), unknown_api_path_is_json_404(), unknown_page_falls_back_to_index()

### Community 17 - "TypeScript Config"
Cohesion: 0.14
Nodes (13): compilerOptions, isolatedModules, jsx, lib, module, moduleResolution, noEmit, noUnusedLocals (+5 more)

### Community 18 - "Tables Tests"
Cohesion: 0.27
Nodes (7): register_admin(), api_keys_can_manage_schema(), bad_bodies_get_json_errors(), field_errors(), field_lifecycle(), find(), table_lifecycle()

## Ambiguous Edges - Review These
- `DESIGN.md (CRM Design)` → `session.txt (claude resume command)`  [AMBIGUOUS]
  session.txt · relation: conceptually_related_to

## Knowledge Gaps
- **35 isolated node(s):** `crm`, `Text`, `Number`, `Boolean`, `Date` (+30 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 152 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **4 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppError` connect `Records & Link Queries` to `API Entrypoint & Routing`, `Catalog Schema Management`, `Auth & API Keys`, `Search Index & Fields`, `Saved Views`, `DB Connection Pool`, `Router & Health`, `Error Responses`?**
  _High betweenness centrality (0.209) - this node is a cross-community bridge._
- **Are the 5 inferred relationships involving `create_table()` (e.g. with `new_id()` and `ident()`) actually correct?**
  _`create_table()` has 5 INFERRED edges - model-reasoned connections that need verification._
- **What connects `crm`, `Text`, `Number` to the rest of the system?**
  _35 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `API Entrypoint & Routing` be split into smaller, more focused modules?**
  _Cohesion score 0.05402650356778797 - nodes in this community are weakly interconnected._
- **Why does `AppState` connect `API Entrypoint & Routing` to `Auth & API Keys`, `DB Connection Pool`, `Auth Tests`, `OpenAPI Docs Route`, `Router & Health`?**
  _High betweenness centrality (0.080) - this node is a cross-community bridge._
- **Should `Web Frontend API Client` be split into smaller, more focused modules?**
  _Cohesion score 0.06303092082908597 - nodes in this community are weakly interconnected._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `DESIGN.md (CRM Design)` and `session.txt (claude resume command)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._