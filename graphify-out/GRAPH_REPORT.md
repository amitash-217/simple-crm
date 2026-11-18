# Graph Report - crm-p  (2026-11-18)

## Corpus Check
- 57 files · ~50,880 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 5 file(s) not represented in the graph (top: (none) 4, .css 1)

## Summary
- 841 nodes · 2558 edges · 27 communities (24 shown, 3 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 98 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- React Frontend
- Catalog and Fields
- Server Plumbing
- Auth and Sessions
- Records SQL
- Relation and Link Tests
- Design Doc
- Filter Compiler
- Auth Tests
- Views Tests
- Records Tests
- Search Tests
- Frontend Dependencies
- Views Module
- Routing and State
- Tables API
- App Bootstrap
- Users API
- Records API
- Value Parsing
- Relation Fields
- TS Config
- Search API
- Error Handling
- Config
- Main and Shutdown
- Crate Root

## God Nodes (most connected - your core abstractions)
1. `AppError` - 134 edges
2. `AppState` - 58 edges
3. `FieldOut` - 54 edges
4. `Icon()` - 36 edges
5. `AuthUser` - 34 edges
6. `create_table()` - 29 edges
7. `register_admin()` - 26 edges
8. `ident()` - 26 edges
9. `resolve_table()` - 26 edges
10. `add_field()` - 25 edges

## Surprising Connections (you probably didn't know these)
- `Cross-compile from Mac (zigbuild, musl)` --semantically_similar_to--> `Docker musl static build`  [INFERRED] [semantically similar]
  docs/LINUX.md → README.md
- `session.txt (claude resume command)` --conceptually_related_to--> `DESIGN.md (CRM Design)`  [AMBIGUOUS]
  session.txt → DESIGN.md
- `Backups (sqlite3 .backup)` --conceptually_related_to--> `Concurrency model (WAL, single writer, read pool)`  [INFERRED]
  docs/LINUX.md → DESIGN.md
- `app()` --calls--> `router()`  [INFERRED]
  src/api/auth_tests.rs → src/api/mod.rs
- `crm()` --calls--> `register_admin()`  [INFERRED]
  src/api/links_tests.rs → src/api/auth_tests.rs

## Import Cycles
- 1-file cycle: `src/api/search.rs -> src/api/search.rs`

## Hyperedges (group relationships)
- **Runtime-defined schema storage** — design_catalog_tables, design_internal_id_naming, design_schema_changes, design_field_types, design_relations [INFERRED 0.85]
- **Release and deployment pipeline** — scripts_release, readme_docker_build, docs_linux_cross_compile, docs_linux_native_build, design_release_build [INFERRED 0.85]

## Communities (27 total, 3 thin omitted)

### Community 0 - "React Frontend"
Cohesion: 0.05
Nodes (117): react, api, ApiError, errorMessage(), ListArgs, listRecords(), request(), App() (+109 more)

### Community 1 - "Catalog and Fields"
Cohesion: 0.06
Nodes (88): new_id(), add_field(), add_field_rejects_bad_input(), add_field_runs_alter_table_with_the_right_column_type(), change_type(), CleanOption, columns(), convert() (+80 more)

### Community 2 - "Server Plumbing"
Cohesion: 0.06
Nodes (24): asset(), PAGE, page(), routes(), ROUTES, spec(), SPEC, SwaggerAssets (+16 more)

### Community 3 - "Auth and Sessions"
Cohesion: 0.09
Nodes (21): Credentials, login(), logout(), me(), register(), session_cookie(), status(), with_session() (+13 more)

### Community 4 - "Records SQL"
Cohesion: 0.14
Nodes (29): ident(), ident_accepts_only_generated_names(), assemble(), assign(), Cell, Column, Links, check_targets() (+21 more)

### Community 5 - "Relation and Link Tests"
Cohesion: 0.18
Nodes (24): Reply, a_field_can_change_type_when_its_values_convert(), a_table_can_relate_to_itself(), a_to_many_relation_adds_a_field_on_the_other_table_that_stays_in_sync(), bad_filters_on_links_are_rejected(), cond(), Crm, crm() (+16 more)

### Community 6 - "Design Doc"
Cohesion: 0.07
Nodes (31): DESIGN.md (CRM Design), Auth (sessions, API keys, first-user admin), Build order, Catalog tables (tables, fields, field_options, views), Concurrency model (WAL, single writer, read pool), Field types and SQLite storage, Filter JSON tree, React SPA frontend (+23 more)

### Community 7 - "Filter Compiler"
Cohesion: 0.12
Nodes (25): after_cursor(), compile_filter(), compile_sort(), Compiled, ID_EXPR, Kind, Bool, Date (+17 more)

### Community 8 - "Auth Tests"
Cohesion: 0.15
Nodes (19): admin_creates_users_and_members_cannot_manage_them(), api_key_authenticates_until_deleted(), app(), bearer_token_without_key_prefix_is_rejected(), call(), first_user_registers_as_admin_and_registration_then_closes(), last_admin_cannot_be_demoted_or_deleted(), login_checks_password_and_logout_ends_session() (+11 more)

### Community 9 - "Views Tests"
Cohesion: 0.18
Nodes (16): create_stores_ids_and_get_by_id_or_name(), delete_removes_only_that_view(), deleted_table_hides_its_views(), deleting_a_field_marks_views_stale_and_editing_fixes_them(), deleting_a_user_keeps_their_views(), deleting_a_user_who_made_a_table_works(), Env, id() (+8 more)

### Community 10 - "Records Tests"
Cohesion: 0.19
Nodes (21): api_keys_can_use_records(), bad_filters_are_rejected(), bad_paging_parameters(), cond(), create_read_update_delete(), default_order_is_creation_order(), deleted_fields_and_tables_disappear_from_records(), empty_record_and_keys_by_field_id() (+13 more)

### Community 11 - "Search Tests"
Cohesion: 0.21
Nodes (17): a_table_created_before_any_fields_is_searchable_by_name(), enc(), Env, env(), global_search_groups_matches_by_table(), global_search_skips_deleted_tables_and_validates_input(), hostile_search_text_is_harmless(), names() (+9 more)

### Community 12 - "Frontend Dependencies"
Cohesion: 0.10
Nodes (23): react-dom, @types/react, @types/react-dom, typescript, vite, @vitejs/plugin-react, dependencies, react@18.3.1 (+15 more)

### Community 13 - "Views Module"
Cohesion: 0.21
Nodes (17): NOW, COLUMNS, create(), delete(), find(), get(), list(), MAX_HIDDEN (+9 more)

### Community 14 - "Routing and State"
Cohesion: 0.20
Nodes (14): routes(), routes(), AppState, router(), routes(), create(), empty_config(), list() (+6 more)

### Community 15 - "Tables API"
Cohesion: 0.30
Nodes (13): add_field(), create(), delete_field(), FieldPatchBody, list(), list_fields(), NewFieldBody, remove() (+5 more)

### Community 16 - "App Bootstrap"
Cohesion: 0.16
Nodes (7): PLACEHOLDER, create(), CreateKey, KeyOut, list(), remove(), API_KEY_PREFIX

### Community 17 - "Users API"
Cohesion: 0.23
Nodes (11): create(), CreateUser, ensure_other_admin(), list(), remove(), routes(), update(), UpdateUser (+3 more)

### Community 18 - "Records API"
Cohesion: 0.25
Nodes (11): ApiJson, ApiQuery, create(), list(), ListQuery, parse_json(), RecordBody, remove() (+3 more)

### Community 19 - "Value Parsing"
Cohesion: 0.19
Nodes (13): dates_and_datetimes(), emails(), from_sql(), MAX_LINKS, MAX_LONG_TEXT, MAX_TEXT, normalize_phone(), option_id() (+5 more)

### Community 20 - "Relation Fields"
Cohesion: 0.22
Nodes (9): FieldOut, field_by_key(), validate_filter(), canonical_filter(), canonical_key(), Kind, Grid, Kanban (+1 more)

### Community 21 - "TS Config"
Cohesion: 0.14
Nodes (13): compilerOptions, isolatedModules, jsx, lib, module, moduleResolution, noEmit, noUnusedLocals (+5 more)

### Community 22 - "Search API"
Cohesion: 0.27
Nodes (3): routes(), search(), SearchQuery

### Community 23 - "Error Handling"
Cohesion: 0.24
Nodes (8): AppError, BadRequest, Conflict, Db, Forbidden, Internal, NotFound, Unauthorized

## Ambiguous Edges - Review These
- `DESIGN.md (CRM Design)` → `session.txt (claude resume command)`  [AMBIGUOUS]
  session.txt · relation: conceptually_related_to

## Knowledge Gaps
- **36 isolated node(s):** `crm`, `Number`, `Boolean`, `Date`, `Phone` (+31 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 163 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **3 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppError` connect `Error Handling` to `Catalog and Fields`, `Server Plumbing`, `Auth and Sessions`, `Records SQL`, `Filter Compiler`, `Views Tests`, `Views Module`, `Routing and State`, `Tables API`, `App Bootstrap`, `Users API`, `Records API`, `Value Parsing`, `Relation Fields`, `Search API`?**
  _High betweenness centrality (0.197) - this node is a cross-community bridge._
- **What connects `crm`, `Number`, `Boolean` to the rest of the system?**
  _36 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `React Frontend` be split into smaller, more focused modules?**
  _Cohesion score 0.050217864923747274 - nodes in this community are weakly interconnected._
- **Why does `AppState` connect `Routing and State` to `Server Plumbing`, `Auth and Sessions`, `Auth Tests`, `Tables API`, `App Bootstrap`, `Users API`, `Records API`, `Search API`?**
  _High betweenness centrality (0.073) - this node is a cross-community bridge._
- **Should `Catalog and Fields` be split into smaller, more focused modules?**
  _Cohesion score 0.05520941502249913 - nodes in this community are weakly interconnected._
- **Why does `FieldOut` connect `Relation Fields` to `Catalog and Fields`, `Records SQL`, `Filter Compiler`, `Views Module`, `Tables API`, `Value Parsing`?**
  _High betweenness centrality (0.041) - this node is a cross-community bridge._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `DESIGN.md (CRM Design)` and `session.txt (claude resume command)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._