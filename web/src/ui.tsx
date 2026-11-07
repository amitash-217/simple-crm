import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { Icon, type IconName } from "./icons";

/** A button that opens a dropdown panel. The panel closes on a click outside it or on Escape. */
export function Popover(props: {
  label?: ReactNode;
  icon?: IconName;
  count?: number;
  active?: boolean;
  children: ReactNode;
  align?: "left" | "right";
  title?: string;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", esc);
    };
  }, [open]);
  return (
    <div className="popover" ref={ref}>
      <button
        className={`btn tool${props.active ? " active" : ""}${props.label ? "" : " icon-only"}`}
        onClick={() => setOpen(!open)}
        title={props.title}
        aria-label={props.title}
        aria-expanded={open}
      >
        {props.icon && <Icon name={props.icon} size={15} />}
        {props.label}
        {!!props.count && <span className="count">{props.count}</span>}
      </button>
      {open && <div className={`popover-panel ${props.align === "right" ? "right" : ""}`}>{props.children}</div>}
    </div>
  );
}

export function Modal(props: { title: string; onClose: () => void; children: ReactNode; wide?: boolean; small?: boolean }) {
  useEffect(() => {
    const esc = (e: KeyboardEvent) => e.key === "Escape" && props.onClose();
    document.addEventListener("keydown", esc);
    return () => document.removeEventListener("keydown", esc);
  }, [props.onClose]);
  return (
    <div className="backdrop" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div className={`modal${props.wide ? " wide" : ""}${props.small ? " small" : ""}`} role="dialog" aria-label={props.title}>
        <div className="modal-head">
          <h2>{props.title}</h2>
          <button className="btn icon-only ghost" onClick={props.onClose} aria-label="Close">
            <Icon name="x" size={16} />
          </button>
        </div>
        {props.children}
      </div>
    </div>
  );
}

export const ErrorText = ({ text }: { text: string }) =>
  text ? (
    <div className="error" role="alert">
      <Icon name="alert" size={14} /> {text}
    </div>
  ) : null;

export function EmptyState(props: { icon: IconName; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="empty-state">
      <span className="empty-icon">
        <Icon name={props.icon} size={22} />
      </span>
      <h3>{props.title}</h3>
      {props.children && <p>{props.children}</p>}
      {props.action}
    </div>
  );
}

export const initials = (email: string) => (email.trim()[0] ?? "?").toUpperCase();

// In-app replacements for window.confirm and window.prompt, plus toasts for errors from background actions.
interface ConfirmOpts {
  title: string;
  message?: ReactNode;
  confirmLabel?: string;
  danger?: boolean;
}
interface PromptOpts {
  title: string;
  label: string;
  value?: string;
  confirmLabel?: string;
  password?: boolean;
}
interface Toast {
  id: number;
  text: string;
  kind: "error" | "info";
}
type Pending =
  | { kind: "confirm"; opts: ConfirmOpts; resolve: (v: boolean) => void }
  | { kind: "prompt"; opts: PromptOpts; resolve: (v: string | null) => void };

interface DialogApi {
  confirm: (o: ConfirmOpts) => Promise<boolean>;
  prompt: (o: PromptOpts) => Promise<string | null>;
  toast: (text: string, kind?: "error" | "info") => void;
}
const DialogContext = createContext<DialogApi | null>(null);

export function useDialogs(): DialogApi {
  const api = useContext(DialogContext);
  if (!api) throw new Error("useDialogs needs a DialogProvider");
  return api;
}

export function DialogProvider({ children }: { children: ReactNode }) {
  const [pending, setPending] = useState<Pending | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextId = useRef(1);

  const confirm = useCallback((opts: ConfirmOpts) => new Promise<boolean>((resolve) => setPending({ kind: "confirm", opts, resolve })), []);
  const prompt = useCallback((opts: PromptOpts) => new Promise<string | null>((resolve) => setPending({ kind: "prompt", opts, resolve })), []);
  const toast = useCallback((text: string, kind: "error" | "info" = "error") => {
    const id = nextId.current++;
    setToasts((t) => [...t, { id, text, kind }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 5000);
  }, []);

  const api = useRef<DialogApi>({ confirm, prompt, toast });
  return (
    <DialogContext.Provider value={api.current}>
      {children}
      {pending?.kind === "confirm" && (
        <Modal
          small
          title={pending.opts.title}
          onClose={() => {
            pending.resolve(false);
            setPending(null);
          }}
        >
          {pending.opts.message && <p className="dialog-text">{pending.opts.message}</p>}
          <div className="actions">
            <button
              className="btn"
              onClick={() => {
                pending.resolve(false);
                setPending(null);
              }}
            >
              Cancel
            </button>
            <button
              className={`btn ${pending.opts.danger ? "danger-solid" : "primary"}`}
              autoFocus
              onClick={() => {
                pending.resolve(true);
                setPending(null);
              }}
            >
              {pending.opts.confirmLabel ?? "Confirm"}
            </button>
          </div>
        </Modal>
      )}
      {pending?.kind === "prompt" && (
        <PromptDialog
          opts={pending.opts}
          onDone={(v) => {
            pending.resolve(v);
            setPending(null);
          }}
        />
      )}
      <div className="toasts" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={`toast ${t.kind}`}>
            <Icon name={t.kind === "error" ? "alert" : "check"} size={15} />
            <span>{t.text}</span>
            <button className="btn icon-only ghost" onClick={() => setToasts((x) => x.filter((y) => y.id !== t.id))} aria-label="Dismiss">
              <Icon name="x" size={13} />
            </button>
          </div>
        ))}
      </div>
    </DialogContext.Provider>
  );
}

function PromptDialog({ opts, onDone }: { opts: PromptOpts; onDone: (v: string | null) => void }) {
  const [value, setValue] = useState(opts.value ?? "");
  return (
    <Modal small title={opts.title} onClose={() => onDone(null)}>
      <form
        className="form"
        onSubmit={(e) => {
          e.preventDefault();
          if (value.trim()) onDone(value);
        }}
      >
        <label>
          {opts.label}
          <input autoFocus type={opts.password ? "password" : "text"} autoComplete="off" value={value} onChange={(e) => setValue(e.target.value)} />
        </label>
        <div className="actions">
          <button type="button" className="btn" onClick={() => onDone(null)}>
            Cancel
          </button>
          <button className="btn primary" disabled={!value.trim()}>
            {opts.confirmLabel ?? "Save"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
