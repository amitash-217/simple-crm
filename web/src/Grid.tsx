import { useRef, useState, type CSSProperties } from "react";
import { FieldInput, ValueView } from "./FieldInput";
import { Icon, typeIcon } from "./icons";
import type { Field, Rec } from "./types";
import { EmptyState } from "./ui";

interface CellProps {
  field: Field;
  value: unknown;
  onSave: (v: unknown) => void;
}

function Cell({ field, value, onSave }: CellProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<unknown>(null);
  // Enter or Escape closes the editor, and the blur that follows must not commit a second time.
  const settled = useRef(false);
  // The popup for multi-select and relation cells is fixed to the viewport, so the grid's scroll box cannot clip it.
  const [popStyle, setPopStyle] = useState<CSSProperties>({});

  const start = (td: HTMLElement) => {
    const r = td.getBoundingClientRect();
    const top = Math.max(8, Math.min(r.top, window.innerHeight - 240));
    setPopStyle({ top, left: Math.max(8, Math.min(r.left, window.innerWidth - 300)), maxHeight: window.innerHeight - top - 8 });
    settled.current = false;
    setDraft(value);
    setEditing(true);
  };
  const finish = (save: boolean, v: unknown = draft) => {
    if (settled.current) return;
    settled.current = true;
    setEditing(false);
    if (save && JSON.stringify(v) !== JSON.stringify(value)) onSave(v);
  };

  if (field.type === "boolean") {
    return (
      <td className="cell center-cell">
        <input type="checkbox" checked={value === true} onChange={(e) => onSave(e.target.checked)} />
      </td>
    );
  }
  if (!editing) {
    return (
      <td className={`cell${field.type === "number" ? " num" : ""}`} onClick={(e) => start(e.currentTarget)}>
        <ValueView field={field} value={value} />
      </td>
    );
  }
  if (field.type === "multi_select" || field.type === "relation") {
    return (
      <td className="cell editing">
        <div className="cell-pop" style={popStyle}>
          <FieldInput field={field} value={draft} onChange={setDraft} />
          <div className="actions">
            <button className="btn" onClick={() => finish(false)}>
              Cancel
            </button>
            <button className="btn primary" onClick={() => finish(true)}>
              Done
            </button>
          </div>
        </div>
      </td>
    );
  }
  if (field.type === "select") {
    return (
      <td className="cell editing">
        <FieldInput autoFocus field={field} value={draft} onChange={(v) => finish(true, v)} onEscape={() => finish(false)} onBlur={() => finish(false)} />
      </td>
    );
  }
  return (
    <td className="cell editing">
      <FieldInput autoFocus field={field} value={draft} onChange={setDraft} onEnter={() => finish(true)} onEscape={() => finish(false)} onBlur={() => finish(true)} />
    </td>
  );
}

interface GridProps {
  fields: Field[];
  records: Rec[];
  onSave: (rec: Rec, field: Field, value: unknown) => void;
  onOpen: (rec: Rec) => void;
  onAdd: () => void;
  hasMore: boolean;
  onMore: () => void;
  loading: boolean;
  /** True when a search or filter is narrowing the list. */
  narrowed: boolean;
}

export function Grid(p: GridProps) {
  const first = p.loading && p.records.length === 0;
  return (
    <div className="grid-wrap">
      <table className="grid">
        <thead>
          <tr>
            <th className="rownum" />
            {p.fields.map((f) => (
              <th key={f.id} className={f.type === "number" ? "num" : undefined}>
                <span className="th" title={f.type === "multi_select" ? "multi-select" : f.type}>
                  <Icon name={typeIcon(f.type)} size={13} />
                  {f.name}
                </span>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {first &&
            [0, 1, 2, 3, 4].map((i) => (
              <tr key={i}>
                <td className="rownum" />
                {p.fields.map((f, j) => (
                  <td key={f.id} className="cell">
                    <span className="skeleton" style={{ width: `${45 + ((i * 17 + j * 23) % 45)}%` }} />
                  </td>
                ))}
              </tr>
            ))}
          {p.records.map((rec, i) => (
            <tr key={rec.id}>
              <td className="rownum">
                <span className="n">{i + 1}</span>
                <button className="btn ghost icon-only small open" onClick={() => p.onOpen(rec)} title="Open record" aria-label="Open record">
                  <Icon name="expand" size={14} />
                </button>
              </td>
              {p.fields.map((f) => (
                <Cell key={f.id} field={f} value={rec.values[f.name] ?? null} onSave={(v) => p.onSave(rec, f, v)} />
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {p.records.length === 0 && !p.loading && (
        <EmptyState
          icon={p.narrowed ? "search" : "inbox"}
          title={p.narrowed ? "No records match" : "No records yet"}
          action={
            !p.narrowed && (
              <button className="btn primary" onClick={p.onAdd}>
                <Icon name="plus" size={15} /> New record
              </button>
            )
          }
        >
          {p.narrowed ? "Try a different search, or loosen the filter." : "Add the first record to this table."}
        </EmptyState>
      )}
      <div className="grid-foot">
        <button className="btn small" onClick={p.onAdd}>
          <Icon name="plus" size={14} /> New record
        </button>
        {p.hasMore && (
          <button className="btn small" onClick={p.onMore}>
            Load more
          </button>
        )}
        <span className="muted spacer" />
        <span className="muted">
          {p.records.length}
          {p.hasMore ? "+" : ""} {p.records.length === 1 && !p.hasMore ? "record" : "records"}
        </span>
      </div>
    </div>
  );
}
