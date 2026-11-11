import { Confirmation } from "./Downloads";
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ArrowLeft,
  FolderOpen,
  HardDriveDownload,
  LoaderCircle,
  RefreshCw,
  Trash2,
} from "lucide-react";
import {
  ipc,
  desktop,
  message,
  type Course,
  type CourseSync,
  type OfflineCopy,
  type OfflineOutcome,
  type OfflineProgress,
  type OfflineResource,
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
  Skeleton,
} from "../components/ui";

/** Largest file the offline store accepts. */
const MAX_BYTES = 128 * 1024 * 1024;

export function ResourceFiles({
  items,
  selected,
  saved,
  scheduled = [],
  busy,
  onToggle,
}: {
  items: OfflineResource[];
  selected: readonly string[];
  saved: readonly OfflineCopy[];
  scheduled?: readonly string[];
  busy: boolean;
  onToggle: (key: string) => void;
}) {
  const { t, locale } = useI18n();
  return (
    <ul className="group" role="list">
      {items.map((item) => {
        const tooLarge = item.bytes !== null && item.bytes > MAX_BYTES;
        const copies = saved.filter((copy) => copy.resourceKey === item.key);
        return (
          <li key={item.key}>
            <label className="list-row choice-row">
              <input
                className="checkbox"
                type="checkbox"
                checked={selected.includes(item.key)}
                disabled={busy || tooLarge}
                onChange={() => onToggle(item.key)}
              />
              <FileIcon name={item.filename} />
              <span className="row-text">
                <span className="row-title" title={item.filename}>
                  {item.filename}
                </span>
                <span className="row-subtitle">
                  {[
                    item.sectionName,
                    item.activityName,
                    item.bytes === null
                      ? t.offline.unknownSize
                      : t.offline.bytes(
                          new Intl.NumberFormat(locale).format(item.bytes),
                        ),
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
                {tooLarge && (
                  <span className="row-subtitle warning">
                    {t.offline.tooLarge}
                  </span>
                )}
              </span>
              {scheduled.includes(item.key) && (
                <span className="tag" title={t.sync.selected}>
                  <RefreshCw size={11} strokeWidth={2} aria-hidden="true" />
                  <span className="visually-hidden">{t.sync.selected}</span>
                </span>
              )}
              <span
                className="tag"
                data-kind={
                  copies.some((copy) => copy.available) ? "saved" : undefined
                }
              >
                {copies.some((copy) => copy.available)
                  ? t.offline.available
                  : copies.length > 0
                    ? t.offline.copyMissing
                    : t.offline.notSaved}
              </span>
            </label>
          </li>
        );
      })}
    </ul>
  );
}

export function RemoveCopyConfirmation({
  copy,
  siteName,
  busy,
  onCancel,
  onConfirm,
}: {
  copy: OfflineCopy;
  siteName: string;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  return (
    <Confirm
      id="remove-copy"
      title={t.offline.removeTitle(copy.filename, siteName)}
      body={t.offline.removeBody}
      cancel={t.downloads.cancel}
      confirm={t.offline.confirmRemove}
      tone="destructive"
      busy={busy}
      onCancel={onCancel}
      onConfirm={onConfirm}
    />
  );
}

export function SavedFiles({
  items,
  busy,
  onOpen,
  onRefresh,
  onRemove,
  failedKeys = [],
}: {
  items: OfflineCopy[];
  busy: boolean;
  onOpen: (copy: OfflineCopy, reveal: boolean) => void;
  onRefresh: (key: string) => void;
  onRemove: (copy: OfflineCopy) => void;
  failedKeys?: readonly string[];
}) {
  const { t, date } = useI18n();
  if (items.length === 0)
    return (
      <EmptyState
        icon={<HardDriveDownload size={22} strokeWidth={1.75} />}
        title={t.offline.emptySavedTitle}
        hint={t.offline.emptySaved}
      />
    );
  return (
    <ul className="group offline-saved" role="list">
      {items.map((item) => (
        <li
          className="list-row"
          key={item.id}
          data-state={item.available ? undefined : "failed"}
        >
          <FileIcon name={item.filename} />
          <span className="row-text">
            <span className="row-title" title={item.filename}>
              {item.filename}
            </span>
            <span className="row-subtitle">
              {[
                item.sectionName,
                t.offline.lastRefreshed(
                  date(item.refreshedAt * 1000, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  }),
                ),
              ]
                .filter(Boolean)
                .join(" · ")}
            </span>
            {item.available && failedKeys.includes(item.resourceKey) && (
              <span className="row-subtitle warning">
                {t.offline.refreshFailed}
              </span>
            )}
            {!item.available && (
              <span className="row-subtitle warning">{t.offline.missing}</span>
            )}
          </span>
          <span className="row-actions">
            <button
              className="button small"
              disabled={busy || !item.available}
              aria-label={t.downloads.openLabel(item.filename)}
              onClick={() => onOpen(item, false)}
            >
              {t.downloads.open}
            </button>
            <button
              className="button small icon"
              disabled={busy || !item.available}
              title={t.downloads.reveal}
              aria-label={t.downloads.revealLabel(item.filename)}
              onClick={() => onOpen(item, true)}
            >
              <FolderOpen size={13} strokeWidth={2} aria-hidden="true" />
            </button>
            <button
              className="button small icon"
              disabled={busy}
              title={t.offline.refresh}
              aria-label={t.offline.refreshLabel(item.filename)}
              onClick={() => onRefresh(item.resourceKey)}
            >
              <RefreshCw size={13} strokeWidth={2} aria-hidden="true" />
            </button>
            <button
              className="button small icon"
              disabled={busy}
              title={t.offline.remove}
              aria-label={t.offline.removeLabel(item.filename)}
              onClick={() => onRemove(item)}
            >
              <Trash2 size={13} strokeWidth={2} aria-hidden="true" />
            </button>
          </span>
        </li>
      ))}
    </ul>
  );
}

export function CourseSyncControls({
  job,
  busy,
  selected,
  onConfigure,
  onControl,
}: {
  job: CourseSync | null;
  busy: boolean;
  selected: readonly string[];
  onConfigure: (keys: string[], interval: number) => void;
  onControl: (cancel: boolean) => void;
}) {
  const { t, date } = useI18n();
  const when = (seconds: number) =>
    date(seconds * 1000, { dateStyle: "medium", timeStyle: "short" });
  const active = job?.state === "running" || job?.state === "waiting";
  return (
    <section aria-labelledby="course-sync-title">
      <h3 className="section-title" id="course-sync-title">
        {t.sync.title}
      </h3>
      <div className="group">
        <div className="group-row">
          <span className="row-icon" aria-hidden="true">
            {job?.state === "running" ? (
              <LoaderCircle className="spin" size={16} strokeWidth={1.75} />
            ) : (
              <RefreshCw size={16} strokeWidth={1.75} />
            )}
          </span>
          <div className="row-text">
            <span className="row-title" role={job ? "status" : undefined}>
              {job ? t.sync.states[job.state] : t.sync.off}
            </span>
            <span className="row-subtitle">
              {job
                ? [
                    t.sync.count(job.keys.length, job.pending),
                    job.refreshedAt > 0 &&
                      t.offline.lastRefreshed(when(job.refreshedAt)),
                  ]
                    .filter(Boolean)
                    .join(" · ")
                : t.sync.summary}
            </span>
            {job && job.interval > 0 && job.state !== "running" && (
              <span className="row-subtitle">
                {t.sync.next(when(job.nextRun))}
              </span>
            )}
          </div>
          {job && job.interval > 0 && (
            <button
              className="button small"
              disabled={busy}
              onClick={() => onControl(active)}
            >
              {active ? t.sync.cancel : t.sync.now}
            </button>
          )}
        </div>
        {job && (
          <div className="group-row">
            <label className="row-text" htmlFor="course-sync-schedule">
              <span className="row-title">{t.sync.schedule}</span>
            </label>
            <PopupButton
              id="course-sync-schedule"
              disabled={busy}
              value={job.interval}
              onChange={(value) => onConfigure(job.keys, Number(value))}
            >
              <option value={0}>{t.sync.paused}</option>
              <option value={3600}>{t.sync.hourly}</option>
              <option value={86400}>{t.sync.daily}</option>
            </PopupButton>
          </div>
        )}
        <div className="group-row">
          <div className="row-text">
            <span className="row-subtitle">{t.sync.pick}</span>
          </div>
          <button
            className="button small"
            disabled={busy || selected.length === 0 || selected.length > 50}
            onClick={() => onConfigure([...selected], job?.interval || 86400)}
          >
            {job ? t.sync.replaceSelection : t.sync.enable}
          </button>
        </div>
      </div>
      {job && job.missing > 0 && <Notice>{t.sync.missing(job.missing)}</Notice>}
      {job?.error && (
        <Notice tone="error">
          <ErrorText error={job.error} context="navigation" />
        </Notice>
      )}
    </section>
  );
}

export function CourseFiles({
  course,
  siteName,
  userId,
  onBack,
  onOpen,
}: {
  course: Course;
  siteName: string;
  userId: number;
  onBack: () => void;
  onOpen: () => void;
}) {
  const { t, locale } = useI18n();
  const [sync, setSync] = useState<CourseSync | null>(null);
  const [saved, setSaved] = useState<OfflineCopy[]>([]);
  const [catalog, setCatalog] = useState<{
    items: OfflineResource[];
    skipped: number;
  } | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [loadingSaved, setLoadingSaved] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [outcomes, setOutcomes] = useState<OfflineOutcome[]>([]);
  const [failedKeys, setFailedKeys] = useState<string[]>([]);
  const [progress, setProgress] = useState<OfflineProgress | null>(null);
  const [confirming, setConfirming] = useState<OfflineCopy | null>(null);
  const [removing, setRemoving] = useState<OfflineCopy | null>(null);
  const [removalResult, setRemovalResult] = useState<
    "archived" | "kept" | "missing" | null
  >(null);
  const savedHeading = useRef<HTMLHeadingElement>(null);
  const syncRequest = useRef(0);
  const pending = useRef(false);
  const alive = useRef(true);
  const identity = { siteId: course.siteId, courseId: course.id, userId };
  useEffect(() => {
    alive.current = true;
    void ipc
      .offlineSaved({ siteId: course.siteId, courseId: course.id, userId })
      .then((items) => {
        if (alive.current) setSaved(items);
      })
      .catch((error) => {
        if (alive.current) setError(message(error));
      })
      .finally(() => {
        if (alive.current) setLoadingSaved(false);
      });
    async function refreshSync() {
      const request = ++syncRequest.current;
      try {
        const jobs = await ipc.courseSyncList();
        if (alive.current && request === syncRequest.current)
          setSync(
            jobs.find(
              (job) =>
                job.siteId === course.siteId &&
                job.courseId === course.id &&
                job.userId === userId,
            ) ?? null,
          );
      } catch (error) {
        if (alive.current) setError(message(error));
      }
    }
    void refreshSync();
    let stopSync: (() => void) | undefined;
    if (desktop)
      void listen("course-sync://changed", () => {
        void refreshSync();
        void ipc
          .offlineSaved({ siteId: course.siteId, courseId: course.id, userId })
          .then((items) => {
            if (alive.current) setSaved(items);
          })
          .catch((error) => {
            if (alive.current) setError(message(error));
          });
      })
        .then((stop) => {
          if (alive.current) stopSync = stop;
          else stop();
        })
        .catch(() => {});
    let unlisten: (() => void) | undefined;
    if (desktop)
      void listen<OfflineProgress>("offline://progress", ({ payload }) => {
        if (
          alive.current &&
          payload.siteId === course.siteId &&
          payload.courseId === course.id
        )
          setProgress(payload);
      })
        .then((stop) => {
          if (alive.current) unlisten = stop;
          else stop();
        })
        .catch(() => {});
    return () => {
      alive.current = false;
      unlisten?.();
      stopSync?.();
    };
  }, [course.siteId, course.id, userId]);
  async function loadResources() {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    setOutcomes([]);
    try {
      const next = await ipc.offlineResources(identity);
      if (alive.current) {
        setCatalog(next);
        setSelected(
          next.items
            .filter(
              (item) =>
                sync?.keys.includes(item.key) &&
                (item.bytes === null || item.bytes <= MAX_BYTES),
            )
            .map((item) => item.key),
        );
      }
    } catch (error) {
      if (alive.current) setError(message(error));
    } finally {
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  async function save(keys: string[]) {
    if (pending.current || keys.length === 0 || keys.length > 50) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    setOutcomes([]);
    setProgress(null);
    setFailedKeys([]);
    let transferred = false;
    try {
      const result = await ipc.offlineSave(identity, keys);
      transferred = true;
      if (alive.current) {
        setOutcomes(result);
        setFailedKeys(
          result
            .filter((outcome) => outcome.error)
            .map((outcome) => outcome.key),
        );
      }
      const items = await ipc.offlineSaved(identity);
      if (alive.current) {
        setSaved(items);
        setSelected([]);
      }
    } catch (error) {
      if (alive.current) {
        setError(message(error));
        if (!transferred) setFailedKeys(keys);
      }
    } finally {
      pending.current = false;
      if (alive.current) {
        setBusy(false);
        setProgress(null);
      }
    }
  }
  async function open(copy: OfflineCopy, reveal: boolean, confirmed = false) {
    if (!reveal && copy.needsConfirmation && !confirmed) {
      setConfirming(copy);
      return;
    }
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    try {
      await ipc.offlineOpen(identity, copy.id, reveal, confirmed);
    } catch (error) {
      if (alive.current) setError(message(error));
    } finally {
      pending.current = false;
      if (alive.current) {
        setBusy(false);
        setConfirming(null);
      }
    }
  }
  async function updateSync(action: () => Promise<void>) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    try {
      await action();
      const request = ++syncRequest.current;
      const jobs = await ipc.courseSyncList();
      if (alive.current && request === syncRequest.current)
        setSync(
          jobs.find(
            (job) =>
              job.siteId === course.siteId &&
              job.courseId === course.id &&
              job.userId === userId,
          ) ?? null,
        );
    } catch (error) {
      if (alive.current) setError(message(error));
    } finally {
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  function cancelRemoval() {
    setRemoving(null);
    savedHeading.current?.focus();
  }
  async function remove(copy: OfflineCopy) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    setRemovalResult(null);
    try {
      const result = await ipc.offlineRemove(identity, copy.id);
      if (alive.current) {
        setSaved((current) => current.filter((item) => item.id !== copy.id));
        setRemovalResult(result);
        setOutcomes([]);
        setRemoving(null);
        savedHeading.current?.focus();
      }
    } catch (error) {
      if (alive.current) setError(message(error));
    } finally {
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  const number = new Intl.NumberFormat(locale);
  const locked = busy || removing !== null || confirming !== null;
  return (
    <div className="page course-files">
      <PageHeader title={course.name} detail={t.offline.title}>
        <button className="button" onClick={onBack}>
          <ArrowLeft size={14} strokeWidth={2} aria-hidden="true" />
          {t.offline.back}
        </button>
        <button className="button" onClick={onOpen}>
          {t.offline.openCourse}
        </button>
      </PageHeader>
      {error && (
        <Notice tone="error">
          <ErrorText error={error} context="navigation" />
        </Notice>
      )}
      {(busy || sync?.state === "running") && (
        <p className="section-message" role="status">
          <LoaderCircle
            className="spin"
            size={14}
            strokeWidth={2}
            aria-hidden="true"
          />
          <span>
            {t.offline.working}{" "}
            {progress && t.offline.progress(number.format(progress.received))}
          </span>
        </p>
      )}
      {outcomes.length > 0 && (
        <ul className="group" aria-label={t.offline.results}>
          {outcomes.map((result) => {
            const name =
              catalog?.items.find((r) => r.key === result.key)?.filename ??
              saved.find((c) => c.resourceKey === result.key)?.filename ??
              t.offline.file;
            return (
              <li
                className="list-row"
                key={result.key}
                data-state={result.error ? "failed" : "complete"}
              >
                <FileIcon name={name} />
                <span className="row-text">
                  <span className="row-title">{name}</span>
                  {result.error ? (
                    <span className="row-subtitle warning" role="alert">
                      <ErrorText error={result.error} context="navigation" />
                    </span>
                  ) : (
                    <span className="row-subtitle" role="status">
                      {result.reused ? t.offline.unchanged : t.offline.saved}
                    </span>
                  )}
                </span>
              </li>
            );
          })}
        </ul>
      )}
      {confirming && (
        <Confirmation
          item={confirming}
          busy={busy}
          standalone
          onCancel={() => setConfirming(null)}
          onOpen={() => void open(confirming, false, true)}
        />
      )}
      {removalResult && (
        <Notice tone="success">{t.offline.removed[removalResult]}</Notice>
      )}
      {removing && (
        <RemoveCopyConfirmation
          copy={removing}
          siteName={siteName}
          busy={busy}
          onCancel={cancelRemoval}
          onConfirm={() => void remove(removing)}
        />
      )}
      <section aria-labelledby="offline-saved">
        <h3
          className="section-title"
          id="offline-saved"
          ref={savedHeading}
          tabIndex={-1}
        >
          {t.offline.savedTitle}
        </h3>
        {loadingSaved ? (
          <Skeleton rows={2} />
        ) : (
          <SavedFiles
            items={saved}
            busy={locked}
            onOpen={(copy, reveal) => void open(copy, reveal)}
            onRefresh={(key) => void save([key])}
            onRemove={(copy) => {
              setConfirming(null);
              setRemovalResult(null);
              setRemoving(copy);
            }}
            failedKeys={failedKeys}
          />
        )}
      </section>
      <section aria-labelledby="offline-available">
        <div className="section-header">
          <h3 className="section-title" id="offline-available">
            {t.offline.availableTitle}
          </h3>
          <button
            className="button small"
            disabled={busy}
            onClick={() => void loadResources()}
          >
            {t.offline.load}
          </button>
        </div>
        {catalog &&
          (catalog.items.length === 0 ? (
            <Notice>{t.offline.noResources}</Notice>
          ) : (
            <>
              <ResourceFiles
                items={catalog.items}
                selected={selected}
                saved={saved}
                scheduled={sync?.keys}
                busy={busy}
                onToggle={(key) =>
                  setSelected((current) =>
                    current.includes(key)
                      ? current.filter((k) => k !== key)
                      : [...current, key],
                  )
                }
              />
              <div className="list-actions">
                <span className="section-footnote">{t.offline.limitShort}</span>
                <button
                  className="button primary"
                  disabled={
                    busy ||
                    sync?.state === "running" ||
                    selected.length === 0 ||
                    selected.length > 50
                  }
                  onClick={() => void save(selected)}
                >
                  {t.offline.saveSelected(selected.length)}
                </button>
              </div>
            </>
          ))}
        {catalog && catalog.skipped > 0 && (
          <p className="section-footnote">
            {t.offline.skipped(catalog.skipped)}
          </p>
        )}
      </section>
      <CourseSyncControls
        job={sync}
        selected={selected}
        busy={locked}
        onConfigure={(keys, interval) =>
          void updateSync(() =>
            ipc.courseSyncConfigure(identity, keys, interval),
          )
        }
        onControl={(cancel) =>
          void updateSync(() => ipc.courseSyncControl(identity, cancel))
        }
      />
      <details className="help-disclosure section-footnote">
        <summary>{t.common.details}</summary>
        <p>{t.offline.hint}</p>
        <p>{t.offline.freshnessHint}</p>
        <p>{t.offline.limit}</p>
        <p>{t.sync.hint}</p>
        <p>{t.sync.selectionHint}</p>
      </details>
    </div>
  );
}
