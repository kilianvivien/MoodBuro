import { Bell, CalendarDays, CircleAlert } from "lucide-react";
import { ipc, message, type AgendaEvent, type Site } from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import { dayLabel, groupByDay, timeLabel, timeZone } from "../lib/time";
import { useListing } from "../lib/useListing";
import { useRef, useState } from "react";
import {
  EmptyState,
  PageHeader,
  RefreshControl,
  SiteSelect,
  Skeleton,
} from "../components/ui";

export function AgendaList({
  events,
  sites,
  now = new Date(),
  onOpen,
}: {
  events: readonly AgendaEvent[];
  sites: readonly Site[];
  now?: Date;
  onOpen: (event: AgendaEvent) => void;
}) {
  const { t, locale } = useI18n();
  if (events.length === 0)
    return (
      <EmptyState
        icon={<CalendarDays size={22} strokeWidth={1.75} />}
        title={t.agenda.empty}
        hint={t.agenda.emptyHint}
      />
    );
  return groupByDay(events, (event) => event.timeStart, now).map((group) => (
    <section key={group.offset} className="day-section">
      <h3 className="day-title">{dayLabel(locale, group.date, now)}</h3>
      <ul className="group" role="list">
        {group.items.map((event) => {
          const start = new Date(event.timeStart * 1000);
          const end = new Date((event.timeStart + event.duration) * 1000);
          const where = [
            event.courseName,
            sites.length > 1 &&
              sites.find((site) => site.id === event.siteId)?.name,
          ]
            .filter(Boolean)
            .join(" · ");
          return (
            <li className="list-row" key={`${event.siteId}:${event.id}`}>
              <time className="event-time" dateTime={start.toISOString()}>
                {timeLabel(locale, start)}
                {event.duration > 0 && (
                  <span>{t.agenda.until(timeLabel(locale, end))}</span>
                )}
              </time>
              <span className="row-text">
                <span className="row-title" title={event.name}>
                  {event.name}
                </span>
                {where && <span className="row-subtitle">{where}</span>}
              </span>
              <span className="tag" data-kind={event.kind}>
                {t.agenda.kinds[event.kind] ?? t.agenda.kinds.other}
              </span>
              <button
                className="button small"
                aria-label={t.agenda.openLabel(event.name)}
                onClick={() => onOpen(event)}
              >
                {t.agenda.open}
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  ));
}

export function agendaRange(
  from: string,
  through: string,
): { start: number | null; end: number | null; valid: boolean } {
  const parse = (value: string, nextDay: boolean) => {
    if (!value) return null;
    if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return NaN;
    const [year, month, day] = value.split("-").map(Number);
    const date = new Date(year, month - 1, day);
    if (
      date.getFullYear() !== year ||
      date.getMonth() !== month - 1 ||
      date.getDate() !== day
    )
      return NaN;
    if (nextDay) date.setDate(date.getDate() + 1);
    return date.getTime() / 1000;
  };
  const start = parse(from, false);
  const end = parse(through, true);
  return {
    start,
    end,
    valid:
      (start === null || Number.isFinite(start)) &&
      (end === null || Number.isFinite(end)) &&
      (start === null || end === null || start < end),
  };
}

export function Agenda({
  sites,
  onOpenDay,
  onSettings,
}: {
  sites: Site[];
  onOpenDay: (siteId: string, time: number) => void;
  onSettings: () => void;
}) {
  const { t } = useI18n();
  const listing = useListing(sites, ipc.agenda, ipc.refreshAgenda);
  const [siteId, setSiteId] = useState<string | null>(null);
  const [showDates, setShowDates] = useState(false);
  const [from, setFrom] = useState("");
  const [through, setThrough] = useState("");
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exported, setExported] = useState<{
    filename: string;
    count: number;
    revealed: boolean;
  } | null>(null);
  const exportPending = useRef(false);
  const range = agendaRange(from, through);
  async function exportCalendar() {
    if (exportPending.current || !range.valid) return;
    exportPending.current = true;
    setExporting(true);
    setExportError(null);
    setExported(null);
    try {
      setExported(await ipc.exportAgenda(siteId, range.start, range.end));
    } catch (error) {
      setExportError(message(error));
    } finally {
      exportPending.current = false;
      setExporting(false);
    }
  }
  const shownSites = sites.filter((site) => !siteId || site.id === siteId);
  const events = listing.items.filter(
    (event) =>
      range.valid &&
      shownSites.some((site) => site.id === event.siteId) &&
      (range.start === null || event.timeStart >= range.start) &&
      (range.end === null || event.timeStart < range.end),
  );
  const failures = sites.filter((site) => listing.errors[site.id]);
  return (
    <div className="page">
      <PageHeader
        title={t.agenda.title}
        detail={events.length > 0 ? t.agenda.count(events.length) : undefined}
      >
        <button
          className="button icon"
          title={t.agenda.reminders}
          aria-label={t.agenda.reminders}
          onClick={onSettings}
        >
          <Bell size={14} strokeWidth={2} aria-hidden="true" />
        </button>
        <button
          className="button"
          disabled={
            exporting ||
            !range.valid ||
            listing.refreshing.length > 0 ||
            events.length === 0
          }
          onClick={() => void exportCalendar()}
        >
          {exporting ? t.agenda.exporting : t.agenda.export}
        </button>
        <RefreshControl
          sites={sites}
          refreshed={listing.refreshed}
          busy={listing.refreshing.length > 0}
          onRefresh={() => void listing.refresh()}
        />
      </PageHeader>
      <div className="filter-bar">
        <SiteSelect sites={sites} value={siteId} onChange={setSiteId} />
        <button
          className="button small"
          aria-expanded={showDates}
          aria-controls="agenda-date-filters"
          onClick={() => {
            if (showDates) {
              setFrom("");
              setThrough("");
            }
            setShowDates(!showDates);
          }}
        >
          {showDates ? t.agenda.clearDates : t.agenda.dates}
        </button>
      </div>
      {showDates && (
        <div className="filter-bar" id="agenda-date-filters">
          <label className="date-filter">
            {t.agenda.from}
            <span className="text-field compact">
              <input
                type="date"
                value={from}
                onChange={(e) => setFrom(e.target.value)}
              />
            </span>
          </label>
          <label className="date-filter">
            {t.agenda.through}
            <span className="text-field compact">
              <input
                type="date"
                value={through}
                onChange={(e) => setThrough(e.target.value)}
              />
            </span>
          </label>
        </div>
      )}
      {!range.valid && (
        <p className="section-message error" role="alert">
          {t.agenda.invalidRange}
        </p>
      )}
      {exportError && (
        <p className="section-message error" role="alert">
          <ErrorText error={exportError} context="navigation" />
        </p>
      )}
      {exported && (
        <p className="section-message" role="status">
          {t.agenda.exported(exported.count, exported.filename)}{" "}
          {exported.revealed ? "" : t.agenda.revealFailed}
        </p>
      )}
      {failures.map((site) => (
        <p className="section-message error" role="alert" key={site.id}>
          <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
          <span className="error-text">
            {t.common.refreshFailed(site.name)}
            <span className="error-detail">
              <ErrorText error={listing.errors[site.id]} context="navigation" />
            </span>
          </span>
        </p>
      ))}
      {listing.loadError && (
        <p className="section-message error" role="alert">
          <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
          <ErrorText error={listing.loadError} context="navigation" />
        </p>
      )}
      {!listing.loaded ||
      (events.length === 0 && listing.refreshing.length > 0) ? (
        <Skeleton rows={5} />
      ) : (
        <AgendaList
          events={events}
          sites={sites}
          onOpen={(event) => onOpenDay(event.siteId, event.timeStart)}
        />
      )}
      <footer className="agenda-footer">
        <span>{t.agenda.timeZone(timeZone())}</span>
        <details className="help-disclosure">
          <summary>{t.agenda.help}</summary>
          <p>{t.agenda.exportHint}</p>
          <p>{t.agenda.footnote}</p>
        </details>
      </footer>
    </div>
  );
}
