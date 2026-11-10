import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  BookOpen,
  Building2,
  CalendarDays,
  CornerDownLeft,
  Download,
  Bell,
  Pin,
  Settings,
} from "lucide-react";
import { ipc, message, type Activity, type Course, type Site } from "../ipc";
import { ErrorText } from "../i18n/ErrorText";
import { useI18n } from "../i18n/context";
import type { Messages } from "../i18n/messages";
import { search } from "../lib/search";
import { displayHost, initials } from "../lib/site";
import { useListing } from "../lib/useListing";
import { SearchField } from "./ui";

export type QuickView =
  "inbox" | "courses" | "agenda" | "downloads" | "settings";
export type QuickTarget =
  | { kind: "course"; siteId: string; courseId: number }
  | { kind: "activity"; siteId: string; activityId: number }
  | { kind: "site"; siteId: string }
  | { kind: "view"; view: QuickView };
export interface QuickResult {
  id: string;
  title: string;
  subtitle: string;
  pinned?: boolean;
  target: QuickTarget;
}

const MAX_RESULTS = 9;
const MAX_PINNED = 6;
const views: readonly QuickView[] = [
  "inbox",
  "courses",
  "agenda",
  "downloads",
  "settings",
];

/**
 * Local search over saved names: courses, activities, organisations and shell views.
 * With nothing typed, pinned courses lead, then organisations and views.
 */
export function quickOpenResults(
  query: string,
  sites: readonly Site[],
  courses: readonly Course[],
  t: Messages,
  activities: readonly Activity[] = [],
): QuickResult[] {
  const siteName = (id: string) => sites.find((site) => site.id === id)?.name;
  const all: QuickResult[] = [
    ...courses
      .filter((course) => siteName(course.siteId))
      .map((course) => ({
        id: `course:${course.siteId}:${course.id}`,
        title: course.name,
        subtitle: [course.shortName, siteName(course.siteId)]
          .filter(Boolean)
          .join(" · "),
        pinned: course.pinned,
        target: {
          kind: "course" as const,
          siteId: course.siteId,
          courseId: course.id,
        },
      })),
    ...activities
      .filter(
        (a) =>
          siteName(a.siteId) &&
          courses.some((c) => c.siteId === a.siteId && c.id === a.courseId),
      )
      .map((a) => ({
        id: `activity:${a.siteId}:${a.id}`,
        title: a.name,
        subtitle: [
          courses.find((c) => c.siteId === a.siteId && c.id === a.courseId)
            ?.name,
          siteName(a.siteId),
        ]
          .filter(Boolean)
          .join(" · "),
        target: {
          kind: "activity" as const,
          siteId: a.siteId,
          activityId: a.id,
        },
      })),
    ...sites.map((site) => ({
      id: `site:${site.id}`,
      title: site.name,
      subtitle: displayHost(site.wwwroot),
      target: { kind: "site" as const, siteId: site.id },
    })),
    ...views.map((view) => ({
      id: `view:${view}`,
      title: view === "settings" ? t.toolbar.settings : t.toolbar[view],
      subtitle: t.quickOpen.views,
      target: { kind: "view" as const, view },
    })),
  ];
  if (!query.trim())
    return [
      ...all.filter((item) => item.pinned).slice(0, MAX_PINNED),
      ...all.filter(
        (item) =>
          item.target.kind !== "course" && item.target.kind !== "activity",
      ),
    ];
  return search(all, query, (item) => [item.title, item.subtitle]).slice(
    0,
    MAX_RESULTS,
  );
}

const viewIcons: Record<QuickView, typeof Bell> = {
  inbox: Bell,
  courses: BookOpen,
  agenda: CalendarDays,
  downloads: Download,
  settings: Settings,
};
function ResultIcon({ result }: { result: QuickResult }): ReactNode {
  if (result.target.kind === "course")
    return <span className="row-avatar">{initials(result.title)}</span>;
  const Icon =
    result.target.kind === "activity"
      ? BookOpen
      : result.target.kind === "site"
        ? Building2
        : viewIcons[result.target.view];
  return (
    <span className="row-icon">
      <Icon size={16} strokeWidth={1.75} />
    </span>
  );
}

export function QuickOpenPanel({
  query,
  results,
  active,
  onQuery,
  onActive,
  onChoose,
  cacheNote,
  error,
}: {
  error?: string | null;
  cacheNote?: string;
  query: string;
  results: readonly QuickResult[];
  active: number;
  onQuery: (query: string) => void;
  onActive: (index: number) => void;
  onChoose: (result: QuickResult) => void;
}) {
  const { t } = useI18n();
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => input.current?.focus(), []);
  const current = results[active];
  const currentId = current?.id;
  // Keep the keyboard selection visible when the list is longer than the panel.
  useEffect(() => {
    if (currentId)
      document
        .getElementById(`quick-${currentId}`)
        ?.scrollIntoView({ block: "nearest" });
  }, [currentId]);
  return (
    <div
      className="quick-open"
      role="dialog"
      aria-modal="false"
      aria-label={t.quickOpen.label}
    >
      <SearchField
        inputRef={input}
        value={query}
        label={t.quickOpen.label}
        placeholder={t.quickOpen.placeholder}
        role="combobox"
        aria-autocomplete="list"
        aria-expanded={results.length > 0}
        aria-controls="quick-open-results"
        aria-activedescendant={current ? `quick-${current.id}` : undefined}
        onChange={onQuery}
        onKeyDown={(event) => {
          const last = results.length - 1;
          if (event.key === "ArrowDown")
            onActive(active >= last ? 0 : active + 1);
          else if (event.key === "ArrowUp")
            onActive(active <= 0 ? last : active - 1);
          else if (event.key === "Home" && results.length) onActive(0);
          else if (event.key === "End" && results.length) onActive(last);
          else if (event.key === "Enter" && current) onChoose(current);
          else return;
          event.preventDefault();
        }}
      />
      {results.length === 0 ? (
        <div className="quick-empty" role="status">
          <p className="empty-title">{t.quickOpen.empty}</p>
          <p className="empty-hint">{t.quickOpen.emptyHint}</p>
        </div>
      ) : (
        <ul
          className="quick-results"
          id="quick-open-results"
          role="listbox"
          aria-label={t.quickOpen.results}
        >
          {results.map((result, index) => (
            <li
              key={result.id}
              id={`quick-${result.id}`}
              role="option"
              aria-selected={index === active}
              className="quick-result"
              onMouseMove={() => index !== active && onActive(index)}
              onClick={() => onChoose(result)}
            >
              <ResultIcon result={result} />
              <span className="row-text">
                <span className="row-title">{result.title}</span>
                <span className="row-subtitle">{result.subtitle}</span>
              </span>
              {result.pinned && (
                <Pin
                  className="quick-pin"
                  size={12}
                  strokeWidth={2}
                  aria-label={t.quickOpen.pinned}
                />
              )}
              <CornerDownLeft
                className="quick-return"
                size={13}
                strokeWidth={2}
                aria-hidden="true"
              />
            </li>
          ))}
        </ul>
      )}
      {error && (
        <p className="section-message error" role="alert">
          <ErrorText error={error} context="navigation" />
        </p>
      )}
      {cacheNote && (
        <p className="quick-hint" role="status">
          {cacheNote}
        </p>
      )}
      <p className="quick-hint">{t.quickOpen.hint}</p>
    </div>
  );
}

/** ⌘K palette. The current view stays visible underneath. */
export function QuickOpen({
  sites,
  onChoose,
  onClose,
}: {
  sites: Site[];
  onChoose: (target: QuickTarget) => void;
  onClose: () => void;
}) {
  const { t, date } = useI18n();
  const [activityError, setActivityError] = useState<string | null>(null);
  const [activities, setActivities] = useState<Activity[]>([]);
  useEffect(() => {
    let cancelled = false;
    void ipc
      .activities()
      .then((items) => {
        if (!cancelled) setActivities(items);
      })
      .catch((cause) => {
        if (!cancelled) setActivityError(message(cause));
      });
    return () => {
      cancelled = true;
    };
  }, []);
  const { items, refreshed, loadError } = useListing(
    sites,
    ipc.courses,
    ipc.refreshCourses,
  );
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const results = quickOpenResults(query, sites, items, t, activities);
  const oldest = [...refreshed, ...activities].reduce(
    (at, item) => Math.min(at, item.refreshedAt),
    Infinity,
  );
  const cacheNote = Number.isFinite(oldest)
    ? t.quickOpen.cacheDate(
        date(oldest * 1000, { dateStyle: "medium", timeStyle: "short" }),
      )
    : t.quickOpen.cacheEmpty;
  return (
    // Clicking outside the panel dismisses it, like a Spotlight window.
    <div
      className="quick-open-layer"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <QuickOpenPanel
        error={activityError ?? loadError}
        cacheNote={cacheNote}
        query={query}
        results={results}
        active={Math.min(active, Math.max(results.length - 1, 0))}
        onQuery={(next) => {
          setQuery(next);
          setActive(0);
        }}
        onActive={setActive}
        onChoose={(result) => onChoose(result.target)}
      />
    </div>
  );
}
