import { useEffect, useRef, useState, type CSSProperties } from "react";
import { FieldInput, RecordPicker, ValueView } from "./FieldInput";
import { Icon, typeIcon } from "./icons";
import type { Field, Rec, Sort } from "./types";
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
  if (field.type === "relation") {
    // Opens the record list at once; the cell saves when the list closes.
    return (
      <td className="cell editing">
        <ValueView field={field} value={draft} />
        <RecordPicker
          table={field.config.table ?? ""}
          many={!!field.config.many}
          chosen={Array.isArray(draft) ? (draft as string[]) : typeof draft === "string" ? [draft] : []}
          onChange={(ids) => (field.config.many ? setDraft(ids) : finish(true, ids[0] ?? null))}
          onClose={() => finish(true)}
        />
      </td>
    );
  }
  if (field.type === "multi_select") {
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

interface HeaderMenuProps {
  field: Field;
  sorted: "asc" | "desc" | null;
  onSort: (dir: "asc" | "desc" | null) => void;
  onEdit: () => void;
  onHide: () => void;
  onDelete: () => void;
}

/** The chevron on a column header: everything you can do to that column, in one place. */
function HeaderMenu(p: HeaderMenuProps) {
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!pos) return;
    const close = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setPos(null);
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setPos(null);
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", esc);
    };
  }, [pos]);
  const sortable = p.field.type !== "multi_select" && p.field.type !== "relation";
  const run = (fn: () => void) => () => {
    setPos(null);
    fn();
  };
  return (
    <div className="th-menu" ref={ref}>
      <button
        type="button"
        className="btn ghost icon-only small th-menu-btn"
        aria-label={`${p.field.name} options`}
        aria-expanded={!!pos}
        title="Column options"
        onClick={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          setPos(pos ? null : { top: r.bottom + 4, left: Math.max(8, Math.min(r.left, window.innerWidth - 220)) });
        }}
      >
        <Icon name="chevron" size={13} />
      </button>
      {pos && (
        <div className="popover-panel menu fixed" style={pos}>
          <button className="btn ghost" onClick={run(p.onEdit)}>
            <Icon name="edit" size={15} /> Edit field
          </button>
          {sortable && (
            <>
              <button className="btn ghost" onClick={run(() => p.onSort(p.sorted === "asc" ? null : "asc"))}>
                <Icon name="sort" size={15} /> {p.sorted === "asc" ? "Remove sort" : "Sort ascending"}
              </button>
              <button className="btn ghost" onClick={run(() => p.onSort(p.sorted === "desc" ? null : "desc"))}>
                <Icon name="sort" size={15} /> {p.sorted === "desc" ? "Remove sort" : "Sort descending"}
              </button>
            </>
          )}
          {!p.field.is_primary && (
            <>
              <button className="btn ghost" onClick={run(p.onHide)}>
                <Icon name="columns" size={15} /> Hide field
              </button>
              <button className="btn ghost danger" onClick={run(p.onDelete)}>
                <Icon name="trash" size={15} /> Delete field
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}

interface GridProps {
  fields: Field[];
  records: Rec[];
  onSave: (rec: Rec, field: Field, value: unknown) => void;
  onOpen: (rec: Rec) => void;
  onAdd: () => void;
  sort: Sort[];
  onSort: (f: Field, dir: "asc" | "desc" | null) => void;
  onEditField: (f: Field) => void;
  onHideField: (f: Field) => void;
  onDeleteField: (f: Field) => void;
  onAddField: () => void;
  /** Called when a column header is dropped on another: move `from` to `to`'s place. */
  onMoveField: (from: string, to: string) => void;
  hasMore: boolean;
  onMore: () => void;
  loading: boolean;
  /** True when a search or filter is narrowing the list. */
  narrowed: boolean;
}

export function Grid(p: GridProps) {
  const [drag, setDrag] = useState<string | null>(null);
  const [over, setOver] = useState<string | null>(null);
  const first = p.loading && p.records.length === 0;
  const sortDir = (f: Field) => (p.sort[0]?.field === f.id ? p.sort[0].dir : null);
  return (
    <div className="grid-wrap">
      <table className="grid">
        <thead>
          <tr>
            <th className="rownum" />
            {p.fields.map((f) => (
              <th
                key={f.id}
                className={`${f.type === "number" ? "num" : ""}${over === f.id && drag !== f.id ? " drop-target" : ""}${drag === f.id ? " dragging" : ""}`}
                draggable={!f.is_primary}
                onDragStart={(e) => {
                  e.dataTransfer.effectAllowed = "move";
                  e.dataTransfer.setData("text/plain", f.id);
                  setDrag(f.id);
                }}
                onDragOver={(e) => {
                  if (!drag || f.is_primary) return;
                  e.preventDefault();
                  setOver(f.id);
                }}
                onDragEnd={() => {
                  setDrag(null);
                  setOver(null);
                }}
                onDrop={(e) => {
                  e.preventDefault();
                  if (drag && drag !== f.id && !f.is_primary) p.onMoveField(drag, f.id);
                  setDrag(null);
                  setOver(null);
                }}
              >
                <span className="th" title={`${f.type === "multi_select" ? "multi-select" : f.type.replace("_", " ")}${f.is_primary ? "" : " · drag to move"}`}>
                  <Icon name={typeIcon(f.type)} size={13} />
                  {f.name}
                  {sortDir(f) && <span className={`sort-mark ${sortDir(f)}`} aria-label={sortDir(f) === "asc" ? "sorted ascending" : "sorted descending"}><Icon name="chevron" size={12} /></span>}
                </span>
                <HeaderMenu
                  field={f}
                  sorted={sortDir(f)}
                  onSort={(d) => p.onSort(f, d)}
                  onEdit={() => p.onEditField(f)}
                  onHide={() => p.onHideField(f)}
                  onDelete={() => p.onDeleteField(f)}
                />
              </th>
            ))}
            <th className="add-col">
              <button type="button" className="btn ghost icon-only small" onClick={p.onAddField} title="Add field" aria-label="Add field">
                <Icon name="plus" size={15} />
              </button>
            </th>
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
                <td className="add-col" />
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
              <td className="add-col" />
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
