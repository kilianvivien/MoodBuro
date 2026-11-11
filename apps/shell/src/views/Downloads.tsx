import { useEffect, useEffectEvent, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  CalendarDays,
  Copy,
  Download as DownloadIcon,
  Eye,
  FolderOpen,
  LoaderCircle,
  RefreshCw,
  SearchX,
  Trash2,
  TriangleAlert,
  Upload,
  XCircle,
} from "lucide-react";
import { looksLikeTimetable } from "../lib/timetable";
import {
  desktop,
  DOWNLOAD_PAGE_SIZE,
  ipc,
  message,
  type Course,
  type Download,
  type DownloadState,
  type DownloadProgress,
  type Site,
} from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import {
  Confirm,
  EmptyState,
  FileIcon,
  Notice,
  PageHeader,
  PopupButton,
  SearchField,
  SiteSelect,
  Skeleton,
} from "../components/ui";

export function Confirmation({
  item,
  busy,
  standalone = false,
  onCancel,
  onOpen,
}: {
  item: Pick<Download, "filename">;
  busy: boolean;
  /** Not under the row it concerns. */
  standalone?: boolean;
  onCancel: () => void;
  onOpen: () => void;
}) {
  const { t } = useI18n();
  return (
    <Confirm
      id="open-download"
      title={t.downloads.confirmTitle(item.filename)}
      body={t.downloads.confirmBody}
      cancel={t.downloads.cancel}
      confirm={t.downloads.confirm}
      nested={!standalone}
      busy={busy}
      onCancel={onCancel}
      onConfirm={onOpen}
    />
  );
}

/** Rows that are not a finished file show their state instead of its kind. */
const stateIcons = {
  downloading: LoaderCircle,
  failed: TriangleAlert,
  cancelled: XCircle,
  removed: Trash2,
} as const;

export function DownloadList({
  items,
  sites,
  busy,
  confirming,
  filtered = false,
  onOpen,
  onReveal,
  onCopy,
  onPreview,
  onReturn,
  onCancel,
  onConfirm,
  timetableSites = [],
  onTimetable,
  progress = [],
  onStop,
  onRemove,
}: {
  progress?: DownloadProgress[];
  onStop?: (id: string) => void;
  onRemove?: (item: Download) => void;
  items: Download[];
  sites: Site[];
  busy: boolean;
  confirming: string | null;
  /** A search or filter is active, so "nothing here" means "no match". */
  filtered?: boolean;
  onOpen: (item: Download) => void;
  onReveal: (id: string) => void;
  onCopy?: (id: string) => void;
  onPreview?: (id: string) => void;
  onReturn?: (item: Download) => void;
  onCancel: () => void;
  onConfirm: () => void;
  /** Organisations with timetables (the INSP Moodle). */
  timetableSites?: readonly string[];
  onTimetable?: (item: Download) => void;
}) {
  const { t, date, locale } = useI18n();
  if (items.length === 0)
    return filtered ? (
      <EmptyState
        icon={<SearchX size={22} strokeWidth={1.75} />}
        title={t.downloads.noMatch}
        hint={t.downloads.noMatchHint}
      />
    ) : (
      <EmptyState
        icon={<DownloadIcon size={22} strokeWidth={1.75} />}
        title={t.downloads.empty}
        hint={t.downloads.emptyHint}
      />
    );
  return (
    <ul className="group" role="list">
      {items.map((item) => {
        const Icon = item.state === "complete" ? null : stateIcons[item.state];
        const transfer = progress.find((p) => p.id === item.id);
        const number = new Intl.NumberFormat(locale);
        const unavailable = busy || item.state !== "complete";
        return (
          <li key={item.id}>
            <div className="list-row" data-state={item.state}>
              {Icon ? (
                <span className="row-icon" aria-hidden="true">
                  <Icon
                    className={
                      item.state === "downloading" ? "spin" : undefined
                    }
                    size={16}
                    strokeWidth={1.75}
                  />
                </span>
              ) : (
                <FileIcon name={item.filename} />
              )}
              <div className="row-text">
                <strong className="row-title" title={item.filename}>
                  {item.filename}
                </strong>
                <span className="row-subtitle">
                  {[
                    sites.find((s) => s.id === item.siteId)?.name,
                    item.courseName ?? t.downloads.unknownCourse,
                    t.downloads.states[item.state],
                    item.bytes != null
                      ? t.offline.bytes(number.format(item.bytes))
                      : null,
                    date(item.startedAt * 1000, {
                      dateStyle: "medium",
                      timeStyle: "short",
                    }),
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
                {item.state === "downloading" && transfer && (
                  <>
                    <span className="row-subtitle" role="status">
                      {t.downloads.progress(
                        number.format(transfer.received),
                        transfer.total == null
                          ? null
                          : number.format(transfer.total),
                      )}
                    </span>
                    <progress
                      aria-label={item.filename}
                      value={
                        transfer.total && transfer.total > 0
                          ? transfer.received
                          : undefined
                      }
                      max={transfer.total ?? undefined}
                    />
                  </>
                )}
                {item.metadataApplied === false && (
                  <span className="row-subtitle warning">
                    {t.downloads.metadataFailed}
                  </span>
                )}
              </div>
              <div className="row-actions">
                {item.state === "downloading" && transfer && onStop && (
                  <button
                    className="button small"
                    disabled={busy}
                    onClick={() => onStop(item.id)}
                  >
                    {t.downloads.stop}
                  </button>
                )}
                {(item.state === "failed" || item.state === "cancelled") &&
                  onReturn && (
                    <button
                      className="button small"
                      disabled={
                        busy || !sites.some((s) => s.id === item.siteId)
                      }
                      title={t.downloads.returnHint}
                      onClick={() => onReturn(item)}
                    >
                      {t.downloads.returnToMoodle}
                    </button>
                  )}
                {onTimetable &&
                  item.state === "complete" &&
                  timetableSites.includes(item.siteId) &&
                  looksLikeTimetable(item.filename) && (
                    <button
                      className="button icon small"
                      disabled={unavailable}
                      title={t.timetable.openAsTimetable}
                      aria-label={t.timetable.openAsTimetableLabel(
                        item.filename,
                      )}
                      onClick={() => onTimetable(item)}
                    >
                      <CalendarDays
                        size={13}
                        strokeWidth={2}
                        aria-hidden="true"
                      />
                    </button>
                  )}
                {item.canPreview && onPreview && (
                  <button
                    className="button small"
                    disabled={unavailable}
                    aria-label={t.downloads.previewLabel(item.filename)}
                    onClick={() => onPreview(item.id)}
                  >
                    <Eye size={13} strokeWidth={2} aria-hidden="true" />
                    {t.downloads.preview}
                  </button>
                )}
                <button
                  className="button small"
                  aria-label={t.downloads.openLabel(item.filename)}
                  disabled={unavailable}
                  onClick={() => onOpen(item)}
                >
                  {t.downloads.open}
                </button>
                <button
                  className="button small icon"
                  title={t.downloads.reveal}
                  aria-label={t.downloads.revealLabel(item.filename)}
                  disabled={
                    busy || !["complete", "removed"].includes(item.state)
                  }
                  onClick={() => onReveal(item.id)}
                >
                  <FolderOpen size={13} strokeWidth={2} aria-hidden="true" />
                </button>
                {onCopy && (
                  <button
                    className="button small icon"
                    title={t.downloads.copyPath}
                    aria-label={t.downloads.copyPathLabel(item.filename)}
                    disabled={unavailable}
                    onClick={() => onCopy(item.id)}
                  >
                    <Copy size={13} strokeWidth={2} aria-hidden="true" />
                  </button>
                )}
                {item.state === "complete" && onRemove && (
                  <button
                    className="button small icon"
                    title={t.downloads.removeFile}
                    aria-label={t.downloads.removeTitle(item.filename)}
                    disabled={busy}
                    onClick={() => onRemove(item)}
                  >
                    <Trash2 size={13} strokeWidth={2} aria-hidden="true" />
                  </button>
                )}
              </div>
            </div>
            {confirming === item.id && (
              <Confirmation
                item={item}
                busy={busy}
                onCancel={onCancel}
                onOpen={onConfirm}
              />
            )}
          </li>
        );
      })}
    </ul>
  );
}

export function RemoveConfirmation({
  item,
  busy,
  onCancel,
  onConfirm,
}: {
  item: Download;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  return (
    <Confirm
      id="remove-file"
      title={t.downloads.removeTitle(item.filename)}
      body={t.downloads.removeBody}
      cancel={t.downloads.cancel}
      confirm={t.downloads.confirmRemove}
      tone="destructive"
      busy={busy}
      onCancel={onCancel}
      onConfirm={onConfirm}
    />
  );
}

function ClearConfirmation({
  site,
  busy,
  onCancel,
  onConfirm,
}: {
  site: Site | undefined;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  return (
    <Confirm
      id="clear-downloads"
      title={
        site ? t.downloads.clearTitleFor(site.name) : t.downloads.clearTitle
      }
      body={t.downloads.clearBody}
      cancel={t.downloads.cancel}
      confirm={t.downloads.confirmClear}
      tone="destructive"
      busy={busy}
      onCancel={onCancel}
      onConfirm={onConfirm}
    />
  );
}

type StateFilter = "all" | DownloadState;

export function Downloads({
  sites,
  onReturn,
  onUpload,
  timetableSites = [],
  onTimetable,
}: {
  sites: Site[];
  onReturn?: (item: Download) => void;
  onUpload?: () => void;
  timetableSites?: readonly string[];
  onTimetable?: (item: Download) => void;
}) {
  const { t } = useI18n();
  const [revision, setRevision] = useState(0);
  const [items, setItems] = useState<Download[]>([]);
  const [total, setTotal] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const [query, setQuery] = useState("");
  const [siteId, setSiteId] = useState<string | null>(null);
  const [courses, setCourses] = useState<Course[]>([]);
  const [courseId, setCourseId] = useState<number | null>(null);
  const [state, setState] = useState<StateFilter>("all");
  const [pages, setPages] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [removing, setRemoving] = useState<Download | null>(null);
  const [progress, setProgress] = useState<DownloadProgress[]>([]);
  useEffect(() => {
    if (!desktop) return;
    const listener = listen<DownloadProgress[]>("downloads://progress", (e) =>
      setProgress(e.payload),
    );
    return () => {
      void listener.then((stop) => stop()).catch(() => {});
    };
  }, []);
  const [clearing, setClearing] = useState(false);
  const loadSequence = useRef(0);
  useEffect(() => {
    let cancelled = false;
    void ipc
      .courses()
      .then((result) => {
        if (!cancelled) setCourses(result.items);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [sites]);
  const returnFocus = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!confirming && !clearing && !removing && !busy && returnFocus.current) {
      returnFocus.current.focus();
      returnFocus.current = null;
    }
  }, [confirming, clearing, removing, busy]);
  const load = useEffectEvent(async (cancelled: () => boolean) => {
    const sequence = ++loadSequence.current;
    const current = () => !cancelled() && sequence === loadSequence.current;
    try {
      const next: Download[] = [];
      let count = 0;
      // Every page shown so far is reloaded, so a finished transfer updates in place.
      for (let page = 0; page < pages; page += 1) {
        const result = await ipc.downloads({
          query,
          siteId,
          courseId,
          state: state === "all" ? null : state,
          offset: page * DOWNLOAD_PAGE_SIZE,
        });
        next.push(...result.items);
        count = result.total;
        if (result.items.length < DOWNLOAD_PAGE_SIZE) break;
      }
      if (current()) {
        setItems(next);
        setTotal(count);
        setLoaded(true);
      }
    } catch (cause) {
      if (current()) {
        setError(message(cause));
        setLoaded(true);
      }
    }
  });
  useEffect(() => {
    let cancelled = false;
    const refresh = () => {
      void load(() => cancelled);
    };
    // Subscribe before taking the initial snapshot, so an in-flight finish is not missed.
    const listener = desktop
      ? listen("downloads://changed", refresh)
      : Promise.resolve(() => {});
    void listener
      .then(() => {
        if (!cancelled) refresh();
      })
      .catch((cause) => {
        if (!cancelled) {
          setError(message(cause));
          setLoaded(true);
        }
      });
    return () => {
      cancelled = true;
      void listener.then((stop) => stop()).catch(() => {});
    };
  }, [query, siteId, courseId, state, pages, revision]);
  async function action(run: () => Promise<void>, done: string | null = null) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await run();
      setConfirming(null);
      setClearing(false);
      setRemoving(null);
      setNotice(done);
      setRevision((n) => n + 1);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }
  const filter =
    <T,>(set: (value: T) => void) =>
    (value: T) => {
      setPages(1);
      setNotice(null);
      set(value);
    };
  const filtered =
    query.trim() !== "" ||
    siteId !== null ||
    courseId !== null ||
    state !== "all";
  return (
    <div className="page downloads-page">
      <PageHeader
        title={t.downloads.title}
        detail={
          loaded && total > items.length
            ? t.downloads.shown(items.length, total)
            : undefined
        }
      >
        {onUpload && (
          <button
            className="button"
            disabled={busy || sites.length === 0}
            onClick={onUpload}
          >
            <Upload size={14} strokeWidth={2} aria-hidden="true" />
            {t.uploads.title}
          </button>
        )}
        <button
          className="button"
          disabled={busy || sites.length === 0}
          onClick={() =>
            void action(async () => {
              for (const site of sites.filter(
                (s) => !siteId || s.id === siteId,
              ))
                await ipc.refreshCourseFolders(site.id);
              setCourses((await ipc.courses()).items);
            }, t.downloads.foldersRefreshed)
          }
        >
          <RefreshCw size={14} strokeWidth={2} aria-hidden="true" />
          {t.downloads.refreshFolders}
        </button>
        <button
          className="button"
          disabled={busy || clearing || !loaded || (total === 0 && !filtered)}
          onClick={(event) => {
            returnFocus.current = event.currentTarget;
            setNotice(null);
            setClearing(true);
          }}
        >
          {t.downloads.clear}
        </button>
      </PageHeader>
      <div className="filter-bar">
        <SearchField
          value={query}
          label={t.downloads.searchLabel}
          placeholder={t.downloads.searchPlaceholder}
          onChange={filter(setQuery)}
        />
        <SiteSelect
          sites={sites}
          value={siteId}
          onChange={(value) => {
            filter(setSiteId)(value);
            setCourseId(null);
          }}
        />
        <PopupButton
          label={t.downloads.courseFilter}
          value={courseId ?? "all"}
          onChange={(value) =>
            filter(setCourseId)(value === "all" ? null : Number(value))
          }
        >
          <option value="all">{t.downloads.allCourses}</option>
          <option value="0">{t.downloads.unknownCourse}</option>
          {siteId &&
            courses
              .filter((c) => c.siteId === siteId)
              .map((course) => (
                <option key={course.id} value={course.id}>
                  {course.name}
                </option>
              ))}
        </PopupButton>
        <PopupButton
          label={t.downloads.stateFilter}
          value={state}
          onChange={(value) => filter(setState)(value as StateFilter)}
        >
          <option value="all">{t.downloads.allStates}</option>
          {(
            [
              "downloading",
              "complete",
              "failed",
              "cancelled",
              "removed",
            ] as const
          ).map((value) => (
            <option key={value} value={value}>
              {t.downloads.filters[value]}
            </option>
          ))}
        </PopupButton>
      </div>
      {removing && (
        <RemoveConfirmation
          item={removing}
          busy={busy}
          onCancel={() => setRemoving(null)}
          onConfirm={() =>
            void action(() => ipc.removeDownloadFile(removing.id, true))
          }
        />
      )}
      {clearing && (
        <ClearConfirmation
          site={sites.find((site) => site.id === siteId)}
          busy={busy}
          onCancel={() => setClearing(false)}
          onConfirm={() =>
            void action(() => ipc.clearDownloads(siteId), t.downloads.cleared)
          }
        />
      )}
      {error && (
        <Notice tone="error">
          <ErrorText error={error} context="navigation" />
        </Notice>
      )}
      {notice && <Notice tone="success">{notice}</Notice>}
      {!loaded ? (
        <Skeleton />
      ) : (
        <DownloadList
          items={items}
          sites={sites}
          busy={busy}
          confirming={confirming}
          filtered={filtered}
          onReturn={onReturn}
          progress={progress}
          onStop={(id) => void action(() => ipc.cancelDownload(id))}
          onRemove={(item) => {
            returnFocus.current = document.activeElement as HTMLElement;
            setRemoving(item);
          }}
          onOpen={(item) => {
            if (item.needsConfirmation) {
              returnFocus.current = document.activeElement as HTMLElement;
              setConfirming(item.id);
            } else void action(() => ipc.openDownload(item.id));
          }}
          onPreview={(id) => void action(() => ipc.previewDownload(id))}
          timetableSites={timetableSites}
          onTimetable={onTimetable}
          onReveal={(id) => void action(() => ipc.revealDownload(id))}
          onCopy={(id) =>
            void action(() => ipc.copyDownloadPath(id), t.downloads.copied)
          }
          onCancel={() => setConfirming(null)}
          onConfirm={() => {
            if (confirming)
              void action(() => ipc.openDownload(confirming, true));
          }}
        />
      )}
      {loaded && total > items.length && (
        <div className="list-more">
          <button className="button" onClick={() => setPages((n) => n + 1)}>
            {t.downloads.showMore}
          </button>
        </div>
      )}
      <p className="section-footnote">{t.downloads.destination}</p>
      <details className="help-disclosure section-footnote">
        <summary>{t.common.details}</summary>
        <p>{t.downloads.folders}</p>
        <p>{t.downloads.metadataHint}</p>
        <p>{t.downloads.previewHint}</p>
        <p>{t.downloads.returnHint}</p>
      </details>
    </div>
  );
}
