# CRM Design

A no-code database for a small team. Users create their own tables, fields and views at runtime. There are no built-in CRM objects such as People or Companies. The product ships as a single static Rust binary with an embedded React frontend and one SQLite file.

## Goals and non-goals

Goals:

- Users define tables, fields and views without developer help.
- Each user table is a real SQLite table, so queries are fast and indexable.
- A generic REST API creates, reads, updates and deletes records in any user table.
- One binary and one data file. Deployment is copy and run.

Non-goals (v1):

- Per-object or per-field permissions. Everyone in the workspace sees everything.
- Multiple workspaces. One workspace maps to one database file.
- Email or calendar sync, webhooks, workflow automation.
- Computed fields (formula, rollup, lookup).
- Calendar, gallery and chart views.
- Per-field change history.

## Stack

| Layer | Choice |
|---|---|
| Language | Rust |
| HTTP | Axum |
| SQLite driver | rusqlite (bundled SQLite, FTS5 enabled) |
| Frontend | React SPA, built to `web/dist`, embedded with `rust-embed` |
| Auth | Email and password (argon2, cookie sessions) for the UI; API keys for scripts |
| IDs | UUIDv7 |

## Storage model

### Two kinds of tables

1. System tables, created by migrations: `users`, `sessions`, `api_keys`, and the catalog tables below.
2. User tables, created at runtime. The catalog describes them.

### Naming

User-visible names never appear in SQL. Each user table and field gets an internal ID, and the SQL identifiers derive from it:

- Table `t_<id>`, for example `t_01j8x...`.
- Column `f_<id>`.

Renaming a table or field changes only the catalog row. No DDL runs on rename. All identifiers that reach SQL come from this generated scheme, never from user input, which removes the main SQL injection surface of dynamic DDL.

### Catalog tables

- `tables(id, name, sql_name, created_at, created_by, deleted_at)`
- `fields(id, table_id, name, sql_name, type, config_json, position, is_primary, created_at, deleted_at)`
- `field_options(id, field_id, label, color, position)` for select and multi-select
- `views(id, table_id, name, kind, config_json, position, created_at, created_by)`

`views.kind` is `grid` or `kanban` and cannot change after creation. `config_json` holds these keys, all optional except `group_by` on Kanban:

- `filter`: the same JSON tree the records endpoint takes.
- `sort`: the same `[{field, dir}]` array the records endpoint takes.
- `hidden_fields`: field IDs the grid hides. The primary field cannot be hidden.
- `group_by`: the select field whose options make the Kanban columns. Kanban only.

Requests may name fields and select options by name or label. The server validates the config against the table's current fields, then stores field IDs and option IDs, so renaming a field or option never breaks a view. System fields such as `$created_at` stay as they are. A view returns `stale: true` when a field or option it uses has since been deleted. The stored config is kept unchanged until someone patches it. A client can send a view's `filter` and `sort` straight to the records endpoint, because that endpoint accepts IDs.

Views are plain configuration, so deleting one removes its row. A view name is unique within a table, ignoring case. A table holds at most 100 views. Deleting a user clears `created_by` on the tables and views they made.

### Record columns

Every user table has these fixed columns in addition to its user fields:

- `id TEXT NOT NULL UNIQUE` (UUIDv7), the ID the API uses and foreign keys reference
- `created_at`, `updated_at` (ISO 8601 UTC)
- `created_by`, `updated_by` (user ID)
- `seq INTEGER PRIMARY KEY`, an internal row number that is never exposed. The full-text index points at it. SQLite may renumber an implicit rowid during `VACUUM`, which would silently desync the index, so the row number is explicit.

### Field types

| Type | SQLite storage |
|---|---|
| text | `TEXT` |
| number | `REAL` (or `INTEGER` when configured as integer) |
| boolean | `INTEGER` 0/1 |
| date | `TEXT`, ISO 8601 |
| phone | `TEXT`, normalized to E.164 on write |
| select | `TEXT` holding a `field_options.id` |
| multi-select | join table `m_<field_id>(record_id, option_id, position)` |
| relation (to one) | `TEXT` column holding the target record ID, `REFERENCES target(id) ON DELETE SET NULL` |
| relation (to many) | join table `r_<field_id>(record_id, target_id, position)` |

Relations are one-directional: a relation field on table A points to table B, and B gets no inverse field. A relation field's `config` is `{"table": "<target table id>", "many": false}`. A request may name the target table by name or ID, and the catalog stores the ID, so renaming the target breaks nothing. A table may relate to itself.

Join tables have `ON DELETE CASCADE` foreign keys on both sides, and `multi-select` ones reference `field_options`. So deleting a record, or removing a select option, removes the matching links without extra code. Soft-deleting a field leaves its join table and data in place, like any other field. Soft-deleting a target table keeps existing links readable, and new links to it are refused.

### Schema changes

- Create table: insert catalog rows, then run `CREATE TABLE`, in one transaction.
- Add field: insert catalog row, then `ALTER TABLE ... ADD COLUMN` (or create the join table), in one transaction.
- Rename: catalog update only.
- Delete table or field: soft delete. Set `deleted_at` in the catalog. The SQL object and its data stay, and the API stops exposing them. An explicit purge endpoint later runs `DROP`.
- Changing a field's type is not supported in v1. The user adds a new field and migrates data.
- Every table starts with a primary text field named `Name`. It cannot be deleted.
- Names are unique per scope (tables, and fields within a table), compared case-insensitively among live objects. A soft-deleted name can be reused.
- Select options are replaced as a whole through `PATCH` on the field. Entries with an `id` are kept and edited, entries without one are added, and omitted options are removed. Records that held a removed option get that field set to NULL.
- Field types: text, number, boolean, date, phone, select, multi_select and relation. Removing a multi-select option works like removing a select option, and the links to it are removed.

SQLite runs DDL inside transactions, so a failed catalog or DDL step rolls back both.

### Full-text search

Each user table gets an FTS5 external-content table `s_<hex>` over its live text and phone fields, kept current by insert, update and delete triggers. The tokenizer is `unicode61` with accent removal, so `cafe` finds `Café` and matching ignores case. Number, date, boolean, select and relation fields are not indexed.

Adding or soft-deleting a text or phone field drops and rebuilds that table's index and triggers, in the same transaction as the schema change. That keeps the text of deleted fields out of search results, at the cost of one pass over the table.

Search text from a request never reaches `MATCH` as written. It is split into words, each word is quoted, all words are required, and the last word matches as a prefix. FTS5 operators (`OR`, `NEAR`, `-`, `*`, `col:`) in the input are therefore ordinary words or ignored, and text with no letters or digits is a 400. Up to 10 words of up to 100 characters are accepted. Text made only of digits and phone punctuation, like `+1 (415) 555-2671`, also tries the digits joined together, because a phone number is indexed as one run of digits.

Limits: matching is by whole words and word prefixes, never the middle of a word. A phone number is found by its digits from the country code on, so `415 555 2671` without the `+1` does not match.

`q` on the records list narrows the list like a filter, and the order still follows `sort`. `GET /api/search?q=...&limit=` searches every live table, and returns `{"results": [{"table": {"id", "name"}, "matches": [{"id", "title", "snippet"}]}]}` with tables in creation order and matches best first. `title` is the record's primary field. `snippet` is plain text with the matched words wrapped in `«` and `»`. `limit` is matches per table (1 to 50, default 5), and tables without matches are left out.

### Concurrency

- WAL mode, `synchronous = NORMAL`, `foreign_keys = ON`, `busy_timeout` set.
- One dedicated writer connection, behind a mutex or a channel. Writes and DDL are serialized.
- A pool of read connections for queries.
- Axum handlers run SQLite work on `spawn_blocking`.

## REST API

All routes are under `/api`. Auth is a session cookie or an `Authorization: Bearer <api key>` header.

Auth:

- `POST /api/auth/register` works only while no user exists. The first user becomes admin. After that it returns 403.
- `POST /api/auth/login`, `POST /api/auth/logout`
- `GET/POST /api/users`, `PATCH/DELETE /api/users/{id}`: admin only. The admin creates accounts with an email and a temporary password. There is no email sending, so no SMTP dependency.
- `GET/POST/DELETE /api/api-keys`

Schema management:

- `GET/POST /api/tables`
- `GET/PATCH/DELETE /api/tables/{table}` (`GET` returns the table with its fields and select options)
- `GET/POST /api/tables/{table}/fields`
- `PATCH/DELETE /api/tables/{table}/fields/{field}`
- `GET/POST /api/tables/{table}/views`
- `GET/PATCH/DELETE /api/tables/{table}/views/{view}` (`{view}` is an ID or a name; `PATCH` takes `name`, `config` and `position`, and `config` replaces the old one)

Records:

- `GET /api/tables/{table}/records` with `q` (full-text), `filter`, `sort`, `limit`, `cursor` query parameters
- `GET /api/search`, full-text search across all tables
- `POST /api/tables/{table}/records`
- `GET/PATCH/DELETE /api/tables/{table}/records/{id}`

A record looks like `{"id", "created_at", "updated_at", "created_by", "updated_by", "values": {"<field name>": <value>}}`. Writes send `{"values": {...}}`. The `values` wrapper keeps user field names from colliding with system keys. `values` keys may be field names (case-insensitive) or field IDs. `POST` sets the listed fields, and `PATCH` changes only the listed fields. `null` clears a field. Unknown keys are rejected.

Value formats:

| Type | JSON value |
|---|---|
| text | string, up to 100,000 characters |
| number | number; whole numbers only when the field is configured as integer |
| boolean | `true` or `false` |
| date | `YYYY-MM-DD`, or a date-time like `2026-03-01T09:30:00Z` (`Z` or `±HH:MM`) |
| phone | international number; spaces, dashes, dots and parentheses are stripped, stored as E.164 (`+14155552671`) |
| select | option label or ID on write (labels match case-insensitively); the label on read |
| multi_select | array of option labels or IDs on write (no duplicates); array of labels on read, in link order |
| relation (to one) | record ID string, or `null`; the target record must exist |
| relation (to many) | array of record ID strings (no duplicates, at most 1000); every record must exist |

`null` on a multi-select or to-many field means none. Those fields always read back as an array, empty when nothing is linked.

`DELETE` removes the row. Record tables have no soft-delete column.

The `filter` parameter is a URL-encoded JSON tree. Saved views store the same shape in `views.config_json`, so one parser serves both.

```json
{"and": [
  {"field": "Stage", "op": "eq", "value": "Won"},
  {"or": [
    {"field": "Amount", "op": "gt", "value": 100},
    {"field": "Owner", "op": "is_empty"}
  ]}
]}
```

Groups are `and` and `or`. An empty `and` matches everything and an empty `or` matches nothing. Nesting is limited to 8 levels and 50 conditions per filter. Operators: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `contains`, `starts_with`, `is_empty`, `is_not_empty`, `in`. Which operators apply depends on the field type:

| Type | Operators |
|---|---|
| text, phone | `eq`, `neq`, `contains`, `starts_with`, `in`, `is_empty`, `is_not_empty` |
| number, date | `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `in`, `is_empty`, `is_not_empty` |
| boolean | `eq`, `neq`, `is_empty`, `is_not_empty` |
| select | `eq`, `neq`, `in`, `is_empty`, `is_not_empty` (option label or ID) |
| relation (to one) | `eq`, `neq`, `in`, `is_empty`, `is_not_empty` (record ID) |
| multi_select, relation (to many) | `has` (one option label/ID or record ID), `has_any` (array of them), `is_empty`, `is_not_empty` |

`neq` also matches records where the field is empty. `contains` and `starts_with` treat `%` and `_` as plain characters, and match case-insensitively for ASCII letters. Dates compare as strings, so use one format per field. The server maps field names to columns through the catalog, and binds every value as a SQL parameter.

`field` may also be a system column: `$id`, `$created_at`, `$updated_at`, `$created_by`, `$updated_by`.

Relation and multi-select fields cannot be used in `sort`.

`sort` is a JSON array, for example `[{"field": "Amount", "dir": "desc"}, {"field": "Name"}]`, with at most 5 keys. `dir` defaults to `asc`. NULLs sort first ascending and last descending. Select fields sort by option order. Records always end up in a total order, because `id` is appended as the last key. With no `sort`, records come back in creation order.

`limit` is 1 to 200 (default 50). The response is `{"records": [...], "next_cursor": "..." | null}`. `cursor` is keyset-based: it encodes the last row's sort values, so pages stay correct when records are inserted or removed between requests. A cursor works only with the sort it came from.

`{table}` accepts either the table ID or its name. Record bodies use field names as keys, and the server maps them to columns via the catalog.

Errors use a JSON body `{ "error": { "code": "...", "message": "..." } }` with fitting HTTP status codes.

Interactive docs: `GET /api/docs` serves Swagger UI, and `GET /api/openapi.json` serves the OpenAPI 3 spec. Both are public. The spec is handwritten in `docs/openapi.json` and embedded with `include_str!`. Swagger UI's files live in `docs/swagger/` and are embedded with rust-embed, so the page works offline. A test compares the spec's paths with a list of the routes, so a new route cannot be left undocumented unnoticed.

## Frontend

A React SPA with these screens:

- Login.
- Table list and a table-creation flow.
- Grid view with inline editing, field management, filters, sorts and hidden columns.
- Kanban view grouped by a select field, with drag to change the value.
- Saved views per table.

The build output lives in `web/dist` and is embedded in the binary. In development, Vite proxies `/api` to the Rust server.

Built with Vite, React and TypeScript, and no UI library. Routing is hash-based (`#/t/<table>/v/<view>`, `#/settings`). Kanban drag uses native HTML drag and drop. Details worth knowing:

- First run: `GET /api/auth/status` returns `{"needs_setup": true}` while no user exists, and the login screen then offers to create the admin account.
- The UI works with field IDs and option IDs in filters, sorts and view configs. Display names come from the table's field list.
- The filter editor shows one flat group of conditions joined by `and` or `or`. A saved view with nested groups can be cleared but not edited in the UI; the API still accepts nested groups.
- Edits to a view stay in the browser until the user presses Save view. "All records" is not a stored view; its filter and sort are a working draft, and Save as view stores them.
- A stale view loads with its filter and sort switched off and a banner, so the list request cannot fail on a deleted field.
- Kanban loads up to 2000 records at once. The grid loads 100 at a time.
- Relation cells show the target record's primary field value, fetched per record and cached in the page.
- Select and multi-select options get a stable colour from their position in the option list (eight muted tones, drawn as a tint with a hairline border). Colours are not stored.
- Confirmations and text prompts are in-app dialogs, and errors from background actions appear as toasts, because native dialogs block browser automation and look out of place. The UI is dark only: near-black neutral surfaces, hairline borders, an off-white primary button, and one steel-blue accent for focus and counts.
- Limits: no column resize or reorder, no row drag, no bulk edit, no undo.

## Project layout

```
crm-p/
  Cargo.toml
  build.rs           makes sure web/dist has an index page
  Dockerfile         static musl build
  scripts/release.sh frontend + release binary
  DESIGN.md
  docs/              openapi.json and embedded Swagger UI files
  migrations/        system-table SQL, applied in order at startup
  src/
    main.rs          startup, config, router
    config.rs        env and flag parsing
    db.rs            connection setup, pragmas, migrations, writer and read pool
    error.rs         app error type and HTTP mapping
    api/             Axum routes, one module per area
    catalog/         table, field and view metadata, and the DDL behind them
    records/         record CRUD, filters, sorting, keyset pagination
    search/          FTS5 indexes and search
    views/           saved views: config validation, storage
    auth/            password hashing, sessions, API keys
  web/               React SPA (Vite): src/ holds one file per screen or widget
```

## Build order

1. Server skeleton: config, SQLite setup with WAL, migrations, health route, embedded static files.
2. Auth: users, sessions, API keys.
3. Catalog and DDL: create tables and fields, soft delete, rename.
4. Record CRUD with filters, sorting, pagination.
5. Relations and multi-select.
6. FTS5 search.
7. Views storage and API.
8. React SPA: grid, then Kanban.
9. Release build as a static binary.

### Step 9: release

- The release profile uses fat LTO, one codegen unit and symbol stripping. The binary is about 4.5 MB.
- `build.rs` writes a placeholder `web/dist/index.html` when the frontend is not built, so `cargo build` and `cargo test` work on a fresh checkout. A real `npm run build` replaces it.
- `scripts/release.sh` builds the frontend and then the binary. The `Dockerfile` builds a fully static musl binary and ships it alone in a `scratch` image, running as a non-root user with the database in `/data`.
- The server shuts down gracefully on Ctrl-C and SIGTERM.
- A macOS host cannot produce a fully static binary. The Docker build was not run in the development environment, because no Docker daemon was available there.

## Decisions on earlier open questions

- Registration: the first user registers and becomes admin. Admins create all later accounts. No invite links, no open sign-up.
- Filter syntax: a JSON tree, shared with saved views.
- Hard purge: not in v1. Soft-deleted tables and fields stay in the file until a later release adds an admin-only purge.
