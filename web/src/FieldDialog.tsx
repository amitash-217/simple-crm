import { useState } from "react";
import { api, errorMessage } from "./api";
import type { Field, FieldType, Table } from "./types";
import { Icon } from "./icons";
import { ErrorText, Modal } from "./ui";

const TYPES: { id: FieldType; label: string }[] = [
  { id: "text", label: "Text" },
  { id: "number", label: "Number" },
  { id: "boolean", label: "Checkbox" },
  { id: "date", label: "Date" },
  { id: "phone", label: "Phone" },
  { id: "email", label: "Email" },
  { id: "long_text", label: "Long text" },
  { id: "select", label: "Select" },
  { id: "multi_select", label: "Multi-select" },
  { id: "relation", label: "Relation" },
];

interface OptionRow {
  id?: string;
  label: string;
}

/** Types a value can be converted between. Select converts to the others, but not back. */
const CONVERTIBLE: FieldType[] = ["text", "long_text", "email", "phone", "number", "date"];

/** Creates a field, or (with `field`) edits its name, options, settings and type. */
export function FieldDialog(props: { tableId: string; tables: Table[]; field?: Field; onClose: () => void; onSaved: () => void }) {
  const { field } = props;
  const [name, setName] = useState(field?.name ?? "");
  const [type, setType] = useState<FieldType>(field?.type ?? "text");
  const [integer, setInteger] = useState(!!field?.config.integer);
  const [target, setTarget] = useState((props.tables.find((t) => t.id !== props.tableId) ?? props.tables[0])?.id ?? props.tableId);
  const [many, setMany] = useState(true);
  const [related, setRelated] = useState(true);
  const [time, setTime] = useState(!!field?.config.time);
  const [options, setOptions] = useState<OptionRow[]>(field ? field.options.map((o) => ({ id: o.id, label: o.label })) : []);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const hasOptions = type === "select" || type === "multi_select";
  const canChangeType = !!field && (CONVERTIBLE.includes(field.type) || field.type === "select");
  const typeChanged = !!field && type !== field.type;
  const otherName = props.tables.find((t) => t.id === target)?.name ?? "the other table";
  const selfLink = target === props.tableId;
  const base = `/api/tables/${encodeURIComponent(props.tableId)}/fields`;

  const save = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError("");
    const opts = options.filter((o) => o.label.trim()).map((o) => ({ ...(o.id ? { id: o.id } : {}), label: o.label.trim() }));
    try {
      if (field) {
        const settings = type === "number" ? { config: { integer } } : type === "date" ? { config: { time } } : {};
        await api.patch(`${base}/${field.id}`, {
          name,
          ...(hasOptions ? { options: opts } : {}),
          ...(typeChanged ? { type } : {}),
          ...(type === field.type && type !== "number" && type !== "date" ? {} : settings),
        });
      } else {
        const config = type === "number" ? { integer } : type === "relation" ? (related && !selfLink ? { table: target, many, related_many: true, create_related: true } : { table: target, many, create_related: false }) : type === "date" ? { time } : {};
        await api.post(base, { name, type, config, ...(hasOptions ? { options: opts } : {}) });
      }
      props.onSaved();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  return (
    <Modal title={field ? `Edit ${field.name}` : "Add field"} onClose={props.onClose}>
      <form className="form" onSubmit={save}>
        <label>
          Name
          <input autoFocus required value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        {(!field || canChangeType) && (
          <label>
            Type
            <select value={type} onChange={(e) => {
                setType(e.target.value as FieldType);
                setError("");
              }}>
              {TYPES.filter((t) => !field || CONVERTIBLE.includes(t.id) || t.id === field.type).map((t) => (
                <option key={t.id} value={t.id}>
                  {t.label}
                </option>
              ))}
            </select>
          </label>
        )}
        {field && !canChangeType && (
          <p className="muted" style={{ margin: 0, fontSize: 12.5 }}>
            Type: {TYPES.find((t) => t.id === field.type)?.label}. Checkbox, multi-select and relation fields can&apos;t change type.
          </p>
        )}
        {typeChanged && (
          <p className="muted" style={{ margin: 0, fontSize: 12.5 }}>
            Existing values are converted. If any value doesn&apos;t fit the new type, nothing changes and you&apos;ll see which one.
            {field?.type === "select" && " The select's options are removed."}
          </p>
        )}
        {type === "number" && (
          <label className="inline">
            <input type="checkbox" checked={integer} onChange={(e) => setInteger(e.target.checked)} /> Whole numbers only
          </label>
        )}
        {type === "date" && (
          <label className="inline">
            <input type="checkbox" checked={time} onChange={(e) => setTime(e.target.checked)} /> Include time
          </label>
        )}
        {!field && type === "relation" && (
          <>
            <label>
              Links to
              <select value={target} onChange={(e) => setTarget(e.target.value)}>
                {props.tables.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
              </select>
            </label>
            {!selfLink && (
              <label className="inline">
                <input type="checkbox" checked={related} onChange={(e) => setRelated(e.target.checked)} /> Create related field in {otherName}
              </label>
            )}
            <label className="inline">
              <input type="checkbox" checked={many} onChange={(e) => setMany(e.target.checked)} /> Allow multiple relationships
            </label>
          </>
        )}
        {hasOptions && (
          <div>
            <div className="label" style={{ marginBottom: 6 }}>Options</div>
            {options.map((o, i) => (
              <div className="row" key={i}>
                <input autoFocus={!o.id && !o.label} value={o.label} placeholder="Option" onChange={(e) => setOptions(options.map((x, j) => (j === i ? { ...x, label: e.target.value } : x)))} />
                <button type="button" className="btn ghost icon-only" onClick={() => setOptions(options.filter((_, j) => j !== i))} aria-label="Remove option">
                  <Icon name="x" size={15} />
                </button>
              </div>
            ))}
            <button type="button" className="btn small" onClick={() => setOptions([...options, { label: "" }])}>
              <Icon name="plus" size={14} /> Add option
            </button>
            {field && <p className="muted" style={{ margin: "8px 0 0", fontSize: 12.5 }}>Removing an option clears it from every record that uses it.</p>}
          </div>
        )}
        <ErrorText text={error} />
        <div className="actions">
          <button type="button" className="btn" onClick={props.onClose}>
            Cancel
          </button>
          <button className="btn primary" disabled={busy}>
            {field ? "Save" : "Add field"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
