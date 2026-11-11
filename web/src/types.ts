export type FieldType = "text" | "number" | "boolean" | "date" | "phone" | "email" | "long_text" | "select" | "multi_select" | "relation";

export interface Option {
  id: string;
  label: string;
  color: string | null;
  position: number;
}

export interface Field {
  id: string;
  name: string;
  type: FieldType;
  is_primary: boolean;
  position: number;
  // number: { integer }, relation: { table, many }, date: { time }
  config: { integer?: boolean; table?: string; many?: boolean; time?: boolean; related?: string; exclusive?: boolean };
  options: Option[];
}

export interface Table {
  id: string;
  name: string;
  created_at: string;
}

export interface TableDetail extends Table {
  fields: Field[];
}

export interface Rec {
  id: string;
  created_at: string;
  updated_at: string;
  created_by: string | null;
  updated_by: string | null;
  // Keyed by field name.
  values: Record<string, unknown>;
}

export interface Sort {
  field: string;
  dir: "asc" | "desc";
}

/** The shape the API stores. The UI works with field IDs and option IDs throughout. */
export interface ViewConfig {
  filter?: unknown;
  sort?: Sort[];
  hidden_fields?: string[];
  /** Field IDs in display order. Fields not listed follow in their default order. */
  field_order?: string[];
  group_by?: string;
}

export interface View {
  id: string;
  name: string;
  kind: "grid" | "kanban";
  config: ViewConfig;
  position: number;
  stale: boolean;
}

export interface User {
  id: string;
  email: string;
  is_admin: boolean;
}
