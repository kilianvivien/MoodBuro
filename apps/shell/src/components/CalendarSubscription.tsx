import { useEffect, useRef, useState } from "react";
import { desktop, ipc, message, type CalendarHandoff, type Site } from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import { initials } from "../lib/site";
import { Confirm, Notice } from "./ui";

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
  return (
    <Confirm
      id="calendar-consent"
      title={t.calendar.consentTitle(site.name)}
      body={t.calendar.consent}
      cancel={t.downloads.cancel}
      confirm={busy ? t.calendar.connecting : t.calendar.confirm}
      busy={busy}
      onCancel={onCancel}
      onConfirm={onConfirm}
    >
      {repeat && <p className="confirm-body">{t.calendar.duplicate}</p>}
    </Confirm>
  );
}
export function CalendarSubscription({
  sites,
  request = 0,
  onOpenPage,
}: {
  sites: Site[];
  request?: number;
  /** Shows the organisation's own calendar export page in Moodle. */
  onOpenPage?: (siteId: string) => void;
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
              <span className="row-avatar" aria-hidden="true">
                {site.shortLabel || initials(site.name)}
              </span>
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
                    if (onOpenPage) onOpenPage(site.id);
                    else
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
        <Notice tone="error">
          <ErrorText error={error} context="navigation" />
        </Notice>
      )}
      {sent && (
        <Notice tone="success">
          {desktop ? t.calendar.sent : t.calendar.demoSent}
        </Notice>
      )}
      <details className="help-disclosure">
        <summary>{t.common.details}</summary>
        <p>{t.calendar.details}</p>
      </details>
    </details>
  );
}
