import type { ReactNode } from "react";
import type { FieldType } from "./types";

// 24x24 stroke icons, drawn in the Lucide style.
const PATHS = {
  table: (
    <>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M3 9h18M3 15h18M9 3v18" />
    </>
  ),
  plus: <path d="M12 5v14M5 12h14" />,
  search: (
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="m21 21-4.3-4.3" />
    </>
  ),
  filter: <path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3Z" />,
  sort: <path d="m3 16 4 4 4-4M7 20V4M21 8l-4-4-4 4M17 4v16" />,
  columns: (
    <>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M9 3v18M15 3v18" />
    </>
  ),
  chevron: <path d="m6 9 6 6 6-6" />,
  x: <path d="M18 6 6 18M6 6l12 12" />,
  expand: <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" />,
  trash: <path d="M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6M10 11v5M14 11v5" />,
  edit: <path d="M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />,
  more: (
    <>
      <circle cx="5" cy="12" r="1" />
      <circle cx="12" cy="12" r="1" />
      <circle cx="19" cy="12" r="1" />
    </>
  ),
  list: <path d="M3 6h18M3 12h18M3 18h18" />,
  board: (
    <>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M9 7v6M15 7v10" />
    </>
  ),
  settings: <path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6" />,
  logout: <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9" />,
  key: <path d="m21 2-2 2m-7.6 7.6a5.5 5.5 0 1 1-7.8 7.8 5.5 5.5 0 0 1 7.8-7.8Zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3m-3.5 3.5L19 4" />,
  users: (
    <>
      <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M22 21v-2a4 4 0 0 0-3-3.9M16 3.1a4 4 0 0 1 0 7.8" />
      <circle cx="9" cy="7" r="4" />
    </>
  ),
  check: <path d="M20 6 9 17l-5-5" />,
  alert: <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0ZM12 9v4M12 17h.01" />,
  inbox: <path d="M22 12h-6l-2 3h-4l-2-3H2M5.5 5.1 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.5-6.9A2 2 0 0 0 16.7 4H7.3a2 2 0 0 0-1.8 1.1Z" />,
  text: <path d="M4 7V4h16v3M9 20h6M12 4v16" />,
  number: <path d="M4 9h16M4 15h16M10 3 8 21M16 3l-2 18" />,
  boolean: <path d="m9 11 3 3L22 4M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" />,
  date: (
    <>
      <rect x="3" y="4" width="18" height="18" rx="2" />
      <path d="M16 2v4M8 2v4M3 10h18" />
    </>
  ),
  phone: (
    <path d="M22 16.9v3a2 2 0 0 1-2.2 2 19.8 19.8 0 0 1-8.6-3.1 19.5 19.5 0 0 1-6-6A19.8 19.8 0 0 1 2.1 4.2 2 2 0 0 1 4.1 2h3a2 2 0 0 1 2 1.7c.1 1 .4 1.9.7 2.8a2 2 0 0 1-.5 2.1L8.1 9.9a16 16 0 0 0 6 6l1.3-1.3a2 2 0 0 1 2.1-.4c.9.3 1.8.6 2.8.7a2 2 0 0 1 1.7 2Z" />
  ),
  select: (
    <>
      <circle cx="12" cy="12" r="10" />
      <path d="m8 10 4 4 4-4" />
    </>
  ),
  multi: <path d="m3 6 2 2 3-3M3 14l2 2 3-3M12 7h9M12 15h9" />,
  link: <path d="M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7" />,
} satisfies Record<string, ReactNode>;

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg
      className="icon"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {PATHS[name]}
    </svg>
  );
}

const TYPE_ICONS: Record<FieldType, IconName> = {
  text: "text",
  number: "number",
  boolean: "boolean",
  date: "date",
  phone: "phone",
  select: "select",
  multi_select: "multi",
  relation: "link",
};
export const typeIcon = (t: FieldType): IconName => TYPE_ICONS[t];

/** The small mark used by the sidebar and the sign-in page. */
export function Logo({ size = 28 }: { size?: number }) {
  return (
    <span className="logo" style={{ width: size, height: size }} aria-hidden="true">
      <svg width={size * 0.58} height={size * 0.58} viewBox="0 0 24 24" fill="none" stroke="#0c0c0e" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
        <rect x="3" y="3" width="7.5" height="7.5" rx="1.8" />
        <rect x="13.5" y="3" width="7.5" height="7.5" rx="1.8" />
        <rect x="3" y="13.5" width="7.5" height="7.5" rx="1.8" />
        <path d="M17.2 13.5v7.5M13.5 17.2H21" />
      </svg>
    </span>
  );
}
