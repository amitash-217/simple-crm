-- Gives every existing table a `Last modified` field, which shows each record's updated_at.
-- New tables get one from create_table. The ID is a random UUID-shaped string, materialized once per table.
WITH n AS MATERIALIZED (
    SELECT id AS table_id, lower(hex(randomblob(16))) AS h FROM tables WHERE deleted_at IS NULL
)
INSERT INTO fields (id, table_id, name, sql_name, type, config_json, position, is_primary, created_at)
SELECT
    substr(h, 1, 8) || '-' || substr(h, 9, 4) || '-' || substr(h, 13, 4) || '-' || substr(h, 17, 4) || '-' || substr(h, 21),
    table_id,
    CASE WHEN EXISTS (SELECT 1 FROM fields f WHERE f.table_id = n.table_id AND f.deleted_at IS NULL
                      AND lower(f.name) = 'last modified')
         THEN 'Last modified (auto)' ELSE 'Last modified' END,
    'f_' || h,
    'last_modified',
    '{}',
    (SELECT COALESCE(MAX(position), -1) + 1 FROM fields f WHERE f.table_id = n.table_id),
    0,
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
FROM n;
