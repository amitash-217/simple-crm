# Graph Report - crm-p  (2026-11-20)

## Corpus Check
- 59 files · ~53,996 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 5 file(s) not represented in the graph (top: (none) 4, .css 1)

## Summary
- 880 nodes · 2682 edges · 23 communities (20 shown, 3 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 100 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Relation & Field Queries
- Server Bootstrap & Limits
- Catalog Field Management
- Auth & Password Hashing
- App State & Middleware
- Views API
- React App Shell
- Error Handling & Responses
- DESIGN.md Concepts
- Link Field Tests
- Auth Tests
- Records Tests
- Filter Editor UI
- Search Tests
- Grid & Field Dialog UI
- Frontend Dependencies
- Views Tests
- Field Inputs & Pickers
- API Client & Record Inputs
- TypeScript Config
- Server Config
- Community 21
- Community 22

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
- `Backups (sqlite3 .backup)` --conceptually_related_to--> `Concurrency model (WAL, single writer, read pool)`  [INFERRED]
  docs/LINUX.md → DESIGN.md
- `session.txt (claude resume command)` --conceptually_related_to--> `CRM Design (DESIGN.md)`  [AMBIGUOUS]
  session.txt → DESIGN.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Release and deployment pipeline** — scripts_release, readme_docker_build, docs_linux_cross_compile, docs_linux_native_build, design_release_build [INFERRED 0.85]
- **Runtime schema via catalog, internal IDs and soft delete** — design_catalog_tables, design_internal_id_naming, design_schema_changes [EXTRACTED 0.95]
- **Filter, sort and cursor shared by records endpoint and saved views** — design_filter_tree, design_sort_pagination, design_saved_views, design_rest_api [EXTRACTED 0.95]

## Communities (23 total, 3 thin omitted)

### Community 0 - "Relation & Field Queries"
Cohesion: 0.05
Nodes (68): FieldOut, ident(), ident_accepts_only_generated_names(), after_cursor(), compile_filter(), compile_sort(), Compiled, ID_EXPR (+60 more)

### Community 1 - "Server Bootstrap & Limits"
Cohesion: 0.06
Nodes (47): PLACEHOLDER, create(), CreateKey, KeyOut, list(), remove(), routes(), poll() (+39 more)

### Community 2 - "Catalog Field Management"
Cohesion: 0.09
Nodes (67): new_id(), add_field(), add_field_rejects_bad_input(), add_field_runs_alter_table_with_the_right_column_type(), change_type(), CleanOption, columns(), convert() (+59 more)

### Community 3 - "Auth & Password Hashing"
Cohesion: 0.06
Nodes (35): Credentials, login(), logout(), me(), register(), routes(), session_cookie(), status() (+27 more)

### Community 4 - "App State & Middleware"
Cohesion: 0.05
Nodes (27): asset(), PAGE, page(), routes(), ROUTES, spec(), SPEC, SwaggerAssets (+19 more)

### Community 5 - "Views API"
Cohesion: 0.10
Nodes (40): create(), empty_config(), list(), NewViewBody, remove(), routes(), show(), update() (+32 more)

### Community 6 - "React App Shell"
Cohesion: 0.11
Nodes (36): react, api, App(), currentTheme(), NewTable(), Shell(), ThemeToggle(), useChanges() (+28 more)

### Community 7 - "Error Handling & Responses"
Cohesion: 0.09
Nodes (24): assert_index_matches_content(), db(), DEFAULT_LIMIT, every_table_has_an_index_and_triggers(), fts_ident(), global(), hex(), is_indexed() (+16 more)

### Community 8 - "DESIGN.md Concepts"
Cohesion: 0.07
Nodes (36): CRM Design (DESIGN.md), Auth (sessions, API keys, first-user admin), Catalog tables (tables, fields, field_options, views), Concurrency model (WAL, single writer, read pool), Dark-only UI style, Field types, JSON filter tree, React SPA frontend (+28 more)

### Community 9 - "Link Field Tests"
Cohesion: 0.18
Nodes (24): Reply, a_field_can_change_type_when_its_values_convert(), a_table_can_relate_to_itself(), a_to_many_relation_adds_a_field_on_the_other_table_that_stays_in_sync(), bad_filters_on_links_are_rejected(), cond(), Crm, crm() (+16 more)

### Community 10 - "Auth Tests"
Cohesion: 0.15
Nodes (19): admin_creates_users_and_members_cannot_manage_them(), api_key_authenticates_until_deleted(), app(), bearer_token_without_key_prefix_is_rejected(), call(), first_user_registers_as_admin_and_registration_then_closes(), last_admin_cannot_be_demoted_or_deleted(), login_checks_password_and_logout_ends_session() (+11 more)

### Community 11 - "Records Tests"
Cohesion: 0.18
Nodes (22): api_keys_can_use_records(), bad_filters_are_rejected(), bad_paging_parameters(), cond(), create_read_update_delete(), default_order_is_creation_order(), deleted_fields_and_tables_disappear_from_records(), empty_record_and_keys_by_field_id() (+14 more)

### Community 12 - "Filter Editor UI"
Cohesion: 0.12
Nodes (28): fromLocalInput(), pad2(), toLocalInput(), complete(), Cond, FilterEditor(), Mode, NO_VALUE (+20 more)

### Community 13 - "Search Tests"
Cohesion: 0.21
Nodes (17): a_table_created_before_any_fields_is_searchable_by_name(), enc(), Env, env(), global_search_groups_matches_by_table(), global_search_skips_deleted_tables_and_validates_input(), hostile_search_text_is_harmless(), names() (+9 more)

### Community 14 - "Grid & Field Dialog UI"
Cohesion: 0.24
Nodes (25): errorMessage(), FieldDialog(), SortEditor(), Grid(), HeaderMenu(), Icon(), typeIcon(), Kanban() (+17 more)

### Community 15 - "Frontend Dependencies"
Cohesion: 0.10
Nodes (23): react-dom, @types/react, @types/react-dom, typescript, vite, @vitejs/plugin-react, dependencies, react@18.3.1 (+15 more)

### Community 16 - "Views Tests"
Cohesion: 0.27
Nodes (16): create_stores_ids_and_get_by_id_or_name(), delete_removes_only_that_view(), deleted_table_hides_its_views(), deleting_a_field_marks_views_stale_and_editing_fixes_them(), deleting_a_user_keeps_their_views(), deleting_a_user_who_made_a_table_works(), Env, id() (+8 more)

### Community 17 - "Field Inputs & Pickers"
Cohesion: 0.14
Nodes (21): CONVERTIBLE, OptionRow, TYPES, InputProps, PickRow, TONE_NAMES, CellProps, GridProps (+13 more)

### Community 18 - "API Client & Record Inputs"
Cohesion: 0.20
Nodes (17): ApiError, ListArgs, listRecords(), request(), FieldInput(), optionTone(), RecordChipLabel(), RecordPicker() (+9 more)

### Community 19 - "TypeScript Config"
Cohesion: 0.14
Nodes (13): compilerOptions, isolatedModules, jsx, lib, module, moduleResolution, noEmit, noUnusedLocals (+5 more)

## Ambiguous Edges - Review These
- `session.txt (claude resume command)` → `CRM Design (DESIGN.md)`  [AMBIGUOUS]
  session.txt · relation: conceptually_related_to
- `Dark-only UI style` → `Theme bootstrap script (localStorage theme)`  [AMBIGUOUS]
  web/index.html · relation: conceptually_related_to

## Knowledge Gaps
- **39 isolated node(s):** `crm`, `Number`, `Boolean`, `Date`, `Phone` (+34 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 168 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **3 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppError` connect `Views API` to `Relation & Field Queries`, `Server Bootstrap & Limits`, `Catalog Field Management`, `Auth & Password Hashing`, `App State & Middleware`, `Error Handling & Responses`?**
  _High betweenness centrality (0.184) - this node is a cross-community bridge._
- **What connects `crm`, `Number`, `Boolean` to the rest of the system?**
  _39 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Relation & Field Queries` be split into smaller, more focused modules?**
  _Cohesion score 0.05238796549547654 - nodes in this community are weakly interconnected._
- **Why does `AppState` connect `Server Bootstrap & Limits` to `Auth Tests`, `Auth & Password Hashing`, `App State & Middleware`, `Views API`?**
  _High betweenness centrality (0.077) - this node is a cross-community bridge._
- **Should `Server Bootstrap & Limits` be split into smaller, more focused modules?**
  _Cohesion score 0.058672276764843385 - nodes in this community are weakly interconnected._

### Low-confidence Hints
_AMBIGUOUS edges — the extractor was unsure. Verify before acting on these._

- **What is the exact relationship between `session.txt (claude resume command)` and `CRM Design (DESIGN.md)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **What is the exact relationship between `Dark-only UI style` and `Theme bootstrap script (localStorage theme)`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._