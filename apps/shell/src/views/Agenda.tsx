import { CalendarDays, CircleAlert } from "lucide-react";
import { ipc, type AgendaEvent, type Site } from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import { dayLabel, groupByDay, timeLabel, timeZone } from "../lib/time";
import { useListing } from "../lib/useListing";
import { useState } from "react";
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

export function Agenda({
  sites,
  onOpenDay,
}: {
  sites: Site[];
  onOpenDay: (siteId: string, time: number) => void;
}) {
  const { t } = useI18n();
  const listing = useListing(sites, ipc.agenda, ipc.refreshAgenda);
  const [siteId, setSiteId] = useState<string | null>(null);
  const shownSites = sites.filter((site) => !siteId || site.id === siteId);
  const events = listing.items.filter((event) =>
    shownSites.some((site) => site.id === event.siteId),
  );
  const failures = sites.filter((site) => listing.errors[site.id]);
  return (
    <div className="page">
      <PageHeader
        title={t.agenda.title}
        detail={events.length > 0 ? t.agenda.count(events.length) : undefined}
      >
        <SiteSelect sites={sites} value={siteId} onChange={setSiteId} />
        <RefreshControl
          sites={sites}
          refreshed={listing.refreshed}
          busy={listing.refreshing.length > 0}
          onRefresh={() => void listing.refresh()}
        />
      </PageHeader>
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
      <p className="section-footnote">
        {t.agenda.timeZone(timeZone())} {t.agenda.footnote}
      </p>
    </div>
  );
}
