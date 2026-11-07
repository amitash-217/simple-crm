// Shows a related record by its primary field's value. Lookups are cached per page session.
import { useEffect, useState } from "react";
import { api } from "./api";
import type { Rec, TableDetail } from "./types";

const tables = new Map<string, Promise<TableDetail>>();
const labels = new Map<string, Promise<string>>();

export function tableDetail(id: string): Promise<TableDetail> {
  let p = tables.get(id);
  if (!p) {
    p = api.get<TableDetail>(`/api/tables/${encodeURIComponent(id)}`);
    p.catch(() => tables.delete(id));
    tables.set(id, p);
  }
  return p;
}

/** Call after anything that may change a name or a record, so later lookups read fresh data. */
export function clearLabelCache() {
  tables.clear();
  labels.clear();
}

export async function labelOf(tableId: string, rec: Rec): Promise<string> {
  const primary = (await tableDetail(tableId)).fields.find((f) => f.is_primary);
  const v = primary ? rec.values[primary.name] : null;
  return v === null || v === undefined || v === "" ? "(untitled)" : String(v);
}

export function recordLabel(tableId: string, id: string): Promise<string> {
  const key = `${tableId}/${id}`;
  let p = labels.get(key);
  if (!p) {
    p = api
      .get<Rec>(`/api/tables/${encodeURIComponent(tableId)}/records/${encodeURIComponent(id)}`)
      .then((r) => labelOf(tableId, r))
      .catch(() => "(deleted)");
    labels.set(key, p);
  }
  return p;
}

export function useRecordLabel(tableId: string, id: string): string {
  const [label, setLabel] = useState("…");
  useEffect(() => {
    let live = true;
    recordLabel(tableId, id).then((l) => live && setLabel(l));
    return () => {
      live = false;
    };
  }, [tableId, id]);
  return label;
}
