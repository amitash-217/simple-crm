import { useCallback, useEffect, useRef, useState } from "react";
import { api, errorMessage, listRecords } from "./api";
import { FieldDialog } from "./FieldDialog";
import { FilterEditor, SortEditor } from "./Filter";
import { Grid } from "./Grid";
import { Kanban } from "./Kanban";
import { clearLabelCache } from "./labels";
import { RecordModal } from "./RecordModal";
import type { Field, Rec, Table, TableDetail, View, ViewConfig } from "./types";
import { Icon, typeIcon } from "./icons";
import { ErrorText, Modal, Popover, useDialogs } from "./ui";

/** A config reduced to what the API stores, so two configs compare equal when they mean the same. */
function clean(c: ViewConfig): ViewConfig {
  const out: ViewConfig = {};
  if (c.filter) out.filter = c.filter;
  if (c.sort?.length) out.sort = c.sort;
  if (c.hidden_fields?.length) out.hidden_fields = [...c.hidden_fields].sort();
  if (c.field_order?.length) out.field_order = c.field_order;
  if (c.group_by) out.group_by = c.group_by;
  return out;
}
const same = (a: ViewConfig, b: ViewConfig) => JSON.stringify(clean(a)) === JSON.stringify(clean(b));

function conditionCount(filter: unknown): number {
  const f = filter as { and?: unknown[]; or?: unknown[] } | undefined;
  if (!f) return 0;
  return f.and?.length ?? f.or?.length ?? 1;
}

const GRID_PAGE = 100;
const BOARD_MAX = 2000;

function useRecords(tableId: string, cfg: ViewConfig, q: string, all: boolean) {
  const [records, setRecords] = useState<Rec[]>([]);
  const [next, setNext] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const seq = useRef(0);
  const key = JSON.stringify([tableId, cfg.filter, cfg.sort, q, all]);

  const reload = useCallback(async () => {
    const mine = ++seq.current;
    setLoading(true);
    setError("");
    try {
      let cursor: string | null = null;
      let acc: Rec[] = [];
      do {
        const r = await listRecords(tableId, { filter: cfg.filter, sort: cfg.sort, q, limit: all ? 200 : GRID_PAGE, cursor });
        if (mine !== seq.current) return;
        acc = acc.concat(r.records);
        cursor = r.next_cursor;
      } while (all && cursor && acc.length < BOARD_MAX);
      setRecords(acc);
      setNext(cursor);
    } catch (e) {
      if (mine === seq.current) setError(errorMessage(e));
    } finally {
      if (mine === seq.current) setLoading(false);
    }
    // `key` stands for tableId, filter, sort, q and all.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  useEffect(() => {
    reload();
  }, [reload]);

  const more = async () => {
    if (!next) return;
    const mine = seq.current;
    try {
      const r = await listRecords(tableId, { filter: cfg.filter, sort: cfg.sort, q, limit: GRID_PAGE, cursor: next });
      if (mine !== seq.current) return;
      setRecords((rs) => rs.concat(r.records));
      setNext(r.next_cursor);
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  return { records, setRecords, next, loading, error, reload, more };
}

interface Props {
  tableId: string;
  viewId: string | null;
  tables: Table[];
  navigate: (hash: string) => void;
  onTablesChanged: () => void;
}

export function TablePage({ tableId, viewId, tables, navigate, onTablesChanged }: Props) {
  const [table, setTable] = useState<TableDetail | null>(null);
  const [views, setViews] = useState<View[]>([]);
  const [error, setError] = useState("");
  const [cfg, setCfg] = useState<ViewConfig>({});
  const [q, setQ] = useState("");
  const [dq, setDq] = useState("");
  const [modal, setModal] = useState<{ rec: Rec | null; initial?: Record<string, unknown> } | null>(null);
  const [fieldDlg, setFieldDlg] = useState<{ field?: Field } | null>(null);
  const [viewDlg, setViewDlg] = useState(false);

  const { confirm, prompt, toast } = useDialogs();
  const base = `/api/tables/${encodeURIComponent(tableId)}`;
  const view = views.find((v) => v.id === viewId) ?? null;
  const isKanban = view?.kind === "kanban";
  const rows = useRecords(tableId, cfg, dq, isKanban);

  const load = useCallback(async () => {
    try {
      const [t, v] = await Promise.all([api.get<TableDetail>(base), api.get<View[]>(`${base}/views`)]);
      setTable(t);
      setViews(v);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [base]);

  useEffect(() => {
    load();
  }, [load]);
  useEffect(() => {
    const t = setTimeout(() => setDq(q), 250);
    return () => clearTimeout(t);
  }, [q]);

  // Switching views, or saving one, resets the working config to what is stored. A stale view
  // drops its filter and sort, which would otherwise make every list request fail.
  const viewKey = view ? `${view.id}${JSON.stringify(view.config)}${view.stale}` : "all";
  useEffect(() => {
    if (!view) setCfg({});
    else setCfg(view.stale ? { hidden_fields: view.config.hidden_fields, field_order: view.config.field_order, group_by: view.config.group_by } : view.config);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [viewKey]);

  const schemaChanged = async () => {
    clearLabelCache();
    await load();
    rows.reload();
  };

  if (!table) return <div className="main">{error ? <ErrorText text={error} /> : <p className="muted pad">Loading…</p>}</div>;

  // The view's order first, then any field it does not list, in the table's order. The primary field stays first.
  const listed = cfg.field_order ?? [];
  const byId = new Map(table.fields.map((f) => [f.id, f]));
  const fields = [
    ...table.fields.filter((f) => f.is_primary),
    ...listed.map((id) => byId.get(id)).filter((f): f is Field => !!f && !f.is_primary),
    ...table.fields.filter((f) => !f.is_primary && !listed.includes(f.id)),
  ];
  const hidden = cfg.hidden_fields ?? [];
  const visible = fields.filter((f) => !hidden.includes(f.id));
  const selectFields = fields.filter((f) => f.type === "select");
  const groupField = selectFields.find((f) => f.id === cfg.group_by);
  const dirty = view ? !same(cfg, view.config) : !same(cfg, {});
  const go = (id: string | null) => navigate(id ? `#/t/${tableId}/v/${id}` : `#/t/${tableId}`);

  const run = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (e) {
      toast(errorMessage(e));
    }
  };

  const saveCell = (rec: Rec, field: Field, value: unknown) =>
    run(async () => {
      const r = await api.patch<Rec>(`${base}/records/${rec.id}`, { values: { [field.id]: value } });
      rows.setRecords((rs) => rs.map((x) => (x.id === r.id ? r : x)));
      if (field.is_primary) clearLabelCache();
      // Linking a record can take it from another row, so refetch the rows.
      if (field.type === "relation") rows.reload();
    });

  const moveCard = (rec: Rec, label: string | null) => {
    if (!groupField) return;
    rows.setRecords((rs) => rs.map((x) => (x.id === rec.id ? { ...x, values: { ...x.values, [groupField.name]: label } } : x)));
    run(async () => {
      try {
        await api.patch(`${base}/records/${rec.id}`, { values: { [groupField.id]: label } });
      } catch (e) {
        rows.reload();
        throw e;
      }
    });
  };

  const moveField = (from: string, to: string) => {
    const ids = fields.map((f) => f.id).filter((id) => id !== from);
    ids.splice(ids.indexOf(to) + (fields.findIndex((f) => f.id === from) < fields.findIndex((f) => f.id === to) ? 1 : 0), 0, from);
    setCfg((c) => ({ ...c, field_order: ids }));
  };

  const toggleHidden = (f: Field) => setCfg({ ...cfg, hidden_fields: hidden.includes(f.id) ? hidden.filter((h) => h !== f.id) : [...hidden, f.id] });

  const saveView = () => view && run(async () => {
    await api.patch(`${base}/views/${view.id}`, { config: clean(cfg) });
    await load();
  });

  const renameView = () =>
    view &&
    run(async () => {
      const name = await prompt({ title: "Rename view", label: "View name", value: view.name });
      if (name && name.trim() && name !== view.name) {
        await api.patch(`${base}/views/${view.id}`, { name });
        await load();
      }
    });

  const deleteView = () =>
    view &&
    run(async () => {
      const ok = await confirm({ title: `Delete the view “${view.name}”?`, message: "Records are not affected. Only the saved filter, sort and layout go away.", confirmLabel: "Delete view", danger: true });
      if (!ok) return;
      await api.del(`${base}/views/${view.id}`);
      go(null);
      await load();
    });

  const renameTable = () =>
    run(async () => {
      const name = await prompt({ title: "Rename table", label: "Table name", value: table.name });
      if (name && name.trim() && name !== table.name) {
        await api.patch(base, { name });
        clearLabelCache();
        onTablesChanged();
        await load();
      }
    });

  const deleteTable = () =>
    run(async () => {
      const ok = await confirm({ title: `Delete the table “${table.name}”?`, message: "Its records and views become unavailable.", confirmLabel: "Delete table", danger: true });
      if (!ok) return;
      await api.del(base);
      clearLabelCache();
      navigate("#/");
      onTablesChanged();
    });

  const deleteField = (f: Field) =>
    run(async () => {
      const ok = await confirm({ title: `Delete the field “${f.name}”?`, message: f.type === "relation" && f.config.related ? "Its values are no longer shown, and the related field in the linked table is deleted too. Views that use them will need updating." : "Its values are no longer shown. Views that use it will need updating.", confirmLabel: "Delete field", danger: true });
      if (!ok) return;
      await api.del(`${base}/fields/${f.id}`);
      await schemaChanged();
    });

  const filterCount = conditionCount(cfg.filter);
  const narrowed = filterCount > 0 || dq.trim() !== "";
  const nRecords = rows.records.length;

  return (
    <div className="main">
      <div className="head">
        <span className="tbl-ico">
          <Icon name="table" size={17} />
        </span>
        <div>
          <h1>{table.name}</h1>
          <div className="subtitle">
            {fields.length} {fields.length === 1 ? "field" : "fields"}
            {!rows.loading && ` · ${nRecords}${rows.next ? "+" : ""} ${nRecords === 1 && !rows.next ? "record" : "records"}`}
          </div>
        </div>
        <span className="spacer" />
        <Popover icon="more" title="Table options" align="right">
          <div className="menu">
            <button className="btn ghost" onClick={renameTable}>
              <Icon name="edit" size={15} /> Rename table
            </button>
            <button className="btn ghost danger" onClick={deleteTable}>
              <Icon name="trash" size={15} /> Delete table
            </button>
          </div>
        </Popover>
      </div>

      <div className="tabs" role="tablist">
        <button className={`tab${!view ? " on" : ""}`} role="tab" aria-selected={!view} onClick={() => go(null)}>
          <Icon name="list" size={14} /> All records
        </button>
        {views.map((v) => (
          <button key={v.id} className={`tab${view?.id === v.id ? " on" : ""}`} role="tab" aria-selected={view?.id === v.id} onClick={() => go(v.id)}>
            <Icon name={v.kind === "kanban" ? "board" : "table"} size={14} />
            {v.name}
            {v.stale && (
              <span title="Uses a deleted field" className="danger">
                <Icon name="alert" size={13} />
              </span>
            )}
          </button>
        ))}
        <button className="tab add" onClick={() => setViewDlg(true)}>
          <Icon name="plus" size={14} /> View
        </button>
        {view && (
          <span className="tab-actions">
            <button className="btn ghost small" onClick={renameView}>
              <Icon name="edit" size={13} /> Rename
            </button>
            <button className="btn ghost small danger" onClick={deleteView}>
              <Icon name="trash" size={13} /> Delete
            </button>
          </span>
        )}
      </div>

      <div className="toolbar">
        <div className="search-box">
          <Icon name="search" size={15} />
          <input type="search" aria-label="Search records" placeholder="Search records…" value={q} onChange={(e) => setQ(e.target.value)} />
        </div>
        <Popover icon="filter" label="Filter" count={filterCount} active={filterCount > 0}>
          <FilterEditor fields={fields} filter={cfg.filter} onChange={(f) => setCfg((c) => ({ ...c, filter: f }))} />
        </Popover>
        <Popover icon="sort" label="Sort" count={cfg.sort?.length ?? 0} active={!!cfg.sort?.length}>
          <SortEditor fields={fields} sort={cfg.sort ?? []} onChange={(s) => setCfg((c) => ({ ...c, sort: s }))} />
        </Popover>
        <Popover icon="columns" label="Fields" count={hidden.length} active={hidden.length > 0}>
          <div className="panel-body">
            <div className="panel-title">Show, edit or remove fields</div>
            {fields.map((f) => (
              <div className="row" key={f.id}>
                <label className="inline grow">
                  <input type="checkbox" checked={!hidden.includes(f.id)} disabled={f.is_primary} onChange={() => toggleHidden(f)} />
                  <Icon name={typeIcon(f.type)} size={14} />
                  {f.name}
                </label>
                <button className="btn ghost icon-only small" onClick={() => setFieldDlg({ field: f })} title={`Edit ${f.name}`} aria-label={`Edit ${f.name}`}>
                  <Icon name="edit" size={14} />
                </button>
                {!f.is_primary && (
                  <button className="btn ghost icon-only small danger" onClick={() => deleteField(f)} title={`Delete ${f.name}`} aria-label={`Delete ${f.name}`}>
                    <Icon name="trash" size={14} />
                  </button>
                )}
              </div>
            ))}
            <button className="btn" onClick={() => setFieldDlg({})}>
              <Icon name="plus" size={15} /> Add field
            </button>
          </div>
        </Popover>
        <span className="spacer" />
        {dirty && view && (
          <>
            <button className="btn" onClick={() => setCfg(view.config)}>
              Reset
            </button>
            <button className="btn primary" onClick={saveView}>
              Save view
            </button>
          </>
        )}
        {dirty && !view && (
          <button className="btn primary" onClick={() => setViewDlg(true)}>
            <Icon name="plus" size={15} /> Save as view
          </button>
        )}
      </div>

      {view?.stale && (
        <div className="banner warn">
          <Icon name="alert" size={15} />
          This view used a field or option that was deleted, so its filter and sort are switched off. Set them again and save to fix the view.
        </div>
      )}
      {(error || rows.error) && <ErrorText text={error || rows.error} />}

      {isKanban && !groupField && (
        <div className="banner">
          Choose the select field that makes the columns:
          <select value="" onChange={(e) => setCfg({ ...cfg, group_by: e.target.value })}>
            <option value="" />
            {selectFields.map((f) => (
              <option key={f.id} value={f.id}>
                {f.name}
              </option>
            ))}
          </select>
          {selectFields.length === 0 && <span className="muted">This table has no select fields yet. Add one from Fields.</span>}
        </div>
      )}
      {isKanban && groupField ? (
        <>
          <Kanban
            fields={visible}
            groupField={groupField}
            records={rows.records}
            onMove={moveCard}
            onOpen={(rec) => setModal({ rec })}
            onAdd={(label) => setModal({ rec: null, initial: { [groupField.id]: label } })}
          />
          {rows.next && <p className="muted pad">Showing the first {BOARD_MAX} records. Narrow the filter to see others.</p>}
        </>
      ) : (
        !isKanban && (
          <Grid
            fields={visible}
            records={rows.records}
            onSave={saveCell}
            onOpen={(rec) => setModal({ rec })}
            onAdd={() => setModal({ rec: null })}
            onMoveField={moveField}
            sort={cfg.sort ?? []}
            onSort={(f, dir) => setCfg((c) => ({ ...c, sort: dir ? [{ field: f.id, dir }] : (c.sort ?? []).filter((s) => s.field !== f.id) }))}
            onEditField={(f) => setFieldDlg({ field: f })}
            onHideField={toggleHidden}
            onDeleteField={deleteField}
            onAddField={() => setFieldDlg({})}
            hasMore={!!rows.next}
            onMore={rows.more}
            loading={rows.loading}
            narrowed={narrowed}
          />
        )
      )}

      {modal && (
        <RecordModal
          key={modal.rec?.id ?? "new"}
          table={table}
          record={modal.rec}
          initial={modal.initial}
          onClose={() => setModal(null)}
          onSaved={() => {
            setModal(null);
            rows.reload();
          }}
          onDeleted={() => {
            setModal(null);
            rows.reload();
          }}
        />
      )}
      {fieldDlg && (
        <FieldDialog
          tableId={tableId}
          tables={tables}
          field={fieldDlg.field}
          onClose={() => setFieldDlg(null)}
          onSaved={() => {
            setFieldDlg(null);
            schemaChanged();
          }}
        />
      )}
      {viewDlg && (
        <NewViewDialog
          selectFields={selectFields}
          onClose={() => setViewDlg(false)}
          onCreate={async (name, kind, groupBy) => {
            const config = clean({ ...cfg, group_by: kind === "kanban" ? groupBy : undefined });
            const v = await api.post<View>(`${base}/views`, { name, kind, config });
            setViewDlg(false);
            await load();
            go(v.id);
          }}
        />
      )}
    </div>
  );
}

function NewViewDialog(props: {
  selectFields: Field[];
  onClose: () => void;
  onCreate: (name: string, kind: "grid" | "kanban", groupBy: string) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [kind, setKind] = useState<"grid" | "kanban">("grid");
  const [groupBy, setGroupBy] = useState(props.selectFields[0]?.id ?? "");
  const [error, setError] = useState("");
  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await props.onCreate(name, kind, groupBy);
    } catch (err) {
      setError(errorMessage(err));
    }
  };
  return (
    <Modal title="New view" onClose={props.onClose}>
      <form className="form" onSubmit={submit}>
        <p className="muted">The view starts with the filter, sort and hidden fields you have set now.</p>
        <label>
          Name
          <input autoFocus required value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label>
          Layout
          <select value={kind} onChange={(e) => setKind(e.target.value as "grid" | "kanban")}>
            <option value="grid">Grid</option>
            <option value="kanban">Kanban board</option>
          </select>
        </label>
        {kind === "kanban" &&
          (props.selectFields.length ? (
            <label>
              Columns from
              <select value={groupBy} onChange={(e) => setGroupBy(e.target.value)}>
                {props.selectFields.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.name}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <div className="error">Add a select field first. A board needs one for its columns.</div>
          ))}
        <ErrorText text={error} />
        <div className="actions">
          <button type="button" className="btn" onClick={props.onClose}>
            Cancel
          </button>
          <button className="btn primary" disabled={kind === "kanban" && !groupBy}>
            Create view
          </button>
        </div>
      </form>
    </Modal>
  );
}
