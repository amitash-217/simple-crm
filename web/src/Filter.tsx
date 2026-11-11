import { useState } from "react";
import { RelationInput, fromLocalInput, toLocalInput } from "./FieldInput";
import { Icon } from "./icons";
import type { Field, Sort } from "./types";

type Mode = "and" | "or";
export interface Cond {
  field: string;
  op: string;
  value?: unknown;
}

const NO_VALUE = ["is_empty", "is_not_empty"];

const OPS: Record<string, string[]> = {
  text: ["contains", "eq", "neq", "starts_with", "is_empty", "is_not_empty"],
  number: ["eq", "neq", "gt", "gte", "lt", "lte", "is_empty", "is_not_empty"],
  date: ["eq", "neq", "gt", "gte", "lt", "lte", "is_empty", "is_not_empty"],
  boolean: ["eq", "is_empty", "is_not_empty"],
  select: ["eq", "neq", "is_empty", "is_not_empty"],
  multi: ["has", "is_empty", "is_not_empty"],
  link: ["eq", "neq", "is_empty", "is_not_empty"],
};

const OP_LABEL: Record<string, string> = {
  eq: "is",
  neq: "is not",
  contains: "contains",
  starts_with: "starts with",
  gt: ">",
  gte: "≥",
  lt: "<",
  lte: "≤",
  has: "includes",
  is_empty: "is empty",
  is_not_empty: "is not empty",
};

function opsFor(f: Field): string[] {
  if (f.type === "text" || f.type === "long_text" || f.type === "email" || f.type === "phone") return OPS.text;
  if (f.type === "multi_select" || (f.type === "relation" && (f.config.many || f.config.related))) return OPS.multi;
  if (f.type === "relation") return OPS.link;
  return OPS[f.type];
}

const complete = (c: Cond) => NO_VALUE.includes(c.op) || (c.value !== undefined && c.value !== null && c.value !== "");

/** Reads a stored filter back into rows. Returns null for trees this editor cannot show (nested groups). */
function parse(filter: unknown): { mode: Mode; rows: Cond[] } | null {
  if (!filter) return { mode: "and", rows: [] };
  const f = filter as Record<string, unknown>;
  const isCond = (x: any) => x && typeof x === "object" && typeof x.field === "string";
  if (isCond(f)) return { mode: "and", rows: [f as unknown as Cond] };
  const mode = "and" in f ? "and" : "or" in f ? "or" : null;
  const items = mode ? (f[mode] as unknown[]) : null;
  if (!mode || !Array.isArray(items) || !items.every(isCond)) return null;
  return { mode, rows: items as Cond[] };
}

export function FilterEditor({ fields, filter, onChange }: { fields: Field[]; filter: unknown; onChange: (f: unknown) => void }) {
  const parsed = parse(filter);
  const [mode, setMode] = useState<Mode>(parsed?.mode ?? "and");
  const [rows, setRows] = useState<Cond[]>(parsed?.rows ?? []);

  if (!parsed) {
    return (
      <div className="panel-body">
        <p className="muted">This view uses nested filter groups, which this editor cannot show.</p>
        <button className="btn" onClick={() => onChange(undefined)}>
          Clear filter
        </button>
      </div>
    );
  }

  // Only complete rows reach the server, so a half-typed row never breaks the list.
  const emit = (m: Mode, rs: Cond[]) => {
    setMode(m);
    setRows(rs);
    const ok = rs.filter(complete).map((c) => (NO_VALUE.includes(c.op) ? { field: c.field, op: c.op } : c));
    onChange(ok.length ? { [m]: ok } : undefined);
  };
  const set = (i: number, patch: Partial<Cond>) => emit(mode, rows.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const add = () => emit(mode, [...rows, { field: fields[0].id, op: opsFor(fields[0])[0] }]);

  return (
    <div className="panel-body">
      {rows.length === 0 && <p className="muted" style={{ margin: 0 }}>No filters. Showing every record.</p>}
      {rows.map((r, i) => {
        const field = fields.find((f) => f.id === r.field);
        return (
          <div className="row" key={i}>
            {i === 0 ? (
              <span className="lead">Where</span>
            ) : i === 1 ? (
              <select className="lead" value={mode} onChange={(e) => emit(e.target.value as Mode, rows)}>
                <option value="and">and</option>
                <option value="or">or</option>
              </select>
            ) : (
              <span className="lead">{mode}</span>
            )}
            <select
              value={r.field}
              onChange={(e) => {
                const nf = fields.find((f) => f.id === e.target.value)!;
                set(i, { field: nf.id, op: opsFor(nf)[0], value: undefined });
              }}
            >
              {!field && <option value={r.field}>(deleted field)</option>}
              {fields.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.name}
                </option>
              ))}
            </select>
            {field && (
              <select value={r.op} onChange={(e) => set(i, { op: e.target.value, value: NO_VALUE.includes(e.target.value) ? undefined : r.value })}>
                {opsFor(field).map((op) => (
                  <option key={op} value={op}>
                    {OP_LABEL[op]}
                  </option>
                ))}
              </select>
            )}
            {field && !NO_VALUE.includes(r.op) && <ValueInput field={field} value={r.value} onChange={(v) => set(i, { value: v })} />}
            <button className="btn ghost icon-only" onClick={() => emit(mode, rows.filter((_, j) => j !== i))} aria-label="Remove filter">
              <Icon name="x" size={15} />
            </button>
          </div>
        );
      })}
      <button className="btn" onClick={add}>
        <Icon name="plus" size={15} /> Add filter
      </button>
    </div>
  );
}

/** Filter values use option IDs for select fields; the server accepts labels or IDs. */
function ValueInput({ field, value, onChange }: { field: Field; value: unknown; onChange: (v: unknown) => void }) {
  switch (field.type) {
    case "number":
      return <input type="number" value={(value as number | undefined) ?? ""} onChange={(e) => onChange(e.target.value === "" ? undefined : Number(e.target.value))} />;
    case "date":
      return field.config.time ? (
        <input type="datetime-local" value={toLocalInput(value)} onChange={(e) => onChange(fromLocalInput(e.target.value) ?? undefined)} />
      ) : (
        <input type="date" value={(value as string | undefined)?.slice(0, 10) ?? ""} onChange={(e) => onChange(e.target.value || undefined)} />
      );
    case "boolean":
      return (
        <select value={value === undefined ? "" : String(value)} onChange={(e) => onChange(e.target.value === "" ? undefined : e.target.value === "true")}>
          <option value="" />
          <option value="true">checked</option>
          <option value="false">unchecked</option>
        </select>
      );
    case "select":
    case "multi_select":
      return (
        <select value={(value as string | undefined) ?? ""} onChange={(e) => onChange(e.target.value || undefined)}>
          <option value="" />
          {field.options.map((o) => (
            <option key={o.id} value={o.id}>
              {o.label}
            </option>
          ))}
        </select>
      );
    case "relation":
      // A filter compares against one record, even for a to-many field.
      return <RelationInput field={{ ...field, config: { ...field.config, many: false } }} value={value} onChange={(v) => onChange(v ?? undefined)} />;
    default:
      return <input value={(value as string | undefined) ?? ""} onChange={(e) => onChange(e.target.value || undefined)} />;
  }
}

const SYSTEM_SORTS = [
  { id: "$created_at", name: "Created" },
  { id: "$updated_at", name: "Updated" },
];

export function SortEditor({ fields, sort, onChange }: { fields: Field[]; sort: Sort[]; onChange: (s: Sort[] | undefined) => void }) {
  const sortable = [...fields.filter((f) => f.type !== "multi_select" && !(f.type === "relation")).map((f) => ({ id: f.id, name: f.name })), ...SYSTEM_SORTS];
  const emit = (s: Sort[]) => onChange(s.length ? s : undefined);
  const used = new Set(sort.map((s) => s.field));
  const free = sortable.filter((f) => !used.has(f.id));
  return (
    <div className="panel-body">
      {sort.length === 0 && <p className="muted" style={{ margin: 0 }}>No sorting. Records appear in creation order.</p>}
      {sort.map((s, i) => (
        <div className="row" key={i}>
          <span className="lead">{i === 0 ? "Sort by" : "then by"}</span>
          <select value={s.field} onChange={(e) => emit(sort.map((x, j) => (j === i ? { ...x, field: e.target.value } : x)))}>
            {sortable
              .filter((f) => f.id === s.field || !used.has(f.id))
              .map((f) => (
                <option key={f.id} value={f.id}>
                  {f.name}
                </option>
              ))}
            {!sortable.some((f) => f.id === s.field) && <option value={s.field}>(deleted field)</option>}
          </select>
          <select value={s.dir} onChange={(e) => emit(sort.map((x, j) => (j === i ? { ...x, dir: e.target.value as "asc" | "desc" } : x)))}>
            <option value="asc">ascending</option>
            <option value="desc">descending</option>
          </select>
          <button className="btn ghost icon-only" onClick={() => emit(sort.filter((_, j) => j !== i))} aria-label="Remove sort">
            <Icon name="x" size={15} />
          </button>
        </div>
      ))}
      {sort.length < 5 && free.length > 0 && (
        <button className="btn" onClick={() => emit([...sort, { field: free[0].id, dir: "asc" }])}>
          <Icon name="plus" size={15} /> Add sort
        </button>
      )}
    </div>
  );
}
