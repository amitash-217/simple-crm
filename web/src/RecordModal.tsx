import { useEffect, useRef, useState } from "react";
import { api, errorMessage } from "./api";
import { FieldInput } from "./FieldInput";
import type { Rec, TableDetail } from "./types";
import { Icon, typeIcon } from "./icons";
import { Drawer, ErrorText, Popover, useDialogs } from "./ui";

interface Props {
  table: TableDetail;
  /** The record to edit, or null to create one. */
  record: Rec | null;
  /** Starting values for a new record, keyed by field ID. */
  initial?: Record<string, unknown>;
  onClose: () => void;
  /** A new record was created. */
  onSaved: (rec: Rec) => void;
  /** An open record was changed. The pane stays open. */
  onChanged?: (rec: Rec) => void;
  onDeleted?: (rec: Rec) => void;
}

const stamp = (s: string) => s.replace("T", " ").replace("Z", "");

/** The record side pane. An open record saves as you edit. A new record is created with the Create button. */
export function RecordModal({ table, record, initial, onClose, onSaved, onChanged, onDeleted }: Props) {
  const original = (id: string, name: string): unknown => (record ? (record.values[name] ?? null) : (initial?.[id] ?? null));
  const [draft, setDraft] = useState<Record<string, unknown>>(() => Object.fromEntries(table.fields.map((f) => [f.id, original(f.id, f.name)])));
  // What the server has, so an edit sends only what differs from it.
  const saved = useRef<Record<string, unknown>>(draft);
  const [error, setError] = useState("");
  const [status, setStatus] = useState<"" | "saving" | "saved">("");
  const { confirm } = useDialogs();
  const base = `/api/tables/${encodeURIComponent(table.id)}/records`;
  const primary = table.fields.find((f) => f.is_primary);
  const title = record ? String(record.values[primary?.name ?? ""] ?? "").trim() || "Untitled" : `New ${table.name} record`;

  const changes = () => {
    const values: Record<string, unknown> = {};
    for (const f of table.fields) {
      const now = draft[f.id];
      if (record ? JSON.stringify(now) !== JSON.stringify(saved.current[f.id]) : now !== null && now !== undefined) values[f.id] = now;
    }
    return values;
  };

  const persist = async (): Promise<boolean> => {
    if (!record) return true;
    const values = changes();
    if (Object.keys(values).length === 0) return true;
    setStatus("saving");
    try {
      const rec = await api.patch<Rec>(`${base}/${record.id}`, { values });
      saved.current = { ...saved.current, ...values };
      setError("");
      setStatus("saved");
      onChanged?.(rec);
      return true;
    } catch (err) {
      setError(errorMessage(err));
      setStatus("");
      return false;
    }
  };

  // Save a moment after the last edit.
  useEffect(() => {
    if (!record || Object.keys(changes()).length === 0) return;
    const t = setTimeout(persist, 800);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draft]);

  const create = async (e: React.FormEvent) => {
    e.preventDefault();
    setStatus("saving");
    try {
      onSaved(await api.post<Rec>(base, { values: changes() }));
    } catch (err) {
      setError(errorMessage(err));
      setStatus("");
    }
  };

  // Opening another record, or leaving the page, saves a pending edit instead of dropping it.
  const flush = useRef(persist);
  flush.current = persist;
  const gone = useRef(false);
  useEffect(
    () => () => {
      if (!gone.current) void flush.current();
    },
    [],
  );

  const close = async () => {
    if (await persist()) onClose();
  };

  const remove = async () => {
    if (!record) return;
    const ok = await confirm({ title: "Delete this record?", message: "This cannot be undone.", confirmLabel: "Delete record", danger: true });
    if (!ok) return;
    try {
      await api.del(`${base}/${record.id}`);
      gone.current = true;
      onDeleted?.(record);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const actions = record && (
    <Popover icon="more" title="Record options" align="right">
      <div className="menu">
        <button className="btn ghost danger" onClick={remove}>
          <Icon name="trash" size={16} /> Delete record
        </button>
      </div>
    </Popover>
  );

  return (
    <Drawer title={title} chip={title.charAt(0).toUpperCase()} actions={actions} onClose={close}>
      <form className={`rec-form${record ? "" : " fresh"}`} onSubmit={create}>
        <div className="rec-fields">
          {table.fields.map((f) => (
            <div key={f.id} className="rec-row">
              <span className="rec-label" title={f.name}>
                <Icon name={typeIcon(f.type)} size={16} />
                <span>{f.name}</span>
              </span>
              <div className="rec-value">
                <FieldInput field={f} value={draft[f.id]} onChange={(v) => setDraft({ ...draft, [f.id]: v })} />
              </div>
            </div>
          ))}
        </div>
        <ErrorText text={error} />
        {record ? (
          <p className="rec-meta">
            <span>
              Created {stamp(record.created_at)} · Updated {stamp(record.updated_at)}
            </span>
            <span>{status === "saving" ? "Saving…" : status === "saved" ? "Saved" : ""}</span>
          </p>
        ) : (
          <div className="actions">
            <span className="spacer" />
            <button type="button" className="btn" onClick={onClose}>
              Cancel
            </button>
            <button className="btn primary" disabled={status === "saving"}>
              Create
            </button>
          </div>
        )}
      </form>
    </Drawer>
  );
}
