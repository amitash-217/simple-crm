# Graph Report - crm-p  (2026-11-21)

## Corpus Check
- 59 files · ~54,280 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 5 file(s) not represented in the graph (top: (none) 4, .css 1)

## Summary
- 881 nodes · 2686 edges · 35 communities (31 shown, 4 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 100 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- API Routes and Handlers
- Auth and Password Hashing
- React App Shell
- Design Docs and Architecture
- Link Field Tests
- Search Index and Field Mutations
- Search API Tests
- Record Storage Core
- Record API Tests
- Filter and Sort Compiler
- Catalog Table Creation
- Filter Editor UI
- Grid, Kanban and Dialogs
- App Errors and Validation
- Frontend Dependencies
- View API Tests
- Field Types UI
- Saved Views
- Field Type Conversion
- Database Connection Pool
- Auth API Tests
- Frontend API Client
- Change Polling
- Value Normalization
- OpenAPI Docs Page
- Router and Health
- TypeScript Config
- Catalog Table Lookup
- Table API Tests
- Field Options Sync
- Link Field Metadata
- Server Entrypoint
- Config Loading
- Cargo Crate
- Favicon

## God Nodes (most connected - your core abstractions)
1. `AppError` - 136 edges
2. `AppState` - 64 edges
3. `FieldOut` - 54 edges
4. `AuthUser` - 37 edges
5. `Icon()` - 37 edges
6. `create_table()` - 30 edges
7. `register_admin()` - 28 edges
8. `ident()` - 26 edges
9. `resolve_table()` - 26 edges
10. `add_field()` - 25 edges

## Surprising Connections (you probably didn't know these)
- `Cross-compile from Mac (zigbuild, musl)` --semantically_similar_to--> `Docker musl static build`  [INFERRED] [semantically similar]
  docs/LINUX.md → README.md
- `Theme bootstrap script (localStorage theme)` --conceptually_related_to--> `Dark-only UI style`  [AMBIGUOUS]
  web/index.html → DESIGN.md
- `writes_bump_the_version_and_reads_do_not()` --calls--> `register_admin()`  [INFERRED]
  src/api/changes.rs → src/api/auth_tests.rs
- `session.txt (claude resume command)` --conceptually_related_to--> `CRM Design (DESIGN.md)`  [AMBIGUOUS]
  session.txt → DESIGN.md
- `Backups (sqlite3 .backup)` --conceptually_related_to--> `Concurrency model (WAL, single writer, read pool)`  [INFERRED]
  docs/LINUX.md → DESIGN.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Runtime schema via catalog, internal IDs and soft delete** — design_catalog_tables, design_internal_id_naming, design_schema_changes [EXTRACTED 0.95]
- **Filter, sort and cursor shared by records endpoint and saved views** — design_filter_tree, design_sort_pagination, design_saved_views, design_rest_api [EXTRACTED 0.95]
- **Release and deployment pipeline** — scripts_release, readme_docker_build, docs_linux_cross_compile, docs_linux_native_build, design_release_build [INFERRED 0.85]

## Communities (35 total, 4 thin omitted)

### Community 0 - "API Routes and Handlers"
Cohesion: 0.06
Nodes (50): PLACEHOLDER, create(), CreateKey, KeyOut, list(), remove(), routes(), ApiJson (+42 more)

### Community 1 - "Auth and Password Hashing"
Cohesion: 0.06
Nodes (34): Credentials, login(), logout(), me(), register(), routes(), session_cookie(), status() (+26 more)

### Community 2 - "React App Shell"
Cohesion: 0.11
Nodes (36): react, api, App(), currentTheme(), NewTable(), Shell(), ThemeToggle(), useChanges() (+28 more)

### Community 3 - "Design Docs and Architecture"
Cohesion: 0.07
Nodes (36): CRM Design (DESIGN.md), Auth (sessions, API keys, first-user admin), Catalog tables (tables, fields, field_options, views), Concurrency model (WAL, single writer, read pool), Dark-only UI style, Field types, JSON filter tree, React SPA frontend (+28 more)

### Community 4 - "Link Field Tests"
Cohesion: 0.18
Nodes (24): Reply, a_field_can_change_type_when_its_values_convert(), a_table_can_relate_to_itself(), a_to_many_relation_adds_a_field_on_the_other_table_that_stays_in_sync(), bad_filters_on_links_are_rejected(), cond(), Crm, crm() (+16 more)

### Community 5 - "Search Index and Field Mutations"
Cohesion: 0.11
Nodes (28): ident(), ident_accepts_only_generated_names(), NewField, delete(), assert_index_matches_content(), db(), DEFAULT_LIMIT, every_table_has_an_index_and_triggers() (+20 more)

### Community 6 - "Search API Tests"
Cohesion: 0.15
Nodes (17): a_table_created_before_any_fields_is_searchable_by_name(), enc(), Env, env(), global_search_groups_matches_by_table(), global_search_skips_deleted_tables_and_validates_input(), hostile_search_text_is_harmless(), names() (+9 more)

### Community 7 - "Record Storage Core"
Cohesion: 0.15
Nodes (25): assemble(), assign(), Cell, Column, Links, check_targets(), create(), decode_cursor() (+17 more)

### Community 8 - "Record API Tests"
Cohesion: 0.18
Nodes (22): api_keys_can_use_records(), bad_filters_are_rejected(), bad_paging_parameters(), cond(), create_read_update_delete(), default_order_is_creation_order(), deleted_fields_and_tables_disappear_from_records(), empty_record_and_keys_by_field_id() (+14 more)

### Community 9 - "Filter and Sort Compiler"
Cohesion: 0.12
Nodes (24): after_cursor(), compile_filter(), compile_sort(), Compiled, ID_EXPR, Kind, Bool, Date (+16 more)

### Community 10 - "Catalog Table Creation"
Cohesion: 0.21
Nodes (28): new_id(), add_field(), add_field_rejects_bad_input(), add_field_runs_alter_table_with_the_right_column_type(), columns(), create_table(), create_table_builds_sql_table_with_primary_field(), db() (+20 more)

### Community 11 - "Filter Editor UI"
Cohesion: 0.12
Nodes (28): fromLocalInput(), pad2(), toLocalInput(), complete(), Cond, FilterEditor(), Mode, NO_VALUE (+20 more)

### Community 12 - "Grid, Kanban and Dialogs"
Cohesion: 0.24
Nodes (25): errorMessage(), FieldDialog(), SortEditor(), Grid(), HeaderMenu(), Icon(), typeIcon(), Kanban() (+17 more)

### Community 13 - "App Errors and Validation"
Cohesion: 0.18
Nodes (20): resolve_table(), validate_name(), AppError, BadRequest, Conflict, Db, Forbidden, Internal (+12 more)

### Community 14 - "Frontend Dependencies"
Cohesion: 0.10
Nodes (23): react-dom, @types/react, @types/react-dom, typescript, vite, @vitejs/plugin-react, dependencies, react@18.3.1 (+15 more)

### Community 15 - "View API Tests"
Cohesion: 0.27
Nodes (16): create_stores_ids_and_get_by_id_or_name(), delete_removes_only_that_view(), deleted_table_hides_its_views(), deleting_a_field_marks_views_stale_and_editing_fixes_them(), deleting_a_user_keeps_their_views(), deleting_a_user_who_made_a_table_works(), Env, id() (+8 more)

### Community 16 - "Field Types UI"
Cohesion: 0.14
Nodes (21): CONVERTIBLE, OptionRow, TYPES, InputProps, PickRow, TONE_NAMES, CellProps, GridProps (+13 more)

### Community 17 - "Saved Views"
Cohesion: 0.15
Nodes (15): NOW, field_by_key(), validate_filter(), validate_sort(), canonical_filter(), canonical_key(), COLUMNS, Kind (+7 more)

### Community 18 - "Field Type Conversion"
Cohesion: 0.14
Nodes (17): change_type(), convert(), create_storage(), FieldConfig, FieldType, Boolean, Date, Email (+9 more)

### Community 19 - "Database Connection Pool"
Cohesion: 0.19
Nodes (10): backfill_checkboxes(), connect(), Db, migrate(), migrates_and_is_idempotent(), MIGRATIONS, poisoned(), READERS (+2 more)

### Community 20 - "Auth API Tests"
Cohesion: 0.21
Nodes (12): admin_creates_users_and_members_cannot_manage_them(), api_key_authenticates_until_deleted(), app(), bearer_token_without_key_prefix_is_rejected(), call(), first_user_registers_as_admin_and_registration_then_closes(), last_admin_cannot_be_demoted_or_deleted(), login_checks_password_and_logout_ends_session() (+4 more)

### Community 21 - "Frontend API Client"
Cohesion: 0.20
Nodes (17): ApiError, ListArgs, listRecords(), request(), FieldInput(), optionTone(), RecordChipLabel(), RecordPicker() (+9 more)

### Community 22 - "Change Polling"
Cohesion: 0.16
Nodes (7): poll(), PollQuery, routes(), track(), WAIT, writes_bump_the_version_and_reads_do_not(), ApiQuery

### Community 23 - "Value Normalization"
Cohesion: 0.19
Nodes (13): dates_and_datetimes(), emails(), from_sql(), MAX_LINKS, MAX_LONG_TEXT, MAX_TEXT, normalize_phone(), option_id() (+5 more)

### Community 24 - "OpenAPI Docs Page"
Cohesion: 0.14
Nodes (7): asset(), PAGE, page(), ROUTES, spec(), SPEC, SwaggerAssets

### Community 25 - "Router and Health"
Cohesion: 0.20
Nodes (7): app(), Assets, health(), health_returns_ok(), static_files(), unknown_api_path_is_json_404(), unknown_page_falls_back_to_index()

### Community 26 - "TypeScript Config"
Cohesion: 0.14
Nodes (13): compilerOptions, isolatedModules, jsx, lib, module, moduleResolution, noEmit, noUnusedLocals (+5 more)

### Community 27 - "Catalog Table Lookup"
Cohesion: 0.23
Nodes (11): delete_field(), get_table(), list_tables(), relation_target(), resolve_field(), Table, table_from_row(), table_name_taken() (+3 more)

### Community 28 - "Table API Tests"
Cohesion: 0.36
Nodes (7): register_admin(), api_keys_can_manage_schema(), bad_bodies_get_json_errors(), field_errors(), field_lifecycle(), find(), table_lifecycle()

### Community 29 - "Field Options Sync"
Cohesion: 0.32
Nodes (8): CleanOption, field_name_taken(), FieldPatch, OptionInput, OptionOut, sync_options(), update_field(), validate_options()

### Community 31 - "Server Entrypoint"
Cohesion: 0.33
Nodes (4): routes(), router(), main(), shutdown_signal()

## Ambiguous Edges - Review These
- `CRM Design (DESIGN.md)` → `session.txt (claude resume command)`  [AMBIGUOUS]
  session.txt · relation: conceptually_related_to
- `Dark-only UI style` → `Theme bootstrap script (localStorage theme)`  [AMBIGUOUS]
  web/index.html · relation: conceptually_related_to

## Knowledge Gaps
- **39 isolated node(s):** `crm`, `Number`, `Boolean`, `Date`, `Phone` (+34 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 168 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **4 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppError` connect `App Errors and Validation` to `API Routes and Handlers`, `Auth and Password Hashing`, `Search Index and Field Mutations`, `Search API Tests`, `Record Storage Core`, `Filter and Sort Compiler`, `Catalog Table Creation`, `Saved Views`, `Field Type Conversion`, `Database Connection Pool`, `Value Normalization`, `Router and Health`, `Catalog Table Lookup`, `Field Options Sync`?**
  _High betweenness centrality (0.184) - this node is a cross-community bridge._
- **What connects `crm`, `Number`, `Boolean` to the rest of the system?**
  _39 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `API Routes and Handlers` be split into smaller, more focused modules?**
  _Cohesion score 0.06426182513139035 - nodes in this community are weakly interconnected._
- **Why does `AppState` connect `API Routes and Handlers` to `Auth and Password Hashing`, `Database Connection Pool`, `Auth API Tests`, `Change Polling`, `OpenAPI Docs Page`, `Router and Health`, `Server Entrypoint`?**
  _High betweenness centrality (0.077) - this node is a cross-community bridge._
- **Should `Auth and Password Hashing` be split into smaller, more focused modules?**
  _Cohesion score 0.06293706293706294 - nodes in this community are weakly interconnected._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `CRM Design (DESIGN.md)` and `session.txt (claude resume command)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Dark-only UI style` and `Theme bootstrap script (localStorage theme)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._