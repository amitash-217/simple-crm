import { useCallback, useEffect, useState } from "react";
import { api, errorMessage } from "./api";
import { Auth } from "./Auth";
import { Icon, Logo } from "./icons";
import { Settings } from "./Settings";
import { TablePage } from "./TablePage";
import type { Table, User } from "./types";
import { CardStack, DialogProvider, ErrorText, initials, Modal, TableTile, tableTone, toneOf } from "./ui";

/** The theme in effect: the saved choice, else the system's. */
const currentTheme = (): "light" | "dark" => {
  const set = document.documentElement.dataset.theme;
  if (set === "light" || set === "dark") return set;
  return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
};

/** Switches between the light and dark theme and remembers the choice on this device. */
function ThemeToggle() {
  const [theme, setTheme] = useState(currentTheme);
  const flip = () => {
    const next = theme === "dark" ? "light" : "dark";
    const root = document.documentElement;
    root.classList.add("theming");
    setTimeout(() => root.classList.remove("theming"), 300);
    root.dataset.theme = next;
    try {
      localStorage.setItem("theme", next);
    } catch {
      // Private mode: the choice lasts until the tab closes.
    }
    setTheme(next);
  };
  return (
    <button className="nav-item" onClick={flip} aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}>
      <Icon name={theme === "dark" ? "sun" : "moon"} size={16} /> {theme === "dark" ? "Light mode" : "Dark mode"}
    </button>
  );
}

/** Counts writes on the server, from this page, another tab or the API. Starts at 0. */
function useChanges(): number {
  const [n, setN] = useState(0);
  useEffect(() => {
    let stop = false;
    (async () => {
      let since: number | undefined;
      while (!stop) {
        try {
          // Long-poll: the server answers when the version moves, or after about 25 seconds.
          const { version } = await api.get<{ version: number }>(`/api/changes${since === undefined ? "" : `?since=${since}`}`);
          if (!stop && since !== undefined && version !== since) setN((x) => x + 1);
          since = version;
        } catch {
          await new Promise((r) => setTimeout(r, 5000));
        }
      }
    })();
    return () => {
      stop = true;
    };
  }, []);
  return n;
}

function useHash(): string {
  const [hash, setHash] = useState(location.hash);
  useEffect(() => {
    const on = () => setHash(location.hash);
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  return hash.replace(/^#\/?/, "");
}

export function App() {
  // undefined: still checking the session. null: signed out.
  const [user, setUser] = useState<User | null | undefined>(undefined);
  useEffect(() => {
    api
      .get<User>("/api/auth/me")
      .then(setUser)
      .catch(() => setUser(null));
  }, []);

  if (user === undefined) return <div className="center" />;
  if (user === null) return <Auth onUser={setUser} />;
  return (
    <DialogProvider>
      <Shell
        user={user}
        onLogout={async () => {
          await api.post("/api/auth/logout").catch(() => {});
          location.hash = "";
          setUser(null);
        }}
      />
    </DialogProvider>
  );
}

function Shell({ user, onLogout }: { user: User; onLogout: () => void }) {
  const route = useHash().split("/").filter(Boolean);
  const [tables, setTables] = useState<Table[] | null>(null);
  const [error, setError] = useState("");
  const [creating, setCreating] = useState(false);

  const reload = useCallback(() => api.get<Table[]>("/api/tables").then(setTables).catch((e) => setError(errorMessage(e))), []);
  useEffect(() => {
    reload();
  }, [reload]);
  const changes = useChanges();
  useEffect(() => {
    if (changes) reload();
  }, [changes, reload]);

  const onSettings = route[0] === "settings";
  // With no table in the address, show the first one.
  const tableId = route[0] === "t" ? route[1] : onSettings ? null : (tables?.[0]?.id ?? null);
  const viewId = route[0] === "t" && route[2] === "v" ? (route[3] ?? null) : null;
  const navigate = (h: string) => {
    location.hash = h;
  };

  return (
    <div className="shell">
      <nav className="sidebar" aria-label="Main">
        <div className="brand">
          <Logo />
          CRM
        </div>
        <div className="nav-head">
          <span>Tables</span>
        </div>
        <div className="nav-list">
          {tables?.map((t) => (
            <a key={t.id} href={`#/t/${t.id}`} className={`nav-item tinted tone-${tableTone(t)}${t.id === tableId && !onSettings ? " on" : ""}`}>
              <TableTile table={t} />
              <span className="nm">{t.name}</span>
            </a>
          ))}
          <button className="nav-item add" onClick={() => setCreating(true)}>
            <Icon name="plus" size={16} /> New table
          </button>
        </div>
        <div className="nav-foot">
          <a href="#/settings" className={`nav-item${onSettings ? " on" : ""}`}>
            <Icon name="settings" size={16} /> Settings
          </a>
          <ThemeToggle />
          <button className="nav-item" onClick={onLogout}>
            <Icon name="logout" size={16} /> Sign out
          </button>
          <div className="who">
            <span className={`avatar tone-${toneOf(user.email)}`}>{initials(user.email)}</span>
            <span className="who-text">
              <span>{user.email}</span>
              <span>{user.is_admin ? "Admin" : "Member"}</span>
            </span>
          </div>
        </div>
      </nav>
      {onSettings ? (
        <Settings user={user} />
      ) : tableId ? (
        <TablePage key={tableId} tableId={tableId} viewId={viewId} tables={tables ?? []} navigate={navigate} onTablesChanged={reload} changes={changes} />
      ) : (
        <div className="main">
          {error && <ErrorText text={error} />}
          {tables && (
            <div className="card welcome">
              <CardStack />
              <h1>Build your workspace</h1>
              <p className="muted">There are no tables yet. You decide what to track and how.</p>
              <ul>
                <li className="tone-0">
                  <strong>Tables</strong> hold your records, such as companies, deals or tickets.
                </li>
                <li className="tone-1">
                  <strong>Fields</strong> are the columns: text, numbers, dates, selects and links between tables.
                </li>
                <li className="tone-2">
                  <strong>Views</strong> save a filter and sort, as a grid or a Kanban board.
                </li>
              </ul>
              <button className="btn primary" onClick={() => setCreating(true)}>
                <Icon name="plus" size={16} /> Create your first table
              </button>
            </div>
          )}
        </div>
      )}
      {creating && (
        <NewTable
          onClose={() => setCreating(false)}
          onCreated={async (id) => {
            setCreating(false);
            await reload();
            navigate(`#/t/${id}`);
          }}
        />
      )}
    </div>
  );
}

function NewTable({ onClose, onCreated }: { onClose: () => void; onCreated: (id: string) => void }) {
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  return (
    <Modal title="New table" onClose={onClose}>
      <form
        className="form"
        onSubmit={async (e) => {
          e.preventDefault();
          try {
            onCreated((await api.post<Table>("/api/tables", { name })).id);
          } catch (err) {
            setError(errorMessage(err));
          }
        }}
      >
        <label>
          Name
          <input autoFocus required value={name} onChange={(e) => setName(e.target.value)} placeholder="Companies" />
        </label>
        <p className="muted" style={{ margin: 0 }}>
          A new table starts with one text field, Name. Add the rest yourself.
        </p>
        <ErrorText text={error} />
        <div className="actions">
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn primary">Create table</button>
        </div>
      </form>
    </Modal>
  );
}
