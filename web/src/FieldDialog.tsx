import { useState } from "react";
import { api, errorMessage } from "./api";
import type { Field, FieldType, Table } from "./types";
import { Icon } from "./icons";
import { ErrorText, Modal } from "./ui";
import { Select } from "./Pickers";
import { TONE_NAMES, toneIndex } from "./FieldInput";

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
  color: string | null;
}

/** Types a value can be converted between. Select converts to the others, but not back. */
const CONVERTIBLE: FieldType[] = ["text", "long_text", "email", "phone", "number", "date", "boolean"];

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
  const [options, setOptions] = useState<OptionRow[]>(field ? field.options.map((o) => ({ id: o.id, label: o.label, color: o.color })) : []);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  // The option whose colour swatches are showing.
  const [picking, setPicking] = useState<number | null>(null);
  // A new option takes the first colour not in use yet.
  const nextColor = () => TONE_NAMES.slice(0, 7).find((c) => !options.some((o, i) => toneIndex(o.color, i) === TONE_NAMES.indexOf(c))) ?? TONE_NAMES[options.length % 7];

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
    const opts = options.filter((o) => o.label.trim()).map((o) => ({ ...(o.id ? { id: o.id } : {}), label: o.label.trim(), color: o.color }));
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
            <Select value={type} onChange={(e) => {
                setType(e.target.value as FieldType);
                setError("");
              }}>
              {TYPES.filter((t) => !field || CONVERTIBLE.includes(t.id) || t.id === field.type).map((t) => (
                <option key={t.id} value={t.id}>
                  {t.label}
                </option>
              ))}
            </Select>
          </label>
        )}
        {field && !canChangeType && (
          <p className="muted" style={{ margin: 0, fontSize: 12 }}>
            Type: {TYPES.find((t) => t.id === field.type)?.label}. Multi-select and relation fields can&apos;t change type.
          </p>
        )}
        {typeChanged && (
          <p className="muted" style={{ margin: 0, fontSize: 12 }}>
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
              <Select value={target} onChange={(e) => setTarget(e.target.value)}>
                {props.tables.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
              </Select>
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
          <div className="opt-list">
            <div className="label">Options</div>
            {options.map((o, i) => (
              <div className="opt-row" key={i}>
                <div className="row">
                  <button
                    type="button"
                    className={`swatch tone-${toneIndex(o.color, i)}`}
                    onClick={() => setPicking(picking === i ? null : i)}
                    aria-label={`Colour of ${o.label || "option"}`}
                    aria-expanded={picking === i}
                  />
                  <input autoFocus={!o.id && !o.label} value={o.label} placeholder="Option" onChange={(e) => setOptions(options.map((x, j) => (j === i ? { ...x, label: e.target.value } : x)))} />
                  <button type="button" className="btn ghost icon-only" onClick={() => setOptions(options.filter((_, j) => j !== i))} aria-label="Remove option">
                    <Icon name="x" size={16} />
                  </button>
                </div>
                {picking === i && (
                  <div className="swatches" role="group" aria-label="Option colour">
                    {TONE_NAMES.map((c, t) => (
                      <button
                        type="button"
                        key={c}
                        className={`swatch tone-${t}${toneIndex(o.color, i) === t ? " on" : ""}`}
                        aria-label={c}
                        title={c}
                        onClick={() => {
                          setOptions(options.map((x, j) => (j === i ? { ...x, color: c } : x)));
                          setPicking(null);
                        }}
                      />
                    ))}
                  </div>
                )}
              </div>
            ))}
            <button type="button" className="btn small opt-add" onClick={() => setOptions([...options, { label: "", color: nextColor() }])}>
              <Icon name="plus" size={16} /> Add option
            </button>
            {field && <p className="muted" style={{ margin: "8px 0 0", fontSize: 12 }}>Removing an option clears it from every record that uses it</p>}
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
