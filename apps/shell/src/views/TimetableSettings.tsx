import { useEffect, useRef, useState } from "react";
import { useI18n } from "../i18n/context";
import { message, type Site } from "../ipc";
import {
  defaultOptions,
  timetable,
  type CalendarSync,
  type TimetableOptions,
  type TimetableStatus,
} from "../lib/timetable";
import { Notice, PopupButton } from "../components/ui";
import { useTimetableError } from "./Timetable";

const ALARMS = [null, 10, 15, 30, 60, 120] as const;

/** Settings › InspCal: defaults for new timetable weeks and their exports. */
export function TimetableSettings({
  sites,
  status,
}: {
  /** Organisations with timetables (the INSP Moodle). */
  sites: Site[];
  status: TimetableStatus | null;
}) {
  const { t } = useI18n();
  const s = t.timetable.settings;
  return (
    <section aria-labelledby="inspcal-heading">
      <h2 id="inspcal-heading" className="section-title">
        {s.title}
      </h2>
      {sites.map((site) => (
        <SiteDefaults key={site.id} site={site} showName={sites.length > 1} />
      ))}
      <details className="help-disclosure section-footnote">
        <summary>{t.common.details}</summary>
        <p>{s.description}</p>
        <p>{s.calendarHelp}</p>
        {status && <p>{s.engine(status.engine, status.rules)}</p>}
        <p>{s.notices}</p>
      </details>
    </section>
  );
}

function SiteDefaults({ site, showName }: { site: Site; showName: boolean }) {
  const { t } = useI18n();
  const s = t.timetable.settings;
  const errorText = useTimetableError();
  const [options, setOptions] = useState<TimetableOptions>(defaultOptions);
  const [prefix, setPrefix] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const alive = useRef(true);
  const [sync, setSync] = useState<CalendarSync | null>(null);
  const [syncBusy, setSyncBusy] = useState(false);
  const reloadSync = () =>
    timetable
      .calendar(site.id, site.userId)
      .then((status) => alive.current && setSync(status))
      .catch(() => {});
  async function syncAction(run: () => Promise<unknown>) {
    setError(null);
    setSyncBusy(true);
    try {
      await run();
    } catch (e) {
      if (alive.current) setError(message(e));
    } finally {
      await reloadSync();
      if (alive.current) setSyncBusy(false);
    }
  }
  useEffect(() => {
    alive.current = true;
    void reloadSync();
    timetable
      .preferences(site.id, site.userId)
      .then((saved) => {
        if (!alive.current) return;
        setOptions(saved);
        setPrefix(saved.prefix);
      })
      .catch((e) => alive.current && setError(message(e)));
    return () => {
      alive.current = false;
    };
  }, [site.id, site.userId]);
  async function save(change: Partial<TimetableOptions>) {
    const previous = options;
    const next = { ...options, ...change };
    setOptions(next);
    setSaving(true);
    setError(null);
    try {
      await timetable.setPreferences(site.id, site.userId, next);
    } catch (e) {
      // Only this field rolls back.
      if (alive.current) {
        setOptions(previous);
        setPrefix(previous.prefix);
        setError(message(e));
      }
    } finally {
      if (alive.current) setSaving(false);
    }
  }
  const commitPrefix = () => {
    if (prefix !== options.prefix) void save({ prefix });
  };
  return (
    <>
      {showName && <h3 className="day-title">{site.name}</h3>}
      <div className="group">
        <div className="group-row">
          <div className="row-text">
            <span className="row-title">{s.cohort}</span>
          </div>
          <PopupButton
            label={s.cohort}
            value={options.cohort}
            disabled={saving}
            onChange={(cohort) =>
              void save({ cohort: cohort as TimetableOptions["cohort"] })
            }
          >
            {(["ALL", "FI", "CIL", "OFF"] as const).map((c) => (
              <option key={c} value={c}>
                {t.timetable.cohorts[c]}
              </option>
            ))}
          </PopupButton>
        </div>
        <div className="group-row">
          <div className="row-text">
            <label className="row-title" htmlFor={`prefix-${site.id}`}>
              {s.prefix}
            </label>
          </div>
          <span className="text-field compact settings-prefix">
            <input
              id={`prefix-${site.id}`}
              value={prefix}
              maxLength={60}
              placeholder="EDT · "
              disabled={saving}
              onChange={(e) => setPrefix(e.target.value)}
              onBlur={commitPrefix}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitPrefix();
                if (e.key === "Escape") setPrefix(options.prefix);
              }}
            />
          </span>
        </div>
        <div className="group-row">
          <div className="row-text">
            <span className="row-title">{s.alarm}</span>
          </div>
          <PopupButton
            label={s.alarm}
            value={options.alarm ?? ""}
            disabled={saving}
            onChange={(value) =>
              void save({ alarm: value === "" ? null : Number(value) })
            }
          >
            {ALARMS.map((minutes) => (
              <option key={minutes ?? "none"} value={minutes ?? ""}>
                {minutes === null ? s.alarmNone : s.alarmMinutes(minutes)}
              </option>
            ))}
          </PopupButton>
        </div>
        <div className="group-row">
          <div className="row-text">
            <span className="row-title">{s.calendar}</span>
            <span className="row-subtitle">
              {sync?.access === "denied"
                ? s.calendarDenied
                : sync?.enabled
                  ? s.calendarOn
                  : s.calendarOff}
            </span>
          </div>
          <button
            className="button"
            disabled={syncBusy || sync?.access === "denied"}
            onClick={() =>
              void syncAction(() =>
                sync?.enabled
                  ? timetable.calendarDisable(site.id, site.userId)
                  : timetable.calendarEnable(site.id, site.userId),
              )
            }
          >
            {sync?.enabled ? s.calendarStop : s.calendarStart}
          </button>
        </div>
      </div>
      {error && <Notice tone="error">{errorText(error)}</Notice>}
    </>
  );
}
