-- A table's colour name and icon name, chosen in the web app. NULL means the default.
ALTER TABLE tables ADD COLUMN color TEXT;
ALTER TABLE tables ADD COLUMN icon TEXT;
