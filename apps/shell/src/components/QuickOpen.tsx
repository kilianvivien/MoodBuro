import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  BookOpen,
  Building2,
  CalendarDays,
  CornerDownLeft,
  Download,
  Inbox,
  Pin,
  Settings,
} from "lucide-react";
import { ipc, type Course, type Site } from "../ipc";
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
 * Local search over saved names only: courses, organisations and shell views.
 * With nothing typed, pinned courses lead, then organisations and views.
 */
export function quickOpenResults(
  query: string,
  sites: readonly Site[],
  courses: readonly Course[],
  t: Messages,
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
      ...all.filter((item) => item.target.kind !== "course"),
    ];
  return search(all, query, (item) => [item.title, item.subtitle]).slice(
    0,
    MAX_RESULTS,
  );
}

const viewIcons: Record<QuickView, typeof Inbox> = {
  inbox: Inbox,
  courses: BookOpen,
  agenda: CalendarDays,
  downloads: Download,
  settings: Settings,
};
function ResultIcon({ result }: { result: QuickResult }): ReactNode {
  if (result.target.kind === "course")
    return <span className="row-avatar">{initials(result.title)}</span>;
  const Icon =
    result.target.kind === "site" ? Building2 : viewIcons[result.target.view];
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
}: {
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
      <p className="quick-hint">{t.quickOpen.hint}</p>
    </div>
  );
}

/** ⌘L palette. The Moodle view is hidden by Rust while it is open. */
export function QuickOpen({
  sites,
  onChoose,
  onClose,
}: {
  sites: Site[];
  onChoose: (target: QuickTarget) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const { items } = useListing(sites, ipc.courses, ipc.refreshCourses);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const results = quickOpenResults(query, sites, items, t);
  return (
    // Clicking outside the panel dismisses it, like a Spotlight window.
    <div
      className="quick-open-layer"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <QuickOpenPanel
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
