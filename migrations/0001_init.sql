CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    is_admin      INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL
);

CREATE TABLE sessions (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE api_keys (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    key_hash     TEXT NOT NULL UNIQUE,
    created_at   TEXT NOT NULL,
    last_used_at TEXT
);

CREATE TABLE tables (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    sql_name   TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    created_by TEXT REFERENCES users(id),
    deleted_at TEXT
);

CREATE TABLE fields (
    id          TEXT PRIMARY KEY,
    table_id    TEXT NOT NULL REFERENCES tables(id),
    name        TEXT NOT NULL,
    sql_name    TEXT NOT NULL,
    type        TEXT NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    position    INTEGER NOT NULL DEFAULT 0,
    is_primary  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    deleted_at  TEXT,
    UNIQUE (table_id, sql_name)
);

CREATE TABLE field_options (
    id       TEXT PRIMARY KEY,
    field_id TEXT NOT NULL REFERENCES fields(id),
    label    TEXT NOT NULL,
    color    TEXT,
    position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE views (
    id          TEXT PRIMARY KEY,
    table_id    TEXT NOT NULL REFERENCES tables(id),
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('grid', 'kanban')),
    config_json TEXT NOT NULL DEFAULT '{}',
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    created_by  TEXT REFERENCES users(id)
);

CREATE INDEX idx_fields_table ON fields(table_id);
CREATE INDEX idx_field_options_field ON field_options(field_id);
CREATE INDEX idx_views_table ON views(table_id);
