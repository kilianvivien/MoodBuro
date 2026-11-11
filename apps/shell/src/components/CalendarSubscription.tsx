import { useEffect, useRef, useState } from "react";
import { desktop, ipc, message, type CalendarHandoff, type Site } from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";

export function CalendarConsent({
  site,
  repeat,
  busy,
  onCancel,
  onConfirm,
}: {
  site: Site;
  repeat: boolean;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => cancel.current?.focus(), []);
  return (
    <div
      className="confirm standalone"
      role="alertdialog"
      aria-labelledby="calendar-consent-title"
      aria-describedby="calendar-consent-body"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          if (!busy) onCancel();
        }
      }}
    >
      <p id="calendar-consent-title" className="confirm-title">
        {t.calendar.consentTitle(site.name)}
      </p>
      <p id="calendar-consent-body" className="confirm-body">
        {t.calendar.consent}
      </p>
      {repeat && <p className="confirm-body">{t.calendar.duplicate}</p>}
      <div className="confirm-actions">
        <button
          ref={cancel}
          className="button"
          disabled={busy}
          onClick={onCancel}
        >
          {t.downloads.cancel}
        </button>
        <button className="button" disabled={busy} onClick={onConfirm}>
          {busy ? t.calendar.connecting : t.calendar.confirm}
        </button>
      </div>
    </div>
  );
}
export function CalendarSubscription({
  sites,
  request = 0,
}: {
  sites: Site[];
  request?: number;
}) {
  const { t, date } = useI18n();
  const [history, setHistory] = useState<CalendarHandoff[]>([]);
  const [target, setTarget] = useState<{ site: Site; repeat: boolean } | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sent, setSent] = useState(false);
  const alive = useRef(true);
  const pending = useRef(false);
  const historyRequest = useRef(0);
  const summary = useRef<HTMLElement>(null);
  const details = useRef<HTMLDetailsElement>(null);
  const activeTarget =
    target &&
    sites.some(
      (site) =>
        site.id === target.site.id && site.userId === target.site.userId,
    )
      ? target
      : null;
  useEffect(() => {
    if (request > 0 && details.current) {
      details.current.open = true;
      summary.current?.focus();
    }
  }, [request]);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  async function reload() {
    const request = ++historyRequest.current;
    const values = await ipc.calendarHandoffs();
    if (alive.current && request === historyRequest.current) setHistory(values);
  }
  function cancel() {
    setTarget(null);
    summary.current?.focus();
  }
  async function subscribe() {
    if (!target || pending.current) return;
    if (
      !sites.some(
        (site) =>
          site.id === target.site.id && site.userId === target.site.userId,
      )
    ) {
      setTarget(null);
      return;
    }
    pending.current = true;
    setBusy(true);
    setError(null);
    setSent(false);
    try {
      await ipc.subscribeCalendar(
        target.site.id,
        target.site.userId,
        true,
        target.repeat ||
          history.some(
            (h) =>
              h.siteId === target.site.id && h.userId === target.site.userId,
          ),
      );
      if (alive.current) {
        setSent(true);
        setTarget(null);
        summary.current?.focus();
      }
    } catch (error) {
      if (alive.current) setError(message(error));
    } finally {
      try {
        await reload();
      } catch (error) {
        if (alive.current) setError(message(error));
      }
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  return (
    <details
      ref={details}
      className="help-disclosure calendar-subscriptions"
      onToggle={(event) => {
        if (event.currentTarget.open)
          void reload().catch((error) => {
            if (alive.current) setError(message(error));
          });
      }}
    >
      <summary ref={summary}>{t.calendar.title}</summary>
      <p className="section-footnote">{t.calendar.scope}</p>
      {!desktop && <p className="section-footnote">{t.calendar.demo}</p>}
      <ul className="group" role="list">
        {sites.map((site) => {
          const previous = history.find(
            (h) => h.siteId === site.id && h.userId === site.userId,
          );
          return (
            <li className="list-row" key={`${site.id}:${site.userId}`}>
              <span className="row-text">
                <span className="row-title">{site.name}</span>
                {previous && (
                  <span className="row-subtitle">
                    {t.calendar.requested(
                      date(previous.openedAt * 1000, { dateStyle: "medium" }),
                    )}
                  </span>
                )}
              </span>
              <span className="row-actions">
                <button
                  className="button small"
                  disabled={busy || activeTarget !== null}
                  onClick={() => {
                    setTarget({ site, repeat: !!previous });
                    setError(null);
                    setSent(false);
                  }}
                >
                  {previous ? t.calendar.again : t.calendar.setup}
                </button>
                <button
                  className="button small"
                  disabled={busy}
                  onClick={() => {
                    void ipc
                      .openPage(site.id, "calendar-export")
                      .catch((error) => setError(message(error)));
                  }}
                >
                  {t.calendar.moodle}
                </button>
              </span>
            </li>
          );
        })}
      </ul>
      {target &&
        sites.some(
          (s) => s.id === target.site.id && s.userId === target.site.userId,
        ) && (
          <CalendarConsent
            site={target.site}
            repeat={
              target.repeat ||
              history.some(
                (h) =>
                  h.siteId === target.site.id &&
                  h.userId === target.site.userId,
              )
            }
            busy={busy}
            onCancel={cancel}
            onConfirm={() => void subscribe()}
          />
        )}
      {error && (
        <p className="section-message error" role="alert">
          <ErrorText error={error} context="navigation" />
        </p>
      )}
      {sent && (
        <p className="section-message" role="status">
          {desktop ? t.calendar.sent : t.calendar.demoSent}
        </p>
      )}
      <details className="help-disclosure">
        <summary>{t.common.details}</summary>
        <p>{t.calendar.details}</p>
      </details>
    </details>
  );
}
