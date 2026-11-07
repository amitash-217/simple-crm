import { useEffect, useState } from "react";
import { api, errorMessage } from "./api";
import { Logo } from "./icons";
import type { User } from "./types";
import { ErrorText } from "./ui";

/** Sign-in, or first-run setup of the admin account when the database has no users yet. */
export function Auth({ onUser }: { onUser: (u: User) => void }) {
  const [setup, setSetup] = useState<boolean | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api
      .get<{ needs_setup: boolean }>("/api/auth/status")
      .then((s) => setSetup(s.needs_setup))
      .catch((e) => setError(errorMessage(e)));
  }, []);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      onUser(await api.post<User>(setup ? "/api/auth/register" : "/api/auth/login", { email, password }));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  if (setup === null) return <div className="center">{error && <ErrorText text={error} />}</div>;
  return (
    <div className="center">
      <form className="card auth" onSubmit={submit}>
        <Logo size={36} />
        <div>
          <h1>{setup ? "Set up your workspace" : "Welcome back"}</h1>
          <p className="muted" style={{ marginTop: 4 }}>
            {setup ? "Create the admin account. It can add everyone else later." : "Sign in to continue."}
          </p>
        </div>
        <label>
          <span className="label">Email</span>
          <input type="email" autoFocus required autoComplete="username" value={email} onChange={(e) => setEmail(e.target.value)} />
        </label>
        <label>
          <span className="label">Password{setup && " (8 or more characters)"}</span>
          <input
            type="password"
            required
            minLength={setup ? 8 : undefined}
            autoComplete={setup ? "new-password" : "current-password"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        <ErrorText text={error} />
        <button className="btn primary" disabled={busy}>
          {setup ? "Create account" : "Sign in"}
        </button>
      </form>
    </div>
  );
}
