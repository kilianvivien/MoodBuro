import { useEffect, useEffectEvent, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  CheckCheck,
  Check,
  CircleAlert,
  Bell as InboxIcon,
  MessagesSquare,
} from "lucide-react";
import {
  desktop,
  ipc,
  message,
  type InboxItem,
  type MoodlePage,
  type Site,
  type UnreadStatus,
} from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import { dayLabel, groupByDay, timeLabel } from "../lib/time";
import {
  EmptyState,
  PageHeader,
  Segmented,
  SiteSelect,
  Skeleton,
} from "../components/ui";

export type InboxFilter = "all" | "unread";
const key = (item: Pick<InboxItem, "siteId" | "id">) =>
  `${item.siteId}:${item.id}`;

export function InboxList({
  items,
  sites,
  statuses,
  filter,
  busy,
  now = new Date(),
  onOpen,
  onMarkRead,
}: {
  items: readonly InboxItem[];
  sites: readonly Site[];
  statuses: readonly UnreadStatus[];
  filter: InboxFilter;
  /** Keys (`site:id`) of notifications being marked read. */
  busy: readonly string[];
  now?: Date;
  onOpen: (siteId: string, page: MoodlePage) => void;
  onMarkRead: (item: InboxItem) => void;
}) {
  const { t, locale } = useI18n();
  const name = (id: string) => sites.find((site) => site.id === id)?.name;
  const visible = filter === "unread" ? items.filter((n) => !n.read) : items;
  const conversations = statuses.filter(
    (status) => (status.messages ?? 0) > 0 && name(status.siteId),
  );
  return (
    <>
      {conversations.length > 0 && (
        <ul className="group" role="list">
          {conversations.map((status) => (
            <li className="list-row" key={status.siteId}>
              <span className="row-icon" aria-hidden="true">
                <MessagesSquare size={16} strokeWidth={1.75} />
              </span>
              <span className="row-text">
                <span className="row-title">
                  {t.inbox.messages(status.messages ?? 0, name(status.siteId)!)}
                </span>
                <span className="row-subtitle">{t.inbox.messagesHint}</span>
              </span>
              <button
                className="button small"
                onClick={() => onOpen(status.siteId, "messages")}
              >
                {t.inbox.openMessages}
              </button>
            </li>
          ))}
        </ul>
      )}
      {visible.length === 0 ? (
        <EmptyState
          icon={
            filter === "unread" && items.length > 0 ? (
              <Check size={22} strokeWidth={1.75} />
            ) : (
              <InboxIcon size={22} strokeWidth={1.75} />
            )
          }
          title={
            filter === "unread" && items.length > 0
              ? t.inbox.caughtUp
              : t.inbox.empty
          }
          hint={
            filter === "unread" && items.length > 0
              ? t.inbox.caughtUpHint
              : t.inbox.emptyHint
          }
        />
      ) : (
        groupByDay(visible, (item) => item.timeCreated, now).map((group) => (
          <section key={group.offset} className="day-section">
            <h3 className="day-title">{dayLabel(locale, group.date, now)}</h3>
            <ul className="group" role="list">
              {group.items.map((item) => (
                <li
                  className="list-row"
                  key={key(item)}
                  data-unread={!item.read || undefined}
                >
                  <span className="unread-dot" aria-hidden="true" />
                  <span className="row-text">
                    <span className="row-title" title={item.subject}>
                      {!item.read && (
                        <span className="visually-hidden">
                          {t.inbox.unreadState} —{" "}
                        </span>
                      )}
                      {item.subject}
                    </span>
                    <span className="row-subtitle">
                      {[
                        name(item.siteId),
                        timeLabel(locale, new Date(item.timeCreated * 1000)),
                      ]
                        .filter(Boolean)
                        .join(" · ")}
                    </span>
                  </span>
                  <span className="row-actions">
                    {!item.read && (
                      <button
                        className="button small icon"
                        title={t.inbox.markRead}
                        aria-label={t.inbox.markReadLabel(item.subject)}
                        disabled={busy.includes(key(item))}
                        onClick={() => onMarkRead(item)}
                      >
                        <Check size={13} strokeWidth={2} aria-hidden="true" />
                      </button>
                    )}
                    <button
                      className="button small"
                      aria-label={t.inbox.openLabel(item.subject)}
                      onClick={() => onOpen(item.siteId, "notifications")}
                    >
                      {t.inbox.open}
                    </button>
                  </span>
                </li>
              ))}
            </ul>
          </section>
        ))
      )}
    </>
  );
}

export function Inbox({
  sites,
  statuses,
  onOpenPage,
  onChanged,
}: {
  sites: Site[];
  statuses: UnreadStatus[];
  onOpenPage: (siteId: string, page: MoodlePage) => void;
  /** Read state changed; the caller re-reads the unread counts. */
  onChanged?: () => void;
}) {
  const { t } = useI18n();
  const [items, setItems] = useState<InboxItem[] | null>(null);
  const [filter, setFilter] = useState<InboxFilter>("all");
  const [siteId, setSiteId] = useState<string | null>(null);
  const [busy, setBusy] = useState<string[]>([]);
  const [markingAll, setMarkingAll] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useEffectEvent(async (cancelled: () => boolean) => {
    try {
      const next = await ipc.inbox();
      if (!cancelled()) setItems(next);
    } catch (cause) {
      if (!cancelled()) {
        setError(message(cause));
        setItems((current) => current ?? []);
      }
    }
  });
  useEffect(() => {
    let cancelled = false;
    const refresh = () => void load(() => cancelled);
    // Each poll publishes new counts; the cached list changed with them.
    const listener = desktop
      ? listen("unread://changed", refresh)
      : Promise.resolve(() => {});
    refresh();
    return () => {
      cancelled = true;
      void listener.then((stop) => stop()).catch(() => {});
    };
  }, []);

  const scoped = (items ?? []).filter(
    (item) =>
      sites.some((site) => site.id === item.siteId) &&
      (!siteId || item.siteId === siteId),
  );
  const unread = scoped.filter((item) => !item.read);
  // Moodle may hold unread notifications older than the rows cached here, so
  // Mark All as Read also follows the server's count for each organisation.
  const unreadSites = [
    ...new Set([
      ...unread.map((item) => item.siteId),
      ...statuses
        .filter(
          (status) =>
            sites.some((site) => site.id === status.siteId) &&
            (!siteId || status.siteId === siteId) &&
            status.unread - (status.messages ?? 0) > 0,
        )
        .map((status) => status.siteId),
    ]),
  ];

  /** Server first: the row changes only once Moodle accepted the request. */
  async function markRead(item: InboxItem) {
    setError(null);
    setBusy((current) => [...current, key(item)]);
    try {
      await ipc.markRead(item.siteId, item.id);
      onChanged?.();
      setItems(
        (current) =>
          current?.map((n) =>
            key(n) === key(item) ? { ...n, read: true } : n,
          ) ?? null,
      );
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy((current) => current.filter((k) => k !== key(item)));
    }
  }
  async function markAll() {
    setError(null);
    setMarkingAll(true);
    const targets = unreadSites;
    const results = await Promise.allSettled(
      targets.map((id) => ipc.markRead(id, null)),
    );
    const done = targets.filter((_, i) => results[i].status === "fulfilled");
    setItems(
      (current) =>
        current?.map((n) =>
          done.includes(n.siteId) ? { ...n, read: true } : n,
        ) ?? null,
    );
    if (done.length > 0) onChanged?.();
    const failure = results.find((r) => r.status === "rejected");
    if (failure) setError(message((failure as PromiseRejectedResult).reason));
    setMarkingAll(false);
  }

  return (
    <div className="page">
      <PageHeader
        title={t.inbox.title}
        detail={
          unread.length > 0 ? t.inbox.unreadCount(unread.length) : undefined
        }
      >
        <SiteSelect sites={sites} value={siteId} onChange={setSiteId} />
        <Segmented
          name="inbox-filter"
          legend={t.inbox.filter}
          value={filter}
          options={[
            { value: "all", label: t.inbox.all },
            { value: "unread", label: t.inbox.unread },
          ]}
          onChange={setFilter}
        />
        <button
          className="button"
          disabled={unreadSites.length === 0 || markingAll}
          onClick={() => void markAll()}
        >
          <CheckCheck size={13} strokeWidth={2} aria-hidden="true" />
          {t.inbox.markAll}
        </button>
      </PageHeader>
      {error && (
        <p className="section-message error" role="alert">
          <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
          <ErrorText error={error} context="navigation" />
        </p>
      )}
      {items === null ? (
        <Skeleton />
      ) : (
        <InboxList
          items={scoped}
          sites={sites}
          statuses={statuses.filter((s) => !siteId || s.siteId === siteId)}
          filter={filter}
          busy={busy}
          onOpen={onOpenPage}
          onMarkRead={(item) => void markRead(item)}
        />
      )}
      <details className="help-disclosure section-footnote">
        <summary>{t.common.details}</summary>
        <p>{t.inbox.footnote}</p>
      </details>
    </div>
  );
}
