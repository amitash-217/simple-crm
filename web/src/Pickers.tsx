import { Children, Fragment, isValidElement, useEffect, useRef, useState, type KeyboardEvent, type ReactElement, type ReactNode } from "react";
import { Icon } from "./icons";

interface Option {
  value: string;
  label: string;
  disabled: boolean;
}

/** Reads `<option>` children, so a call site can keep the markup of a native select. */
function collect(children: ReactNode): Option[] {
  const out: Option[] = [];
  const walk = (nodes: ReactNode) => {
    for (const c of Children.toArray(nodes)) {
      if (!isValidElement(c)) continue;
      const el = c as ReactElement<{ value?: string; disabled?: boolean; children?: ReactNode }>;
      if (el.type === Fragment) walk(el.props.children);
      else if (el.type === "option") {
        const label = Children.toArray(el.props.children).join("");
        out.push({ value: String(el.props.value ?? label), label, disabled: !!el.props.disabled });
      }
    }
  };
  walk(children);
  return out;
}

/** Where a fixed panel goes: under its trigger, or above it when there is no room below. */
function place(trigger: HTMLElement, want: number, minWidth = 0): React.CSSProperties {
  const r = trigger.getBoundingClientRect();
  const below = window.innerHeight - r.bottom - 8;
  const up = below < Math.min(want, 200) && r.top > below;
  const left = Math.max(8, Math.min(r.left, window.innerWidth - Math.max(r.width, minWidth) - 8));
  return up
    ? { left, minWidth: Math.max(r.width, minWidth), bottom: window.innerHeight - r.top + 4, maxHeight: Math.min(want, r.top - 12) }
    : { left, minWidth: Math.max(r.width, minWidth), top: r.bottom + 4, maxHeight: Math.min(want, below) };
}

/** Closes a panel on a press outside it or a resize, and moves it with its trigger when something scrolls. */
function usePanel(open: boolean, wrap: React.RefObject<HTMLElement | null>, panel: React.RefObject<HTMLElement | null>, close: () => void, move: () => void) {
  useEffect(() => {
    if (!open) return;
    const down = (e: MouseEvent) => {
      if (wrap.current && !wrap.current.contains(e.target as Node)) close();
    };
    const scroll = (e: Event) => {
      if (!panel.current?.contains(e.target as Node)) move();
    };
    document.addEventListener("mousedown", down);
    window.addEventListener("scroll", scroll, true);
    window.addEventListener("resize", close);
    return () => {
      document.removeEventListener("mousedown", down);
      window.removeEventListener("scroll", scroll, true);
      window.removeEventListener("resize", close);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);
}

interface SelectProps {
  value: string;
  /** Called like a native select's handler: it reads `e.target.value`. */
  onChange: (e: { target: { value: string } }) => void;
  className?: string;
  children: ReactNode;
  autoFocus?: boolean;
  /** Open the list as soon as it appears. */
  defaultOpen?: boolean;
  onBlur?: () => void;
  onKeyDown?: (e: KeyboardEvent) => void;
}

/** A dropdown in the app's own look, in place of the browser's native select. */
export function Select(p: SelectProps) {
  const options = collect(p.children);
  const current = options.find((o) => o.value === p.value);
  const [open, setOpen] = useState(false);
  const [at, setAt] = useState(0);
  const [pos, setPos] = useState<React.CSSProperties>({});
  const wrap = useRef<HTMLDivElement>(null);
  const btn = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const close = () => setOpen(false);

  const move = () => btn.current && setPos(place(btn.current, 260));
  const show = () => {
    if (!btn.current) return;
    move();
    setAt(Math.max(0, options.findIndex((o) => o.value === p.value)));
    setOpen(true);
  };
  useEffect(() => {
    if (p.defaultOpen) show();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  usePanel(open, wrap, panel, close, move);

  useEffect(() => {
    if (open) panel.current?.querySelector<HTMLElement>('[data-on="true"]')?.scrollIntoView({ block: "nearest" });
  }, [open, at]);

  const choose = (o: Option | undefined) => {
    if (!o || o.disabled) return;
    setOpen(false);
    if (o.value !== p.value) p.onChange({ target: { value: o.value } });
    btn.current?.focus();
  };

  const key = (e: KeyboardEvent) => {
    if (open) {
      const step = (d: number) => {
        e.preventDefault();
        let i = at;
        do i = (i + d + options.length) % options.length;
        while (options[i]?.disabled && i !== at);
        setAt(i);
      };
      if (e.key === "ArrowDown") return step(1);
      if (e.key === "ArrowUp") return step(-1);
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        return choose(options[at]);
      }
      if (e.key === "Escape") {
        e.stopPropagation();
        return close();
      }
      if (e.key === "Tab") close();
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === " " || (e.key === "Enter" && !p.onKeyDown)) {
      e.preventDefault();
      return show();
    }
    p.onKeyDown?.(e);
  };

  return (
    <div
      ref={wrap}
      className={`select-wrap${p.className ? ` ${p.className}` : ""}`}
      onBlur={(e) => {
        if (!wrap.current?.contains(e.relatedTarget as Node | null)) p.onBlur?.();
      }}
    >
      <button ref={btn} type="button" className="select-btn" autoFocus={p.autoFocus} aria-haspopup="listbox" aria-expanded={open} onClick={() => (open ? close() : show())} onKeyDown={key}>
        <span className={current?.label ? "" : "muted"}>{current?.label || "—"}</span>
        <Icon name="chevron" size={16} />
      </button>
      {open && (
        // A label around the select would forward a click inside the list to the trigger and reopen it.
        <div ref={panel} className="select-panel" role="listbox" style={pos} onMouseDown={(e) => e.preventDefault()} onClick={(e) => e.preventDefault()}>
          {options.map((o, i) => (
            <div
              key={`${o.value}-${i}`}
              role="option"
              aria-selected={o.value === p.value}
              aria-disabled={o.disabled}
              data-on={i === at}
              className={`select-opt${i === at ? " on" : ""}${o.value === p.value ? " picked" : ""}`}
              onMouseEnter={() => setAt(i)}
              onClick={() => choose(o)}
            >
              <span>{o.label || "—"}</span>
              {o.value === p.value && <Icon name="check" size={16} />}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
const two = (n: number) => String(n).padStart(2, "0");
const ymd = (y: number, m: number, d: number) => `${y}-${two(m + 1)}-${two(d)}`;

function parse(v: string) {
  const m = /^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}))?/.exec(v);
  return m ? { y: +m[1], m: +m[2] - 1, d: +m[3], hh: m[4] === undefined ? null : +m[4], mm: m[5] === undefined ? null : +m[5] } : null;
}

interface DateProps {
  /** `YYYY-MM-DD`, or `YYYY-MM-DDTHH:mm` with `time`, in local time. Empty for none. */
  value: string;
  time?: boolean;
  onChange: (v: string) => void;
  autoFocus?: boolean;
  defaultOpen?: boolean;
  onBlur?: () => void;
  onKeyDown?: (e: KeyboardEvent) => void;
}

/** A calendar (and, with `time`, a time of day) in the app's own look, in place of the native date input. */
export function DatePicker(p: DateProps) {
  const set = parse(p.value);
  const today = new Date();
  const [open, setOpen] = useState(false);
  const [view, setView] = useState({ y: set?.y ?? today.getFullYear(), m: set?.m ?? today.getMonth() });
  const [pos, setPos] = useState<React.CSSProperties>({});
  const wrap = useRef<HTMLDivElement>(null);
  const btn = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const close = () => setOpen(false);

  const move = () => btn.current && setPos(place(btn.current, p.time ? 390 : 340, 264));
  const show = () => {
    if (!btn.current) return;
    move();
    const s = parse(p.value);
    if (s) setView({ y: s.y, m: s.m });
    setOpen(true);
  };
  useEffect(() => {
    if (p.defaultOpen) show();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  usePanel(open, wrap, panel, close, move);

  const nowTime = { hh: today.getHours(), mm: today.getMinutes() };
  const emit = (y: number, m: number, d: number, hh: number, mm: number) => p.onChange(p.time ? `${ymd(y, m, d)}T${two(hh)}:${two(mm)}` : ymd(y, m, d));
  const pick = (y: number, m: number, d: number) => {
    emit(y, m, d, set?.hh ?? nowTime.hh, set?.mm ?? nowTime.mm);
    if (!p.time) close();
  };
  const setTime = (hh: number, mm: number) => {
    const base = set ?? { y: today.getFullYear(), m: today.getMonth(), d: today.getDate() };
    emit(base.y, base.m, base.d, hh, mm);
  };
  const shift = (d: number) =>
    setView((v) => {
      const n = v.y * 12 + v.m + d;
      return { y: Math.floor(n / 12), m: ((n % 12) + 12) % 12 };
    });

  // Monday-first grid of six weeks.
  const first = new Date(view.y, view.m, 1);
  const lead = (first.getDay() + 6) % 7;
  const cells = Array.from({ length: 42 }, (_, i) => new Date(view.y, view.m, 1 - lead + i));
  const isSet = (d: Date) => !!set && set.y === d.getFullYear() && set.m === d.getMonth() && set.d === d.getDate();
  const isToday = (d: Date) => d.getFullYear() === today.getFullYear() && d.getMonth() === today.getMonth() && d.getDate() === today.getDate();

  const label = set ? `${set.d} ${MONTHS[set.m].slice(0, 3)} ${set.y}${p.time && set.hh !== null ? `, ${two(set.hh)}:${two(set.mm ?? 0)}` : ""}` : "";

  const key = (e: KeyboardEvent) => {
    if (open) {
      if (e.key === "Escape") {
        e.stopPropagation();
        close();
      }
      return;
    }
    if (e.key === "ArrowDown" || e.key === " " || (e.key === "Enter" && !p.onKeyDown)) {
      e.preventDefault();
      return show();
    }
    p.onKeyDown?.(e);
  };

  return (
    <div
      ref={wrap}
      className="select-wrap date-wrap"
      onBlur={(e) => {
        if (!wrap.current?.contains(e.relatedTarget as Node | null)) p.onBlur?.();
      }}
    >
      <button ref={btn} type="button" className="select-btn" autoFocus={p.autoFocus} aria-haspopup="dialog" aria-expanded={open} onClick={() => (open ? close() : show())} onKeyDown={key}>
        <span className={label ? "" : "muted"}>{label || (p.time ? "Select date and time" : "Select date")}</span>
        <Icon name="date" size={16} />
      </button>
      {open && (
        <div
          ref={panel}
          className="date-panel"
          role="dialog"
          aria-label="Choose a date"
          style={pos}
          onClick={(e) => e.preventDefault()}
          // Keep focus on the trigger, so a grid cell is not saved while you pick. The time boxes still take focus.
          onMouseDown={(e) => (e.target as HTMLElement).tagName !== "INPUT" && e.preventDefault()}
        >
          <div className="date-head">
            <button type="button" className="date-nav prev" onClick={() => shift(-1)} aria-label="Previous month">
              <Icon name="chevron" size={16} />
            </button>
            <span className="date-title">
              {MONTHS[view.m]} {view.y}
            </span>
            <button type="button" className="date-nav next" onClick={() => shift(1)} aria-label="Next month">
              <Icon name="chevron" size={16} />
            </button>
          </div>
          <div className="date-grid">
            {DAYS.map((d) => (
              <span key={d} className="date-dow">
                {d}
              </span>
            ))}
            {cells.map((d) => (
              <button
                type="button"
                key={d.getTime()}
                className={`date-day${d.getMonth() !== view.m ? " out" : ""}${isSet(d) ? " on" : ""}${isToday(d) ? " today" : ""}`}
                aria-label={d.toDateString()}
                aria-pressed={isSet(d)}
                onClick={() => pick(d.getFullYear(), d.getMonth(), d.getDate())}
              >
                {d.getDate()}
              </button>
            ))}
          </div>
          {p.time && (
            <div className="date-time">
              <span className="muted">Time</span>
              <TimeBox value={set?.hh ?? nowTime.hh} max={23} label="Hours" onChange={(h) => setTime(h, set?.mm ?? nowTime.mm)} />
              <span>:</span>
              <TimeBox value={set?.mm ?? nowTime.mm} max={59} label="Minutes" onChange={(m) => setTime(set?.hh ?? nowTime.hh, m)} />
            </div>
          )}
          <div className="date-foot">
            <button
              type="button"
              className="btn ghost small"
              onClick={() => {
                p.onChange("");
                close();
              }}
            >
              Clear
            </button>
            <span className="grow" />
            <button
              type="button"
              className="btn small"
              onClick={() => {
                emit(today.getFullYear(), today.getMonth(), today.getDate(), nowTime.hh, nowTime.mm);
                setView({ y: today.getFullYear(), m: today.getMonth() });
                if (!p.time) close();
              }}
            >
              {p.time ? "Now" : "Today"}
            </button>
            {p.time && (
              <button
                type="button"
                className="btn primary small"
                onClick={() => {
                  close();
                  btn.current?.focus();
                }}
              >
                Done
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

/** Two digits you can type, or step with the arrow keys. It keeps what you type until you leave it. */
function TimeBox(p: { value: number; max: number; label: string; onChange: (n: number) => void }) {
  const [text, setText] = useState<string | null>(null);
  const commit = (s: string) => {
    const n = Math.min(p.max, Math.max(0, parseInt(s, 10)));
    if (!isNaN(n)) p.onChange(n);
  };
  return (
    <input
      className="time-box"
      inputMode="numeric"
      maxLength={2}
      aria-label={p.label}
      value={text ?? two(p.value)}
      onFocus={(e) => e.target.select()}
      onChange={(e) => {
        const s = e.target.value.replace(/\D/g, "");
        setText(s);
        if (s !== "") commit(s);
      }}
      onBlur={() => setText(null)}
      onKeyDown={(e) => {
        if (e.key === "ArrowUp" || e.key === "ArrowDown") {
          e.preventDefault();
          setText(null);
          p.onChange((p.value + (e.key === "ArrowUp" ? 1 : -1) + p.max + 1) % (p.max + 1));
        }
        if (e.key === "Enter") e.currentTarget.blur();
      }}
    />
  );
}
