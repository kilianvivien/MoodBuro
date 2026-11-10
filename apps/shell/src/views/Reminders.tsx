import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ipc,
  desktop,
  message,
  type ReminderSettings,
  type Site,
} from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";

export function ReminderRows({
  sites,
  settings,
  busy,
  onChange,
  now = Date.now() / 1000,
}: {
  sites: Site[];
  settings: ReminderSettings[];
  busy: boolean;
  now?: number;
  onChange: (settings: ReminderSettings, hour: boolean, day: boolean) => void;
}) {
  const { t, date } = useI18n();
  return (
    <ul className="group" role="list">
      {sites.map((site) => {
        const saved = settings.find(
          (s) => s.siteId === site.id && s.userId === site.userId,
        );
        return (
          <li className="group-row" key={site.id}>
            <div className="row-text">
              <span className="row-title">{site.name}</span>
              <span className="row-subtitle">
                {saved?.refreshedAt
                  ? t.reminders.updated(
                      date(saved.refreshedAt * 1000, {
                        dateStyle: "medium",
                        timeStyle: "short",
                      }),
                    )
                  : t.reminders.never}
              </span>
              {saved &&
                (saved.hour || saved.day) &&
                (saved.refreshedAt === null ||
                  now - saved.refreshedAt > 900) && (
                  <span className="row-subtitle" role="status">
                    {t.reminders.stale}
                  </span>
                )}
              {saved?.refreshError && (
                <span className="error" role="alert">
                  <ErrorText error={saved.refreshError} context="navigation" />
                </span>
              )}
              {saved?.deliveryFailed && (
                <span className="error" role="alert">
                  {t.reminders.deliveryFailed}
                </span>
              )}
            </div>
            <div className="row-actions">
              {(["day", "hour"] as const).map((offset) => (
                <label className="reminder-choice" key={offset}>
                  <input
                    type="checkbox"
                    checked={saved?.[offset] ?? false}
                    disabled={busy || !saved}
                    aria-label={t.reminders.label(
                      site.name,
                      t.reminders[offset],
                    )}
                    onChange={(event) => {
                      if (saved)
                        onChange(
                          saved,
                          offset === "hour" ? event.target.checked : saved.hour,
                          offset === "day" ? event.target.checked : saved.day,
                        );
                    }}
                  />
                  {t.reminders[offset]}
                </label>
              ))}
            </div>
          </li>
        );
      })}
    </ul>
  );
}
export function Reminders({ sites }: { sites: Site[] }) {
  const { t } = useI18n();
  const [settings, setSettings] = useState<ReminderSettings[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);
  const pending = useRef(false);
  // Never reuse a setting response after account replacement.
  const identity = sites.map((s) => `${s.id}:${s.userId}`).join("\n");
  useEffect(() => {
    alive.current = true;
    let current = true;
    let stop: (() => void) | undefined;
    const reload = async () => {
      try {
        const next = await ipc.reminders();
        if (current) {
          setSettings(next);
        }
      } catch (error) {
        if (current) setError(message(error));
      }
    };
    void reload();
    if (desktop)
      void listen("reminders://changed", () => void reload())
        .then((unlisten) => {
          if (current) stop = unlisten;
          else unlisten();
        })
        .catch((error) => {
          if (current) setError(message(error));
        });
    return () => {
      current = false;
      alive.current = false;
      stop?.();
    };
  }, [identity]);
  async function change(saved: ReminderSettings, hour: boolean, day: boolean) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    try {
      await ipc.setReminders(saved.siteId, saved.userId, hour, day);
      const next = await ipc.reminders();
      if (alive.current) setSettings(next);
    } catch (error) {
      if (alive.current) setError(message(error));
    } finally {
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  if (sites.length === 0) return null;
  return (
    <section aria-labelledby="reminders-title">
      <h2 className="section-title" id="reminders-title">
        {t.reminders.title}
      </h2>
      <p className="section-description">{t.reminders.summary}</p>
      <ReminderRows
        sites={sites}
        settings={settings}
        busy={busy}
        onChange={(saved, hour, day) => void change(saved, hour, day)}
      />
      {busy && (
        <p className="section-message" role="status">
          {t.reminders.saving}
        </p>
      )}
      {error && (
        <p className="section-message error" role="alert">
          <ErrorText error={error} context="settings" />
        </p>
      )}
      <details className="help-disclosure section-footnote">
        <summary>{t.reminders.help}</summary>
        <p>{t.reminders.hint}</p>
        <p>{t.reminders.limits}</p>
      </details>
    </section>
  );
}
