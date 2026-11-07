# Building and running on Linux

There are two ways to get a Linux binary. Build on a Linux machine, or cross-compile from another machine such as a Mac. Both produce one self-contained file, `crm`, with the web app embedded and SQLite compiled in.

## Option 1: build on the Linux machine

1. Install the tools. On Debian or Ubuntu:

   ```sh
   sudo apt update
   sudo apt install build-essential curl
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   Install Node 20 or newer from https://nodejs.org or your package manager. Open a new shell afterwards so `cargo` is on the path.

2. Copy the source to the machine. Leave out build output and databases. On the machine that has the source:

   ```sh
   tar --exclude=target --exclude=node_modules --exclude='*.db*' -czf crm-src.tgz -C /path/to crm-p
   ```

   Copy `crm-src.tgz` over, then unpack it:

   ```sh
   tar xzf crm-src.tgz
   cd crm-p
   ```

3. Build:

   ```sh
   scripts/release.sh
   ```

   The script builds the web app, then the server. The result is `target/release/crm`.

This binary links the system C library (glibc). It runs on Linux machines with the same or a newer glibc than the one it was built on. To avoid that limit, use option 2, or build with the musl target on the machine (`rustup target add x86_64-unknown-linux-musl`, then `scripts/release.sh --target x86_64-unknown-linux-musl`; this needs `musl-tools` from apt).

## Option 2: cross-compile from a Mac

This gives a fully static binary that runs on any Linux of the chosen architecture, with no libraries needed. It has not been tested in the development environment.

1. Install the tools:

   ```sh
   brew install zig
   cargo install cargo-zigbuild
   rustup target add x86_64-unknown-linux-musl
   ```

   For ARM servers such as a Raspberry Pi or AWS Graviton, use `aarch64-unknown-linux-musl` here and in step 3.

2. Build the web app. It does not depend on the platform, so you build it once:

   ```sh
   cd web && npm ci && npm run build && cd ..
   ```

3. Build the server:

   ```sh
   cargo zigbuild --release --target x86_64-unknown-linux-musl
   ```

   The result is `target/x86_64-unknown-linux-musl/release/crm`.

4. Copy it to the Linux machine, for example `scp target/x86_64-unknown-linux-musl/release/crm user@server:~/`.

## Running it

On the Linux machine:

```sh
chmod +x crm
mkdir -p ~/crm-data
CRM_DB=$HOME/crm-data/crm.db ./crm
```

Open http://127.0.0.1:3000. The first screen creates the admin account. The API documentation is at `/api/docs`.

Settings are environment variables, all optional:

- `CRM_DB`: path of the database file. The default is `crm.db` in the current directory.
- `CRM_BIND`: address and port. The default is `127.0.0.1:3000`. Use `0.0.0.0:3000` to accept connections from other machines.
- `CRM_COOKIE_SECURE=1`: set this when the site is served over HTTPS.

The server speaks plain HTTP. If it is reachable from other machines, put an HTTPS reverse proxy such as Caddy or nginx in front of it and set `CRM_COOKIE_SECURE=1`.

### Running as a service

To keep it running and start it at boot, create `/etc/systemd/system/crm.service`:

```ini
[Unit]
Description=CRM
After=network.target

[Service]
ExecStart=/opt/crm/crm
Environment=CRM_DB=/var/lib/crm/crm.db
Environment=CRM_BIND=127.0.0.1:3000
DynamicUser=yes
StateDirectory=crm
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Put the binary at `/opt/crm/crm`, then run:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now crm
```

`systemctl stop crm` sends SIGTERM, and the server finishes in-flight requests before it exits.

### Backups

The database is one SQLite file, plus a `-wal` file next to it while the server runs. Stop the server and copy `crm.db` to back it up. For a backup while the server runs, use `sqlite3 /var/lib/crm/crm.db ".backup '/path/to/backup.db'"`.
