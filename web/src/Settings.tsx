import { useCallback, useEffect, useState, type ReactNode } from "react";
import { api, errorMessage } from "./api";
import { Icon, type IconName } from "./icons";
import type { User } from "./types";
import { ErrorText, initials, useDialogs } from "./ui";

interface Key {
  id: string;
  name: string;
  created_at: string;
  last_used_at: string | null;
}
interface Account extends User {
  created_at: string;
}

function Section(props: { icon: IconName; title: string; children: ReactNode }) {
  return (
    <section className="card section">
      <div className="section-head">
        <span className="tbl-ico">
          <Icon name={props.icon} size={15} />
        </span>
        <h2>{props.title}</h2>
      </div>
      <div className="section-body">{props.children}</div>
    </section>
  );
}

export function Settings({ user }: { user: User }) {
  return (
    <div className="main">
      <div className="head">
        <div>
          <h1>Settings</h1>
          <div className="subtitle">Access for people and for your own code.</div>
        </div>
      </div>
      <ApiKeys />
      {user.is_admin && <Users me={user} />}
    </div>
  );
}

function ApiKeys() {
  const [keys, setKeys] = useState<Key[]>([]);
  const [name, setName] = useState("");
  const [fresh, setFresh] = useState<string | null>(null);
  const [error, setError] = useState("");
  const { confirm } = useDialogs();

  const load = useCallback(() => api.get<Key[]>("/api/api-keys").then(setKeys).catch((e) => setError(errorMessage(e))), []);
  useEffect(() => {
    load();
  }, [load]);

  const create = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      setError("");
      const r = await api.post<{ key: string }>("/api/api-keys", { name });
      setFresh(r.key);
      setName("");
      load();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Section icon="key" title="API keys">
      <p>
        Send a key as <code>Authorization: Bearer &lt;key&gt;</code> to create, read, update and delete records from your own code.
      </p>
      {fresh && (
        <div className="banner" style={{ display: "block", margin: 0 }}>
          <strong>Copy this key now.</strong> It is not shown again.
          <code className="secret">{fresh}</code>
        </div>
      )}
      {keys.length > 0 && (
        <table className="plain">
          <tbody>
            {keys.map((k) => (
              <tr key={k.id}>
                <td>
                  <strong style={{ fontWeight: 600 }}>{k.name}</strong>
                </td>
                <td className="muted">{k.last_used_at ? `Last used ${k.last_used_at.slice(0, 10)}` : "Never used"}</td>
                <td>
                  <button
                    className="btn ghost small danger"
                    onClick={async () => {
                      const ok = await confirm({
                        title: `Delete the key “${k.name}”?`,
                        message: "Anything still using it stops working.",
                        confirmLabel: "Delete key",
                        danger: true,
                      });
                      if (!ok) return;
                      await api.del(`/api/api-keys/${k.id}`).catch((e) => setError(errorMessage(e)));
                      load();
                    }}
                  >
                    <Icon name="trash" size={13} /> Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <form className="add-row" onSubmit={create}>
        <input required placeholder="Key name, for example “Zapier”" autoComplete="off" value={name} onChange={(e) => setName(e.target.value)} />
        <button className="btn primary">
          <Icon name="plus" size={15} /> Create key
        </button>
      </form>
      <ErrorText text={error} />
    </Section>
  );
}

function Users({ me }: { me: User }) {
  const [users, setUsers] = useState<Account[]>([]);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [admin, setAdmin] = useState(false);
  const [error, setError] = useState("");
  const { confirm, prompt, toast } = useDialogs();

  const load = useCallback(() => api.get<Account[]>("/api/users").then(setUsers).catch((e) => setError(errorMessage(e))), []);
  useEffect(() => {
    load();
  }, [load]);

  const act = async (fn: () => Promise<unknown>, done?: string) => {
    try {
      setError("");
      await fn();
      if (done) toast(done, "info");
      load();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Section icon="users" title="Users">
      <table className="plain">
        <tbody>
          {users.map((u) => (
            <tr key={u.id}>
              <td>
                <span className="row">
                  <span className="avatar">{initials(u.email)}</span>
                  <span>
                    {u.email}
                    {u.id === me.id && <span className="muted"> · you</span>}
                  </span>
                </span>
              </td>
              <td>
                <label className="inline">
                  <input type="checkbox" checked={u.is_admin} onChange={(e) => act(() => api.patch(`/api/users/${u.id}`, { is_admin: e.target.checked }))} /> Admin
                </label>
              </td>
              <td>
                <button
                  className="btn ghost small"
                  onClick={async () => {
                    const pw = await prompt({
                      title: `New password for ${u.email}`,
                      label: "At least 8 characters. They are signed out everywhere.",
                      confirmLabel: "Set password",
                      password: true,
                    });
                    if (pw) act(() => api.patch(`/api/users/${u.id}`, { password: pw }), "Password changed");
                  }}
                >
                  Reset password
                </button>
                <button
                  className="btn ghost small danger"
                  onClick={async () => {
                    const ok = await confirm({ title: `Delete ${u.email}?`, message: "They can no longer sign in. Their records stay.", confirmLabel: "Delete user", danger: true });
                    if (ok) act(() => api.del(`/api/users/${u.id}`));
                  }}
                >
                  <Icon name="trash" size={13} /> Delete
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <form
        className="add-row"
        onSubmit={(e) => {
          e.preventDefault();
          act(async () => {
            await api.post("/api/users", { email, password, is_admin: admin });
            setEmail("");
            setPassword("");
            setAdmin(false);
          });
        }}
      >
        <input type="email" required autoComplete="off" placeholder="Email" value={email} onChange={(e) => setEmail(e.target.value)} />
        <input type="password" required minLength={8} autoComplete="new-password" placeholder="Password (8+ characters)" value={password} onChange={(e) => setPassword(e.target.value)} />
        <label className="inline">
          <input type="checkbox" checked={admin} onChange={(e) => setAdmin(e.target.checked)} /> Admin
        </label>
        <button className="btn primary">
          <Icon name="plus" size={15} /> Add user
        </button>
      </form>
      <ErrorText text={error} />
    </Section>
  );
}
