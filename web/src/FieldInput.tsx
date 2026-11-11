import { useEffect, useState } from "react";
import { errorMessage, listRecords } from "./api";
import { labelOf, tableDetail, useRecordLabel } from "./labels";
import { Icon } from "./icons";
import type { Field, Rec } from "./types";

/** A stable colour class for an option, by its place in the field's option list. */
export function optionTone(field: Field, label: string): string {
  const i = field.options.findIndex((o) => o.label === label);
  return `tone-${(i < 0 ? 7 : i) % 8}`;
}

interface InputProps {
  field: Field;
  value: unknown;
  onChange: (v: unknown) => void;
  autoFocus?: boolean;
  onEnter?: () => void;
  onEscape?: () => void;
  onBlur?: () => void;
}

const pad2 = (n: number) => String(n).padStart(2, "0");

/** A stored date or UTC date-time as the local `YYYY-MM-DDTHH:mm` a datetime-local input wants. */
export function toLocalInput(v: unknown): string {
  if (typeof v !== "string" || !v) return "";
  if (v.length === 10) return `${v}T00:00`;
  const d = new Date(v);
  if (isNaN(d.getTime())) return "";
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}T${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

/** The reverse: a local datetime-local value as a UTC ISO string, or `null` when empty. */
export function fromLocalInput(s: string): string | null {
  if (!s) return null;
  const d = new Date(s);
  return isNaN(d.getTime()) ? null : d.toISOString();
}

/** One editor per field type. Values use the REST API's JSON shapes; empty input becomes `null`. */
export function FieldInput(p: InputProps) {
  const { field, value, onChange } = p;
  const keys = {
    autoFocus: p.autoFocus,
    onBlur: p.onBlur,
    onKeyDown: (e: React.KeyboardEvent) => {
      if (e.key === "Enter") p.onEnter?.();
      if (e.key === "Escape") p.onEscape?.();
    },
  };
  switch (field.type) {
    case "long_text":
      return (
        <textarea
          autoFocus={p.autoFocus}
          onBlur={p.onBlur}
          rows={4}
          value={(value as string | null) ?? ""}
          onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)}
          // Enter commits, Shift+Enter adds a line.
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey && p.onEnter) {
              e.preventDefault();
              p.onEnter();
            }
            if (e.key === "Escape") p.onEscape?.();
          }}
        />
      );
    case "text":
    case "phone":
    case "email":
      return (
        <input
          {...keys}
          type={field.type === "phone" ? "tel" : field.type === "email" ? "email" : "text"}
          placeholder={field.type === "phone" ? "+14155552671" : field.type === "email" ? "name@example.com" : ""}
          value={(value as string | null) ?? ""}
          onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)}
        />
      );
    case "number":
      return (
        <input
          {...keys}
          type="number"
          step={field.config.integer ? 1 : "any"}
          value={(value as number | null) ?? ""}
          onChange={(e) => onChange(e.target.value === "" ? null : Number(e.target.value))}
        />
      );
    case "date":
      return field.config.time ? (
        <input {...keys} type="datetime-local" value={toLocalInput(value)} onChange={(e) => onChange(fromLocalInput(e.target.value))} />
      ) : (
        <input
          {...keys}
          type="date"
          value={typeof value === "string" ? value.slice(0, 10) : ""}
          onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)}
        />
      );
    case "boolean":
      return <input type="checkbox" autoFocus={p.autoFocus} checked={value === true} onChange={(e) => onChange(e.target.checked)} />;
    case "select":
      return (
        <select {...keys} value={(value as string | null) ?? ""} onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)}>
          <option value="" />
          {field.options.map((o) => (
            <option key={o.id} value={o.label}>
              {o.label}
            </option>
          ))}
        </select>
      );
    case "multi_select": {
      const chosen = Array.isArray(value) ? (value as string[]) : [];
      const toggle = (label: string) => onChange(chosen.includes(label) ? chosen.filter((l) => l !== label) : [...chosen, label]);
      return (
        <div className="chips">
          {field.options.map((o) => (
            <button type="button" key={o.id} className={`chip pick ${optionTone(field, o.label)}${chosen.includes(o.label) ? " on" : ""}`} onClick={() => toggle(o.label)}>
              {o.label}
            </button>
          ))}
          {field.options.length === 0 && <span className="muted">No options yet</span>}
        </div>
      );
    }
    case "relation":
      return <RelationInput field={field} value={value} onChange={onChange} />;
  }
}

/** Chips for the linked records and a "+" that opens a searchable list of the target table's records, as Baserow does. */
export function RelationInput({ field, value, onChange }: Pick<InputProps, "field" | "value" | "onChange">) {
  const target = field.config.table ?? "";
  const many = !!field.config.many;
  const chosen: string[] = many ? (Array.isArray(value) ? (value as string[]) : []) : typeof value === "string" ? [value] : [];
  const [picking, setPicking] = useState(false);
  const remove = (id: string) => onChange(many ? chosen.filter((c) => c !== id) : null);

  return (
    <div className="relation">
      <div className="chips">
        {chosen.map((id) => (
          <span className="chip" key={id}>
            <RecordChipLabel table={target} id={id} />
            <button type="button" className="x" onClick={() => remove(id)} aria-label="Remove">
              <Icon name="x" size={12} />
            </button>
          </span>
        ))}
        <button type="button" className="chip pick add-link" onClick={() => setPicking(true)} aria-label="Link a record" title="Link a record">
          <Icon name="plus" size={12} />
        </button>
      </div>
      {picking && (
        <RecordPicker
          table={target}
          many={many}
          chosen={chosen}
          onChange={(ids) => onChange(many ? ids : (ids[0] ?? null))}
          onClose={() => setPicking(false)}
        />
      )}
    </div>
  );
}

interface PickRow {
  rec: Rec;
  label: string;
}

const PICK_PAGE = 25;

/** A modal list of the target table's records with search. A to-one link closes on pick; a to-many link stays open until Done. */
export function RecordPicker(p: { table: string; many: boolean; chosen: string[]; onChange: (ids: string[]) => void; onClose: () => void }) {
  const [q, setQ] = useState("");
  const [rows, setRows] = useState<PickRow[]>([]);
  const [next, setNext] = useState<string | null>(null);
  const [cols, setCols] = useState<Field[]>([]);
  const [err, setErr] = useState("");
  const [loading, setLoading] = useState(true);

  // Escape closes only this picker, not the record dialog underneath it.
  useEffect(() => {
    const esc = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopImmediatePropagation();
      p.onClose();
    };
    document.addEventListener("keydown", esc, true);
    return () => document.removeEventListener("keydown", esc, true);
  }, [p.onClose]);

  useEffect(() => {
    tableDetail(p.table).then((t) => setCols(t.fields.filter((f) => !f.is_primary && f.type !== "relation" && f.type !== "multi_select").slice(0, 2)), () => {});
  }, [p.table]);

  const load = async (cursor: string | null, live: () => boolean) => {
    try {
      const r = await listRecords(p.table, { q, limit: PICK_PAGE, cursor });
      const got = await Promise.all(r.records.map(async (rec) => ({ rec, label: await labelOf(p.table, rec) })));
      if (!live()) return;
      setRows((old) => (cursor ? [...old, ...got] : got));
      setNext(r.next_cursor);
      setErr("");
    } catch (e) {
      if (live()) setErr(errorMessage(e));
    } finally {
      if (live()) setLoading(false);
    }
  };
  useEffect(() => {
    let alive = true;
    setLoading(true);
    const t = setTimeout(() => load(null, () => alive), 200);
    return () => {
      alive = false;
      clearTimeout(t);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [q, p.table]);

  const pick = (id: string) => {
    if (!p.many) {
      p.onChange([id]);
      p.onClose();
    } else {
      p.onChange(p.chosen.includes(id) ? p.chosen.filter((c) => c !== id) : [...p.chosen, id]);
    }
  };

  return (
    <div className="backdrop" onMouseDown={(e) => e.target === e.currentTarget && p.onClose()}>
      <div className="modal wide" role="dialog" aria-label="Select records">
        <div className="modal-head">
          <h2>Select {p.many ? "records" : "a record"}</h2>
          <button type="button" className="btn icon-only ghost" onClick={p.onClose} aria-label="Close">
            <Icon name="x" size={16} />
          </button>
        </div>
        <div className="search-box">
          <Icon name="search" size={15} />
          <input autoFocus type="search" aria-label="Search records" placeholder="Search records…" value={q} onChange={(e) => setQ(e.target.value)} />
        </div>
        <div className="pick-list">
          {err && <div className="error">{err}</div>}
          {rows.map(({ rec, label }) => {
            const on = p.chosen.includes(rec.id);
            return (
              <button type="button" key={rec.id} className={`pick-row${on ? " on" : ""}`} onClick={() => pick(rec.id)}>
                {p.many && <input type="checkbox" readOnly checked={on} tabIndex={-1} />}
                <span className="pick-main">{label}</span>
                {cols.map((c) => (
                  <span className="pick-extra muted" key={c.id}>
                    {String(rec.values[c.name] ?? "")}
                  </span>
                ))}
              </button>
            );
          })}
          {!err && !loading && rows.length === 0 && <div className="muted pad">No matching records</div>}
          {loading && <div className="muted pad">Loading…</div>}
          {next && !loading && (
            <button type="button" className="btn small" onClick={() => load(next, () => true)}>
              Load more
            </button>
          )}
        </div>
        {p.many && (
          <div className="actions">
            <span className="muted spacer">{p.chosen.length} selected</span>
            <button type="button" className="btn primary" onClick={p.onClose}>
              Done
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

export function RecordChipLabel({ table, id }: { table: string; id: string }) {
  return <>{useRecordLabel(table, id)}</>;
}

/** Read-only display of a stored value. */
export function ValueView({ field, value }: { field: Field; value: unknown }) {
  if (value === null || value === undefined || value === "" || (Array.isArray(value) && value.length === 0)) return null;
  switch (field.type) {
    case "boolean":
      return value ? <Icon name="check" size={15} /> : null;
    case "select":
      return <span className={`chip ${optionTone(field, String(value))}`}>{String(value)}</span>;
    case "multi_select":
      return (
        <span className="chips">
          {(value as string[]).map((l) => (
            <span className={`chip ${optionTone(field, l)}`} key={l}>
              {l}
            </span>
          ))}
        </span>
      );
    case "relation": {
      const ids = Array.isArray(value) ? (value as string[]) : [value as string];
      return (
        <span className="chips">
          {ids.map((id) => (
            <span className="chip" key={id}>
              <RecordChipLabel table={field.config.table ?? ""} id={id} />
            </span>
          ))}
        </span>
      );
    }
    case "date": {
      const s = String(value);
      if (!field.config.time || s.length === 10) return <span>{s.slice(0, 10)}</span>;
      const d = new Date(s);
      return <span>{isNaN(d.getTime()) ? s : d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" })}</span>;
    }
    case "email":
      return (
        <a href={`mailto:${String(value)}`} onClick={(e) => e.stopPropagation()}>
          {String(value)}
        </a>
      );
    default:
      return <span>{String(value)}</span>;
  }
}
