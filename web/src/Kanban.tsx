import { useState } from "react";
import { optionTone, ValueView } from "./FieldInput";
import { Icon } from "./icons";
import type { Field, Rec } from "./types";

interface Props {
  fields: Field[];
  groupField: Field;
  records: Rec[];
  onMove: (rec: Rec, label: string | null) => void;
  onOpen: (rec: Rec) => void;
  onAdd: (label: string | null) => void;
}

/** Columns are the group-by select field's options, in the order the user arranged them, plus "No value". */
export function Kanban({ fields, groupField, records, onMove, onOpen, onAdd }: Props) {
  const [over, setOver] = useState<string | null | undefined>(undefined);
  const primary = fields.find((f) => f.is_primary) ?? fields[0];
  const extras = fields.filter((f) => f.id !== primary?.id && f.id !== groupField.id);
  const columns: { label: string | null; title: string; tone: string }[] = [
    ...[...groupField.options]
      .sort((a, b) => a.position - b.position)
      .map((o) => ({ label: o.label as string | null, title: o.label, tone: optionTone(groupField, o.label) })),
    { label: null, title: "No value", tone: "tone-7" },
  ];

  const valueOf = (r: Rec) => (r.values[groupField.name] as string | null | undefined) ?? null;

  return (
    <div className="board">
      {columns.map((col) => {
        const cards = records.filter((r) => valueOf(r) === col.label);
        return (
          <div
            key={col.label ?? "∅"}
            className={`column${over === col.label ? " over" : ""}`}
            onDragOver={(e) => {
              e.preventDefault();
              setOver(col.label);
            }}
            onDragLeave={() => setOver(undefined)}
            onDrop={(e) => {
              e.preventDefault();
              setOver(undefined);
              const rec = records.find((r) => r.id === e.dataTransfer.getData("text/plain"));
              if (rec && valueOf(rec) !== col.label) onMove(rec, col.label);
            }}
          >
            <div className={`column-head ${col.tone}`}>
              <span className="dot" />
              <span>{col.title}</span>
              <span className="count-pill">{cards.length}</span>
            </div>
            {cards.map((rec) => {
              const shown = extras.filter((f) => {
                const v = rec.values[f.name];
                return v !== null && v !== undefined && v !== "" && v !== false && !(Array.isArray(v) && v.length === 0);
              });
              return (
                <div key={rec.id} className="card-item" draggable onDragStart={(e) => e.dataTransfer.setData("text/plain", rec.id)} onClick={() => onOpen(rec)}>
                  <div className="card-title">{primary ? String(rec.values[primary.name] ?? "") || "Untitled" : "Record"}</div>
                  {shown.slice(0, 3).map((f) => (
                    <div className="card-line" key={f.id}>
                      <span className="k">{f.name}</span>
                      <ValueView field={f} value={rec.values[f.name]} />
                    </div>
                  ))}
                </div>
              );
            })}
            <button className="btn ghost add-card" onClick={() => onAdd(col.label)}>
              <Icon name="plus" size={14} /> Add record
            </button>
          </div>
        );
      })}
    </div>
  );
}
