# CRM

A no-code database for small teams, built as a single Rust binary on SQLite. Users create their own tables, fields and views; a generic REST API reads and writes the records.

See [DESIGN.md](DESIGN.md) for the architecture and build order.

## Run

```
cargo run
```

Environment variables:

- `CRM_DB`: SQLite file path (default `crm.db`)
- `CRM_BIND`: listen address (default `127.0.0.1:3000`)
- `CRM_COOKIE_SECURE`: set to `1` or `true` to add the `Secure` flag to the session cookie. Turn this on when serving over HTTPS.

Check it works:

```
curl http://127.0.0.1:3000/api/health
```

## Test

```
cargo test
```

## API documentation

Run the server and open http://127.0.0.1:3000/api/docs for an interactive Swagger UI. The raw spec is at `/api/openapi.json`. Both are public and work offline, because Swagger UI is embedded in the binary. The spec is written by hand in `docs/openapi.json`; when you add or change a route, update it. A test fails if a route is missing from the spec.

## Frontend

The React app is in `web/`. Build it before `cargo build --release`, because the binary embeds `web/dist`:

```
cd web && npm install && npm run build
```

For live development, run `cargo run` in one terminal and `npm run dev` in `web/` in another, then open http://localhost:5173. Vite proxies `/api` to the server.

## Release build

`scripts/release.sh` installs the frontend dependencies, builds the frontend, then runs `cargo build --release`. It prints the binary's path, size and checksum. The result is `target/release/crm`: one file with the React app embedded and SQLite compiled in. Copy it anywhere and run it. It needs no other files.

On macOS the binary still links the system libraries (`libSystem`, `libiconv`), so it is self-contained but not fully static. For a fully static Linux binary, use Docker:

```
docker build -t crm .
docker run -p 3000:3000 -v crm-data:/data crm
```

The image holds only the musl-linked binary, with the database in the `/data` volume. Without Docker, install the musl target and run `scripts/release.sh --target x86_64-unknown-linux-musl` on a Linux host that has a musl C toolchain. The server stops cleanly on Ctrl-C or SIGTERM.

For building on Linux, cross-compiling from a Mac, and running as a systemd service, see [docs/LINUX.md](docs/LINUX.md).
