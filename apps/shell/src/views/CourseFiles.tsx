import { Confirmation } from "./Downloads";
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ipc,
  desktop,
  message,
  type Course,
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
  busy,
  onToggle,
}: {
  items: OfflineResource[];
  selected: readonly string[];
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

export function SavedFiles({
  items,
  busy,
  onOpen,
  onRefresh,
}: {
  items: OfflineCopy[];
  busy: boolean;
  onOpen: (copy: OfflineCopy, reveal: boolean) => void;
  onRefresh: (key: string) => void;
}) {
  const { t, date } = useI18n();
  if (items.length === 0)
    return <p className="section-message">{t.offline.emptySaved}</p>;
  return (
    <ul className="group" role="list">
      {items.map((item) => (
        <li className="list-row" key={item.id}>
          <span className="row-text">
            <span className="row-title">{item.filename}</span>
            <span className="row-subtitle">
              {item.sectionName} ·{" "}
              {date(item.savedAt * 1000, {
                dateStyle: "medium",
                timeStyle: "short",
              })}
            </span>
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
          </span>
        </li>
      ))}
    </ul>
  );
}

export function CourseFiles({
  course,
  userId,
  onBack,
  onOpen,
}: {
  course: Course;
  userId: number;
  onBack: () => void;
  onOpen: () => void;
}) {
  const { t, locale } = useI18n();
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
  const [progress, setProgress] = useState<OfflineProgress | null>(null);
  const [confirming, setConfirming] = useState<OfflineCopy | null>(null);
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
        setSelected([]);
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
    try {
      const result = await ipc.offlineSave(identity, keys);
      if (alive.current) setOutcomes(result);
      const items = await ipc.offlineSaved(identity);
      if (alive.current) {
        setSaved(items);
        setSelected([]);
      }
    } catch (error) {
      if (alive.current) setError(message(error));
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
      <p className="section-footnote">{t.offline.hint}</p>
      {error && (
        <p className="section-message error" role="alert">
          <ErrorText error={error} context="navigation" />
        </p>
      )}
      {busy && (
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
      <section aria-labelledby="offline-saved">
        <h3 className="section-title" id="offline-saved">
          {t.offline.savedTitle}
        </h3>
        {loadingSaved ? (
          <p className="section-message">{t.offline.loading}</p>
        ) : (
          <SavedFiles
            items={saved}
            busy={busy}
            onOpen={(copy, reveal) => void open(copy, reveal)}
            onRefresh={(key) => void save([key])}
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
                      busy || selected.length === 0 || selected.length > 50
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
