import { useState } from "react";
import { BookOpen, CircleAlert, Pin, SearchX } from "lucide-react";
import { ipc, message, type Course, type Site } from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import { search } from "../lib/search";
import { initials } from "../lib/site";
import { useListing } from "../lib/useListing";
import {
  EmptyState,
  PageHeader,
  RefreshControl,
  SearchField,
  SiteSelect,
  Skeleton,
} from "../components/ui";

const key = (course: Pick<Course, "siteId" | "id">) =>
  `${course.siteId}:${course.id}`;

function CourseRows({
  courses,
  sites,
  showSite,
  onOpen,
  onPin,
}: {
  courses: readonly Course[];
  sites: readonly Site[];
  showSite: boolean;
  onOpen: (course: Course) => void;
  onPin: (course: Course) => void;
}) {
  const { t } = useI18n();
  return (
    <ul className="group" role="list">
      {courses.map((course) => {
        const subtitle = [
          course.shortName,
          showSite && sites.find((site) => site.id === course.siteId)?.name,
        ]
          .filter(Boolean)
          .join(" · ");
        return (
          <li className="list-row" key={key(course)}>
            <span className="row-avatar" aria-hidden="true">
              {initials(course.name)}
            </span>
            <span className="row-text">
              <span className="row-title" title={course.name}>
                {course.name}
              </span>
              {subtitle && <span className="row-subtitle">{subtitle}</span>}
            </span>
            <span className="row-actions">
              <button
                className="button small icon pin-toggle"
                title={course.pinned ? t.courses.unpin : t.courses.pin}
                aria-label={
                  course.pinned
                    ? t.courses.unpinLabel(course.name)
                    : t.courses.pinLabel(course.name)
                }
                aria-pressed={course.pinned}
                onClick={() => onPin(course)}
              >
                <Pin size={13} strokeWidth={2} aria-hidden="true" />
              </button>
              <button
                className="button small"
                aria-label={t.courses.openLabel(course.name)}
                onClick={() => onOpen(course)}
              >
                {t.courses.open}
              </button>
            </span>
          </li>
        );
      })}
    </ul>
  );
}

export function CourseList({
  courses,
  sites,
  query,
  onOpen,
  onPin,
}: {
  courses: readonly Course[];
  sites: readonly Site[];
  query: string;
  onOpen: (course: Course) => void;
  onPin: (course: Course) => void;
}) {
  const { t } = useI18n();
  const matches = search(courses, query, (course) => [
    course.name,
    course.shortName,
  ]);
  if (courses.length === 0)
    return (
      <EmptyState
        icon={<BookOpen size={22} strokeWidth={1.75} />}
        title={t.courses.empty}
        hint={t.courses.emptyHint}
      />
    );
  if (matches.length === 0)
    return (
      <EmptyState
        icon={<SearchX size={22} strokeWidth={1.75} />}
        title={t.courses.noMatch(query.trim())}
        hint={t.courses.noMatchHint}
      />
    );
  const rows = { sites, onOpen, onPin };
  // While searching, results are one ranked list across organisations.
  if (query.trim())
    return (
      <CourseRows courses={matches} showSite={sites.length > 1} {...rows} />
    );
  const pinned = matches.filter((course) => course.pinned);
  return (
    <>
      {pinned.length > 0 && (
        <section className="day-section" aria-labelledby="pinned-courses">
          <h3 className="day-title" id="pinned-courses">
            {t.courses.pinned}
          </h3>
          <CourseRows courses={pinned} showSite={sites.length > 1} {...rows} />
        </section>
      )}
      {sites.map((site) => {
        const own = matches.filter((course) => course.siteId === site.id);
        if (own.length === 0) return null;
        return (
          <section className="day-section" key={site.id}>
            <div className="section-header">
              <h3 className="day-title">{site.name}</h3>
              <span className="section-count">
                {t.courses.count(own.length)}
              </span>
            </div>
            <CourseRows courses={own} showSite={false} {...rows} />
          </section>
        );
      })}
    </>
  );
}

export function Courses({
  sites,
  onOpenCourse,
}: {
  sites: Site[];
  onOpenCourse: (siteId: string, courseId: number) => void;
}) {
  const { t } = useI18n();
  const listing = useListing(sites, ipc.courses, ipc.refreshCourses);
  const [query, setQuery] = useState("");
  const [siteId, setSiteId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Pins apply at once and roll back if saving fails.
  const [pending, setPending] = useState<Record<string, boolean>>({});

  const shownSites = sites.filter((site) => !siteId || site.id === siteId);
  const courses = listing.items
    .filter((course) => shownSites.some((site) => site.id === course.siteId))
    .map((course) =>
      key(course) in pending
        ? { ...course, pinned: pending[key(course)] }
        : course,
    );

  async function pin(course: Course) {
    const next = !course.pinned;
    setError(null);
    setPending((current) => ({ ...current, [key(course)]: next }));
    try {
      await ipc.pinCourse(course.siteId, course.id, next);
      await listing.reload();
    } catch (cause) {
      setError(message(cause));
    } finally {
      setPending((current) => {
        const rest = { ...current };
        delete rest[key(course)];
        return rest;
      });
    }
  }

  const failures = sites.filter((site) => listing.errors[site.id]);
  return (
    <div className="page">
      <PageHeader
        title={t.courses.title}
        detail={
          courses.length > 0 ? t.courses.count(courses.length) : undefined
        }
      >
        <RefreshControl
          sites={sites}
          refreshed={listing.refreshed}
          busy={listing.refreshing.length > 0}
          onRefresh={() => void listing.refresh()}
        />
      </PageHeader>
      <div className="filter-bar">
        <SearchField
          value={query}
          label={t.courses.searchLabel}
          placeholder={t.courses.searchPlaceholder}
          onChange={setQuery}
        />
        <SiteSelect sites={sites} value={siteId} onChange={setSiteId} />
      </div>
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
      {(error ?? listing.loadError) && (
        <p className="section-message error" role="alert">
          <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
          <ErrorText
            error={(error ?? listing.loadError)!}
            context="navigation"
          />
        </p>
      )}
      {!listing.loaded ||
      (courses.length === 0 && listing.refreshing.length > 0) ? (
        <Skeleton rows={5} />
      ) : (
        <CourseList
          courses={courses}
          sites={shownSites}
          query={query}
          onOpen={(course) => onOpenCourse(course.siteId, course.id)}
          onPin={(course) => void pin(course)}
        />
      )}
      <p className="section-footnote">{t.courses.footnote}</p>
    </div>
  );
}
