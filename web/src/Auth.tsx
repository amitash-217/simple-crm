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
    <div className="auth-split">
      <form className="auth" onSubmit={submit}>
        <Logo size={36} />
        <div>
          <h1>{setup ? "Set up your workspace" : "Sign in"}</h1>
          <p className="muted" style={{ marginTop: 4 }}>
            {setup ? "Create the admin account. It can add everyone else later." : "Use the account your admin created for you"}
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
      <BoardPreview />
    </div>
  );
}

// Sample deals for the sign-in preview, shaped like a real board view.
const PREVIEW: { title: string; tone: number; cards: { name: string; company: string; amount: string }[] }[] = [
  { title: "Lead", tone: 0, cards: [{ name: "Website rebuild", company: "Northwind", amount: "$12,400" }, { name: "Annual support", company: "Halden & Co", amount: "$8,000" }] },
  { title: "Proposal", tone: 2, cards: [{ name: "Fleet tracking", company: "Ortega Freight", amount: "$31,500" }, { name: "Onboarding pack", company: "Lumen Clinic", amount: "$4,200" }] },
  { title: "Won", tone: 1, cards: [{ name: "Data migration", company: "Brightwater", amount: "$18,900" }] },
];

/** A static board of sample deals: what a table looks like once it is set up. */
function BoardPreview() {
  return (
    <div className="auth-preview" aria-hidden="true">
      <div className="preview-window">
        <div className="preview-head">
          <span className="tile tone-3">D</span>
          Deals
          <span className="muted">· Pipeline</span>
        </div>
        <div className="board">
          {PREVIEW.map((col) => (
            <div key={col.title} className={`column tone-${col.tone}`}>
              <div className="column-head">
                <span className="dot" />
                <span>{col.title}</span>
                <span className="count-pill">{col.cards.length}</span>
              </div>
              {col.cards.map((c) => (
                <div key={c.name} className="card-item">
                  <div className="card-title">{c.name}</div>
                  <div className="card-line">
                    <span className="k">Company</span>
                    {c.company}
                  </div>
                  <div className="card-line">
                    <span className="k">Amount</span>
                    {c.amount}
                  </div>
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
