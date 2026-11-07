import { useEffect, useState } from "react";
import { errorMessage, listRecords } from "./api";
import { labelOf, useRecordLabel } from "./labels";
import { Icon } from "./icons";
import type { Field } from "./types";

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
    case "text":
    case "phone":
      return (
        <input
          {...keys}
          type={field.type === "phone" ? "tel" : "text"}
          placeholder={field.type === "phone" ? "+14155552671" : ""}
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
      return (
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

/** Picks records of the relation's target table. Search uses the same full-text `q` as the grid. */
export function RelationInput({ field, value, onChange }: Pick<InputProps, "field" | "value" | "onChange">) {
  const target = field.config.table ?? "";
  const many = !!field.config.many;
  const chosen: string[] = many ? (Array.isArray(value) ? (value as string[]) : []) : typeof value === "string" ? [value] : [];
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<{ id: string; label: string }[]>([]);
  const [err, setErr] = useState("");
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    let live = true;
    const t = setTimeout(async () => {
      try {
        const r = await listRecords(target, { q, limit: 10 });
        const rows = await Promise.all(r.records.map(async (rec) => ({ id: rec.id, label: await labelOf(target, rec) })));
        if (live) {
          setHits(rows);
          setErr("");
        }
      } catch (e) {
        if (live) setErr(errorMessage(e));
      }
    }, 200);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [q, target, open]);

  const add = (id: string) => {
    onChange(many ? [...chosen, id] : id);
    setQ("");
    setOpen(false);
  };
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
      </div>
      {(many || chosen.length === 0) && (
        <>
          <input
            placeholder="Search records…"
            value={q}
            onFocus={() => setOpen(true)}
            onChange={(e) => {
              setQ(e.target.value);
              setOpen(true);
            }}
          />
          {open && (
            <div className="hits">
              {err && <div className="error">{err}</div>}
              {hits
                .filter((h) => !chosen.includes(h.id))
                .map((h) => (
                  <button type="button" key={h.id} className="hit" onMouseDown={(e) => e.preventDefault()} onClick={() => add(h.id)}>
                    {h.label}
                  </button>
                ))}
              {!err && hits.length === 0 && <div className="muted pad">No matching records</div>}
            </div>
          )}
        </>
      )}
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
    case "date":
      return <span>{String(value).slice(0, 10)}</span>;
    default:
      return <span>{String(value)}</span>;
  }
}
