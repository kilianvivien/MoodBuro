import type { Session } from "../../lib/timetable";
import { useI18n } from "../../i18n/context";

/** Minutes since midnight of an `HH:MM` wall time. */
export function minutes(time: string | null): number | null {
  if (!time || !/^\d{2}:\d{2}$/.test(time)) return null;
  const [h, m] = time.split(":").map(Number);
  return h * 60 + m;
}

/** `YYYY-MM-DD` plus `days`, without touching time zones. */
export function addDays(day: string, days: number): string {
  const [y, m, d] = day.split("-").map(Number);
  const date = new Date(Date.UTC(y, m - 1, d + days));
  return date.toISOString().slice(0, 10);
}

/** Monday of the week containing `day` (ISO weekday). */
export function mondayOf(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  const weekday = (new Date(Date.UTC(y, m - 1, d)).getUTCDay() + 6) % 7;
  return addDays(day, -weekday);
}

export interface Placed {
  session: Session;
  /** Fractions of the day column. */
  left: number;
  width: number;
  /** Later sessions are drawn above the ones they indent over. */
  depth: number;
}

const bounds = (s: Session) => {
  const start = minutes(s.start)!;
  return [start, Math.max(minutes(s.end) ?? start + 30, start + 15)] as const;
};

/** Minutes after which a later session indents instead of sharing width. */
const STAGGER = 25;
const INDENT = 0.16;

/**
 * Calendar-style layout of one day's timed sessions. Sessions starting
 * together share the column; one starting later is indented over the
 * earlier ones, whose titles stay visible above it. Nothing is hidden.
 */
export function layoutDay(sessions: Session[]): Placed[] {
  const timed = sessions
    .filter((s) => minutes(s.start) !== null)
    .sort(
      (a, b) =>
        bounds(a)[0] - bounds(b)[0] ||
        bounds(b)[1] - bounds(a)[1] ||
        a.uid.localeCompare(b.uid),
    );
  const overlaps = (a: Session, b: Session) => {
    const [s1, e1] = bounds(a);
    const [s2, e2] = bounds(b);
    return s1 < e2 && s2 < e1;
  };
  // Indentation level: one deeper than the deepest session it starts after.
  const level = new Map<Session, number>();
  for (const s of timed) {
    const below = timed.filter(
      (o) =>
        level.has(o) &&
        overlaps(o, s) &&
        bounds(o)[0] <= bounds(s)[0] - STAGGER,
    );
    level.set(
      s,
      below.length ? Math.max(...below.map((o) => level.get(o)!)) + 1 : 0,
    );
  }
  return timed.map((session) => {
    const start = bounds(session)[0];
    const peers = timed.filter(
      (o) => overlaps(o, session) && Math.abs(bounds(o)[0] - start) < STAGGER,
    );
    const depth = Math.max(...peers.map((o) => level.get(o)!));
    const indent = Math.min(depth, 3) * INDENT;
    const share = (1 - indent) / peers.length;
    return {
      session,
      left: indent + peers.indexOf(session) * share,
      width: share,
      depth,
    };
  });
}

export function WeekCalendar({
  monday,
  sessions,
  selected,
  pending,
  today,
  onSelect,
}: {
  /** `YYYY-MM-DD` of the week's Monday. */
  monday: string;
  sessions: Session[];
  selected: string | null;
  /** Session UIDs with an unanswered choice. */
  pending: ReadonlySet<string>;
  /** Today's date in Paris, `YYYY-MM-DD`. */
  today: string;
  onSelect: (uid: string) => void;
}) {
  const { t, locale } = useI18n();
  const weekend = sessions.some((s) => {
    const index = Math.round(
      (Date.parse(s.date) - Date.parse(monday)) / 86_400_000,
    );
    return index >= 5;
  });
  const days = Array.from({ length: weekend ? 7 : 5 }, (_, i) =>
    addDays(monday, i),
  );
  const starts = sessions
    .map((s) => minutes(s.start))
    .filter((m) => m !== null);
  const ends = sessions
    .map((s) => minutes(s.end) ?? (minutes(s.start) ?? 0) + 30)
    .filter((m) => m > 0);
  const first = Math.floor(Math.min(8 * 60, ...starts) / 60) * 60;
  const last = Math.ceil(Math.max(18 * 60, ...ends) / 60) * 60;
  const span = last - first;
  const hours = Array.from({ length: span / 60 + 1 }, (_, i) => first + i * 60);
  // Paris wall-clock minutes now, for the current-time line.
  const [hh, mm] = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  })
    .format(new Date())
    .split(":")
    .map(Number);
  const now = hh * 60 + mm;
  const heading = new Intl.DateTimeFormat(locale, {
    weekday: "short",
    day: "numeric",
    month: "short",
    timeZone: "UTC",
  });
  const allDayOf = (day: string) =>
    sessions.filter(
      (s) => s.start === null && s.date <= day && s.lastDate >= day,
    );
  const hasAllDay = days.some((day) => allDayOf(day).length > 0);
  const label = (s: Session) =>
    [
      s.title,
      s.start ? `${s.start}–${s.end ?? ""}` : t.timetable.allDay,
      s.location ?? s.locationNote,
      s.tentative ? t.timetable.tentative : null,
      pending.has(s.uid) ? t.timetable.review(1) : null,
    ]
      .filter(Boolean)
      .join(", ");
  // What fits: one line under 50 minutes, title and time under 100.
  const size = (s: Session) => {
    const start = minutes(s.start);
    if (start === null) return "compact";
    const length = (minutes(s.end) ?? start + 30) - start;
    return length < 50 ? "compact" : length < 100 ? "medium" : "large";
  };
  // Entrance order for the stagger: reading order, capped so it stays quick.
  const order = new Map(
    [...sessions]
      .sort(
        (a, b) =>
          a.date.localeCompare(b.date) ||
          (minutes(a.start) ?? -1) - (minutes(b.start) ?? -1),
      )
      .map((s, i) => [s.uid, Math.min(i, 16)]),
  );
  const card = (s: Session, style?: React.CSSProperties) => (
    <button
      key={s.uid}
      type="button"
      className="week-session"
      data-tentative={s.tentative || undefined}
      data-size={size(s)}
      data-pending={pending.has(s.uid) || undefined}
      aria-pressed={selected === s.uid}
      aria-label={label(s)}
      title={label(s)}
      style={{ ...style, "--i": order.get(s.uid) ?? 0 } as React.CSSProperties}
      onClick={() => onSelect(s.uid)}
    >
      <span className="week-session-title">{s.title}</span>
      {s.start && (
        <span className="week-session-meta">
          {s.start}–{s.end}
        </span>
      )}
      {(s.location || s.locationNote) && (
        <span className="week-session-meta week-session-place">
          {s.location ?? s.locationNote}
        </span>
      )}
    </button>
  );
  return (
    <div
      className="week-calendar"
      style={{ "--week-days": days.length } as React.CSSProperties}
    >
      <div className="week-head" aria-hidden="true">
        <span />
        {days.map((day) => (
          <span
            key={day}
            className="week-day-name"
            data-today={day === today || undefined}
          >
            {heading.format(new Date(`${day}T00:00:00Z`))}
          </span>
        ))}
      </div>
      {hasAllDay && (
        <div className="week-all-day">
          <span className="week-hour-label">{t.timetable.allDayShort}</span>
          {days.map((day) => (
            <div key={day} className="week-all-day-cell">
              {allDayOf(day).map((s) => card(s))}
            </div>
          ))}
        </div>
      )}
      <div
        className="week-body"
        style={{ "--week-span": span / 60 } as React.CSSProperties}
      >
        <div className="week-hours" aria-hidden="true">
          {hours.slice(0, -1).map((h) => (
            <span
              key={h}
              className="week-hour-label"
              style={{ top: `${((h - first) / span) * 100}%` }}
            >
              {String(h / 60).padStart(2, "0")}:00
            </span>
          ))}
        </div>
        {days.map((day) => (
          <section
            key={day}
            className="week-column"
            data-today={day === today || undefined}
            aria-label={heading.format(new Date(`${day}T00:00:00Z`))}
          >
            {hours.slice(1, -1).map((h) => (
              <span
                key={h}
                className="week-hour-line"
                aria-hidden="true"
                style={{ top: `${((h - first) / span) * 100}%` }}
              />
            ))}
            {day === today && now >= first && now <= last && (
              <span
                className="week-now"
                aria-hidden="true"
                style={{ top: `${((now - first) / span) * 100}%` }}
              />
            )}
            {layoutDay(sessions.filter((s) => s.date === day)).map(
              ({ session, left, width, depth }) => {
                const start = minutes(session.start)!;
                const end = Math.max(
                  minutes(session.end) ?? start + 30,
                  start + 15,
                );
                return card(session, {
                  top: `${((start - first) / span) * 100}%`,
                  height: `${((end - start) / span) * 100}%`,
                  left: `${left * 100}%`,
                  width: `${width * 100}%`,
                  zIndex: depth + 1,
                });
              },
            )}
          </section>
        ))}
      </div>
    </div>
  );
}

/** The same week as a chronological list: narrow windows and VoiceOver. */
export function WeekList({
  sessions,
  selected,
  pending,
  onSelect,
}: {
  sessions: Session[];
  selected: string | null;
  pending: ReadonlySet<string>;
  onSelect: (uid: string) => void;
}) {
  const { t, locale } = useI18n();
  const heading = new Intl.DateTimeFormat(locale, {
    weekday: "long",
    day: "numeric",
    month: "long",
    timeZone: "UTC",
  });
  const days = [...new Set(sessions.map((s) => s.date))].sort();
  return (
    <div className="week-list">
      {days.map((day) => (
        <section key={day} className="day-section">
          <h3 className="day-title">
            {heading.format(new Date(`${day}T00:00:00Z`))}
          </h3>
          <ul className="group" role="list">
            {sessions
              .filter((s) => s.date === day)
              .sort(
                (a, b) => (minutes(a.start) ?? -1) - (minutes(b.start) ?? -1),
              )
              .map((s) => (
                <li key={s.uid} className="list-row">
                  <span className="event-time">
                    {s.start ?? t.timetable.allDayShort}
                    {s.end && <span>{s.end}</span>}
                  </span>
                  <span className="row-text">
                    <span className="row-title" title={s.title}>
                      {s.title}
                    </span>
                    {(s.location || s.locationNote || s.module) && (
                      <span className="row-subtitle">
                        {[s.location ?? s.locationNote, s.module]
                          .filter(Boolean)
                          .join(" · ")}
                      </span>
                    )}
                  </span>
                  {s.tentative && (
                    <span className="tag">{t.timetable.tentative}</span>
                  )}
                  {pending.has(s.uid) && (
                    <span className="tag" data-kind="due">
                      {t.timetable.review(1)}
                    </span>
                  )}
                  <button
                    className="button small"
                    aria-pressed={selected === s.uid}
                    onClick={() => onSelect(s.uid)}
                  >
                    {t.common.details}
                  </button>
                </li>
              ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
