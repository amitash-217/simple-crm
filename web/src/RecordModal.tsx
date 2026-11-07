import { useState } from "react";
import { api, errorMessage } from "./api";
import { FieldInput } from "./FieldInput";
import type { Rec, TableDetail } from "./types";
import { Icon } from "./icons";
import { ErrorText, Modal, useDialogs } from "./ui";

interface Props {
  table: TableDetail;
  /** The record to edit, or null to create one. */
  record: Rec | null;
  /** Starting values for a new record, keyed by field ID. */
  initial?: Record<string, unknown>;
  onClose: () => void;
  onSaved: (rec: Rec) => void;
  onDeleted?: (rec: Rec) => void;
}

export function RecordModal({ table, record, initial, onClose, onSaved, onDeleted }: Props) {
  const original = (id: string, name: string): unknown => (record ? (record.values[name] ?? null) : (initial?.[id] ?? null));
  const [draft, setDraft] = useState<Record<string, unknown>>(() => Object.fromEntries(table.fields.map((f) => [f.id, original(f.id, f.name)])));
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const { confirm } = useDialogs();
  const base = `/api/tables/${encodeURIComponent(table.id)}/records`;

  const save = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError("");
    // An edit sends only what changed. A new record sends only what has a value.
    const values: Record<string, unknown> = {};
    for (const f of table.fields) {
      const now = draft[f.id];
      const before = original(f.id, f.name);
      if (record ? JSON.stringify(now) !== JSON.stringify(before) : now !== null && now !== undefined) values[f.id] = now;
    }
    try {
      onSaved(record ? await api.patch<Rec>(`${base}/${record.id}`, { values }) : await api.post<Rec>(base, { values }));
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  const remove = async () => {
    if (!record) return;
    const ok = await confirm({ title: "Delete this record?", message: "This cannot be undone.", confirmLabel: "Delete record", danger: true });
    if (!ok) return;
    try {
      await api.del(`${base}/${record.id}`);
      onDeleted?.(record);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Modal title={record ? "Record" : `New ${table.name} record`} onClose={onClose} wide>
      <form className="form" onSubmit={save}>
        {table.fields.map((f) => (
          <div key={f.id} className={`field-row${f.type === "boolean" ? " inline" : ""}`}>
            <span className="label">{f.name}</span>
            <FieldInput field={f} value={draft[f.id]} onChange={(v) => setDraft({ ...draft, [f.id]: v })} />
          </div>
        ))}
        {record && (
          <p className="muted" style={{ margin: 0, fontSize: 12.5 }}>
            Created {record.created_at.replace("T", " ").replace("Z", "")} · Updated {record.updated_at.replace("T", " ").replace("Z", "")}
          </p>
        )}
        <ErrorText text={error} />
        <div className="actions">
          {record && (
            <button type="button" className="btn ghost danger" onClick={remove}>
              <Icon name="trash" size={15} /> Delete
            </button>
          )}
          <span className="spacer" />
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn primary" disabled={busy}>
            Save
          </button>
        </div>
      </form>
    </Modal>
  );
}
