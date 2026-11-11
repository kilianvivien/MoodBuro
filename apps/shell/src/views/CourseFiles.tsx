import { Confirmation } from "./Downloads";
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
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
import { PageHeader } from "../components/ui";

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
      {items.map((item) => (
        <li className="list-row" key={item.key}>
          <label className="offline-file-choice">
            <input
              type="checkbox"
              checked={selected.includes(item.key)}
              disabled={
                busy || (item.bytes !== null && item.bytes > 128 * 1024 * 1024)
              }
              onChange={() => onToggle(item.key)}
            />
            <span className="row-text">
              <span className="row-title">{item.filename}</span>
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
              <span className="row-subtitle">
                {saved.some(
                  (copy) => copy.resourceKey === item.key && copy.available,
                )
                  ? t.offline.available
                  : saved.some((copy) => copy.resourceKey === item.key)
                    ? t.offline.copyMissing
                    : t.offline.notSaved}
              </span>
              {scheduled.includes(item.key) && (
                <span className="row-subtitle">{t.sync.selected}</span>
              )}
              {item.bytes !== null && item.bytes > 128 * 1024 * 1024 && (
                <span className="row-subtitle">{t.offline.tooLarge}</span>
              )}
            </span>
          </label>
        </li>
      ))}
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
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => cancel.current?.focus(), []);
  return (
    <div
      className="confirm standalone"
      role="alertdialog"
      aria-labelledby="remove-copy-title"
      aria-describedby="remove-copy-body"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          event.preventDefault();
          if (!busy) onCancel();
        }
      }}
    >
      <p className="confirm-title" id="remove-copy-title">
        {t.offline.removeTitle(copy.filename, siteName)}
      </p>
      <p className="confirm-body" id="remove-copy-body">
        {t.offline.removeBody}
      </p>
      <div className="confirm-actions">
        <button
          className="button"
          ref={cancel}
          disabled={busy}
          onClick={onCancel}
        >
          {t.downloads.cancel}
        </button>
        <button className="button" disabled={busy} onClick={onConfirm}>
          {t.offline.confirmRemove}
        </button>
      </div>
    </div>
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
    return <p className="section-message">{t.offline.emptySaved}</p>;
  return (
    <ul className="group offline-saved" role="list">
      {items.map((item) => (
        <li className="list-row" key={item.id}>
          <span className="row-text">
            <span className="row-title">{item.filename}</span>
            <span className="row-subtitle">
              {item.sectionName} ·{" "}
              {t.offline.lastRefreshed(
                date(item.refreshedAt * 1000, {
                  dateStyle: "medium",
                  timeStyle: "short",
                }),
              )}
            </span>
            {item.available && (
              <span className="row-subtitle">{t.offline.available}</span>
            )}
            {item.available && failedKeys.includes(item.resourceKey) && (
              <span className="row-subtitle">{t.offline.refreshFailed}</span>
            )}
            {!item.available && (
              <span className="row-subtitle">{t.offline.missing}</span>
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
              className="button small"
              disabled={busy || !item.available}
              aria-label={t.downloads.revealLabel(item.filename)}
              onClick={() => onOpen(item, true)}
            >
              {t.downloads.reveal}
            </button>
            <button
              className="button small"
              disabled={busy}
              aria-label={t.offline.refreshLabel(item.filename)}
              onClick={() => onRefresh(item.resourceKey)}
            >
              {t.offline.refresh}
            </button>
            <button
              className="button small"
              disabled={busy}
              aria-label={t.offline.removeLabel(item.filename)}
              onClick={() => onRemove(item)}
            >
              {t.offline.remove}
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
  return (
    <section aria-labelledby="course-sync-title">
      <h3 className="section-title" id="course-sync-title">
        {t.sync.title}
      </h3>
      <p className="section-footnote">{t.sync.hint}</p>
      {job && (
        <>
          <p className="section-message" role="status">
            {t.sync.states[job.state]} ·{" "}
            {t.sync.count(job.keys.length, job.pending)}
          </p>
          {job.refreshedAt > 0 && (
            <p className="section-footnote">
              {t.offline.lastRefreshed(
                date(job.refreshedAt * 1000, {
                  dateStyle: "medium",
                  timeStyle: "short",
                }),
              )}
            </p>
          )}
          {job.interval > 0 && job.state !== "running" && (
            <p className="section-footnote">
              {t.sync.next(
                date(job.nextRun * 1000, {
                  dateStyle: "medium",
                  timeStyle: "short",
                }),
              )}
            </p>
          )}
          {job.missing > 0 && (
            <p className="section-message">{t.sync.missing(job.missing)}</p>
          )}
          {job.error && (
            <p className="section-message error" role="alert">
              <ErrorText error={job.error} context="navigation" />
            </p>
          )}
          <div className="filter-bar">
            <label>
              {t.sync.schedule}{" "}
              <select
                className="select"
                disabled={busy}
                value={job.interval}
                onChange={(event) =>
                  onConfigure(job.keys, Number(event.target.value))
                }
              >
                <option value={0}>{t.sync.paused}</option>
                <option value={3600}>{t.sync.hourly}</option>
                <option value={86400}>{t.sync.daily}</option>
              </select>
            </label>
            {job.interval > 0 && (
              <button
                className="button"
                disabled={busy}
                onClick={() =>
                  onControl(job.state === "running" || job.state === "waiting")
                }
              >
                {job.state === "running" || job.state === "waiting"
                  ? t.sync.cancel
                  : t.sync.now}
              </button>
            )}
          </div>
        </>
      )}
      <button
        className="button"
        disabled={busy || selected.length === 0 || selected.length > 50}
        onClick={() => onConfigure([...selected], job?.interval || 86400)}
      >
        {job ? t.sync.replaceSelection : t.sync.enable}
      </button>
      <p className="section-footnote">{t.sync.selectionHint}</p>
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
                (item.bytes === null || item.bytes <= 128 * 1024 * 1024),
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
  return (
    <div className="page">
      <PageHeader title={course.name} detail={t.offline.title}>
        <button className="button" onClick={onBack}>
          {t.offline.back}
        </button>
        <button className="button" onClick={onOpen}>
          {t.offline.openCourse}
        </button>
      </PageHeader>
      <details className="help-disclosure">
        <summary>{t.common.details}</summary>
        <p>{t.offline.hint}</p>
      </details>
      {error && (
        <p className="section-message error" role="alert">
          <ErrorText error={error} context="navigation" />
        </p>
      )}
      {(busy || sync?.state === "running") && (
        <p className="section-message" role="status">
          {t.offline.working}{" "}
          {progress &&
            t.offline.progress(
              new Intl.NumberFormat(locale).format(progress.received),
            )}
        </p>
      )}
      {outcomes.length > 0 && (
        <ul className="group" aria-label={t.offline.results}>
          {outcomes.map((result) => (
            <li className="group-row" key={result.key}>
              <span className="row-text">
                <span className="row-title">
                  {catalog?.items.find((r) => r.key === result.key)?.filename ??
                    saved.find((c) => c.resourceKey === result.key)?.filename ??
                    t.offline.file}
                </span>
                {result.error ? (
                  <span className="error" role="alert">
                    <ErrorText error={result.error} context="navigation" />
                  </span>
                ) : (
                  <span role="status">
                    {result.reused ? t.offline.unchanged : t.offline.saved}
                  </span>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
      {confirming && (
        <Confirmation
          item={confirming}
          busy={busy}
          onCancel={() => setConfirming(null)}
          onOpen={() => void open(confirming, false, true)}
        />
      )}
      {removalResult && (
        <p className="section-message" role="status">
          {t.offline.removed[removalResult]}
        </p>
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
      <CourseSyncControls
        job={sync}
        selected={selected}
        busy={busy || removing !== null || confirming !== null}
        onConfigure={(keys, interval) =>
          void updateSync(() =>
            ipc.courseSyncConfigure(identity, keys, interval),
          )
        }
        onControl={(cancel) =>
          void updateSync(() => ipc.courseSyncControl(identity, cancel))
        }
      />
      <section aria-labelledby="offline-saved">
        <h3
          className="section-title"
          id="offline-saved"
          ref={savedHeading}
          tabIndex={-1}
        >
          {t.offline.savedTitle}
        </h3>
        <p className="section-footnote">{t.offline.freshnessHint}</p>
        {loadingSaved ? (
          <p className="section-message">{t.offline.loading}</p>
        ) : (
          <SavedFiles
            items={saved}
            busy={busy || removing !== null || confirming !== null}
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
            className="button"
            disabled={busy}
            onClick={() => void loadResources()}
          >
            {t.offline.load}
          </button>
        </div>
        {catalog && (
          <>
            {catalog.skipped > 0 && (
              <p className="section-footnote">
                {t.offline.skipped(catalog.skipped)}
              </p>
            )}
            {catalog.items.length === 0 ? (
              <p className="section-message">{t.offline.noResources}</p>
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
                <div className="filter-bar">
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
                  <span className="section-footnote">{t.offline.limit}</span>
                </div>
              </>
            )}
          </>
        )}
      </section>
    </div>
  );
}
