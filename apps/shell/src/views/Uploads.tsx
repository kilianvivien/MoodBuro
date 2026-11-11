import { useEffect, useRef, useState } from "react";
import { CircleAlert, CircleCheck, LoaderCircle, Upload } from "lucide-react";
import {
  ipc,
  message,
  type Site,
  type Course,
  type UploadFile,
  type DraftAssignment,
  type UploadReview,
  type DraftUpload,
} from "../ipc";
import {
  Confirm,
  FileIcon,
  Notice,
  PageHeader,
  PopupButton,
} from "../components/ui";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";

export function UploadConfirmation({
  review,
  busy,
  onCancel,
  onConfirm,
  onStop,
}: {
  review: UploadReview;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
  onStop: () => void;
}) {
  const { t, locale } = useI18n();
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => cancel.current?.focus(), []);
  return (
    <div
      className="confirm standalone"
      role="alertdialog"
      aria-labelledby="upload-confirm-title"
      aria-describedby="upload-confirm-body"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          e.preventDefault();
          if (!busy) onCancel();
        }
      }}
    >
      <p className="confirm-title" id="upload-confirm-title">
        {t.uploads.confirmTitle(review.filename)}
      </p>
      <p className="confirm-body" id="upload-confirm-body">
        {review.organisation} · {review.course} · {review.assignment}
        {" · "}
        {t.offline.bytes(new Intl.NumberFormat(locale).format(review.bytes))}
      </p>
      <p className="confirm-body">{t.uploads.draftOnly}</p>
      <div className="confirm-actions">
        {busy ? (
          <button className="button" onClick={onStop}>
            {t.uploads.stop}
          </button>
        ) : (
          <button className="button" ref={cancel} onClick={onCancel}>
            {t.downloads.cancel}
          </button>
        )}
        <button className="button primary" disabled={busy} onClick={onConfirm}>
          {busy && (
            <LoaderCircle
              className="spin"
              size={14}
              strokeWidth={2}
              aria-hidden="true"
            />
          )}
          {busy ? t.uploads.uploading : t.uploads.confirm}
        </button>
      </div>
    </div>
  );
}
const historyIcons = {
  saved: CircleCheck,
  reviewed: CircleCheck,
  uncertain: CircleAlert,
  failed: CircleAlert,
} as const;

export function Uploads({
  sites,
  revision = 0,
  onOpened,
}: {
  sites: Site[];
  revision?: number;
  /** Moodle now shows an assignment of this organisation. */
  onOpened?: (siteId: string) => void;
}) {
  const { t, locale, date } = useI18n();
  const [courses, setCourses] = useState<Course[]>([]);
  const [siteId, setSiteId] = useState("");
  const [courseId, setCourseId] = useState("");
  const [assignmentId, setAssignmentId] = useState("");
  const [targets, setTargets] = useState<DraftAssignment[]>([]);
  const [file, setFile] = useState<UploadFile | null>(null);
  const [history, setHistory] = useState<DraftUpload[]>([]);
  const [review, setReview] = useState<UploadReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [ack, setAck] = useState<string | null>(null);
  const pending = useRef(false);
  const alive = useRef(true);
  const title = useRef<HTMLHeadingElement>(null);
  const sequence = useRef(0);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  useEffect(() => {
    let current = true;
    void Promise.all([ipc.courses(), ipc.uploadFiles(), ipc.uploadHistory()])
      .then(([courses, files, history]) => {
        if (current) {
          setCourses(courses.items);
          setHistory(history);
          if (!pending.current) {
            setFile(files.at(-1) ?? null);
            setReview(null);
            setSaved(false);
          }
        }
      })
      .catch((e) => {
        if (current) setError(message(e));
      });
    return () => {
      current = false;
    };
  }, [sites, revision]);
  const site = sites.find((s) => s.id === siteId);
  const chosen = targets.find((a) => a.id === Number(assignmentId));
  async function action(run: () => Promise<void>) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    try {
      await run();
    } catch (e) {
      if (alive.current) setError(message(e));
    } finally {
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  async function load(id: string) {
    const request = ++sequence.current;
    setCourseId(id);
    setAssignmentId("");
    setTargets([]);
    setReview(null);
    if (!site || !id) return;
    await action(async () => {
      const items = await ipc.uploadAssignments({
        siteId: site.id,
        userId: site.userId,
        courseId: Number(id),
      });
      if (alive.current && request === sequence.current) setTargets(items);
    });
  }
  const identity =
    site && courseId
      ? { siteId: site.id, userId: site.userId, courseId: Number(courseId) }
      : null;
  async function open(assignment = Number(assignmentId), target = identity) {
    if (target)
      await action(async () => {
        await ipc.uploadOpen(target, assignment);
        onOpened?.(target.siteId);
      });
  }
  async function confirm() {
    if (!review) return;
    await action(async () => {
      try {
        await ipc.uploadConfirm(review.id, true);
        if (alive.current) {
          setSaved(true);
          setReview(null);
          title.current?.focus();
        }
      } finally {
        if (alive.current) {
          setReview(null);
          title.current?.focus();
        }
        const rows = await ipc.uploadHistory();
        if (alive.current) setHistory(rows);
      }
    });
  }
  function cancel() {
    setReview(null);
    title.current?.focus();
  }
  const locked = busy || !!review;
  return (
    <div className="page uploads-page">
      <PageHeader title={t.uploads.title} />
      <h3 ref={title} tabIndex={-1} className="visually-hidden">
        {t.uploads.title}
      </h3>
      <section>
        <div className="group">
          <div className="group-row">
            {file ? (
              <FileIcon name={file.name} />
            ) : (
              <span className="row-icon" aria-hidden="true">
                <Upload size={16} strokeWidth={1.75} />
              </span>
            )}
            <span className="row-text">
              <span className="row-title">
                {file?.name ?? t.uploads.noFile}
              </span>
              <span className="row-subtitle">
                {file
                  ? t.offline.bytes(
                      new Intl.NumberFormat(locale).format(file.bytes),
                    )
                  : t.uploads.hint}
              </span>
            </span>
            <button
              className="button"
              disabled={locked}
              onClick={() =>
                void action(async () => {
                  const next = await ipc.uploadPick();
                  if (alive.current && next) {
                    setFile(next);
                    setSaved(false);
                  }
                })
              }
            >
              {t.uploads.chooseFile}
            </button>
          </div>
          <div className="group-row">
            <label className="row-text" htmlFor="upload-site">
              <span className="row-title">{t.uploads.organisation}</span>
            </label>
            <PopupButton
              id="upload-site"
              disabled={locked}
              value={siteId}
              onChange={(value) => {
                setSiteId(value);
                setCourseId("");
                setAssignmentId("");
                setTargets([]);
                setSaved(false);
                sequence.current++;
              }}
            >
              <option value="">{t.uploads.choose}</option>
              {sites.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </PopupButton>
          </div>
          <div className="group-row">
            <label className="row-text" htmlFor="upload-course">
              <span className="row-title">{t.uploads.course}</span>
            </label>
            <PopupButton
              id="upload-course"
              disabled={!site || locked}
              value={courseId}
              onChange={(value) => void load(value)}
            >
              <option value="">{t.uploads.choose}</option>
              {courses
                .filter((c) => c.siteId === siteId)
                .map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
            </PopupButton>
          </div>
          <div className="group-row">
            <label className="row-text" htmlFor="upload-assignment">
              <span className="row-title">{t.uploads.assignment}</span>
            </label>
            <PopupButton
              id="upload-assignment"
              disabled={!courseId || locked}
              value={assignmentId}
              onChange={(value) => {
                setAssignmentId(value);
                setSaved(false);
              }}
            >
              <option value="">{t.uploads.choose}</option>
              {targets.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </PopupButton>
          </div>
          <div className="group-row group-footer">
            <button
              className="button"
              disabled={!chosen || busy}
              onClick={() => void open()}
            >
              {t.uploads.openMoodle}
            </button>
            <button
              className="button primary"
              disabled={!identity || !file || !chosen?.supported || locked}
              onClick={() =>
                void action(async () => {
                  if (identity && file) {
                    const r = await ipc.uploadPrepare(
                      identity,
                      Number(assignmentId),
                      file.id,
                    );
                    if (alive.current) {
                      setReview(r);
                      setSaved(false);
                    }
                  }
                })
              }
            >
              {t.uploads.review}
            </button>
          </div>
        </div>
        {chosen && !chosen.supported && (
          <Notice>{t.uploads.unsupported}</Notice>
        )}
      </section>
      {review && (
        <UploadConfirmation
          review={review}
          busy={busy}
          onCancel={cancel}
          onConfirm={() => void confirm()}
          onStop={() => {
            void ipc.uploadStop(review.id).catch((e) => setError(message(e)));
          }}
        />
      )}
      {error && (
        <Notice tone="error">
          <ErrorText error={error} context="navigation" />
        </Notice>
      )}
      {saved && <Notice tone="success">{t.uploads.saved}</Notice>}
      {ack && (
        <Confirm
          id="upload-ack"
          title={t.uploads.newAttempt}
          body={t.uploads.acknowledge}
          cancel={t.downloads.cancel}
          confirm={t.uploads.checked}
          busy={busy}
          onCancel={() => {
            setAck(null);
            title.current?.focus();
          }}
          onConfirm={() =>
            void action(async () => {
              await ipc.uploadReviewed(ack, true);
              const rows = await ipc.uploadHistory();
              if (alive.current) {
                setHistory(rows);
                setAck(null);
              }
            })
          }
        />
      )}
      {history.length > 0 && (
        <section>
          <h3 className="section-title">{t.uploads.history}</h3>
          <ul className="group" role="list">
            {history.map((row) => {
              const Icon =
                historyIcons[row.state as keyof typeof historyIcons] ?? Upload;
              return (
                <li
                  className="list-row"
                  key={row.id}
                  data-state={
                    row.state === "failed" || row.state === "uncertain"
                      ? "failed"
                      : row.state === "saved" || row.state === "reviewed"
                        ? "complete"
                        : undefined
                  }
                >
                  <span className="row-icon" aria-hidden="true">
                    <Icon size={16} strokeWidth={1.75} />
                  </span>
                  <span className="row-text">
                    <span className="row-title" title={row.filename}>
                      {row.filename}
                    </span>
                    <span className="row-subtitle">
                      {t.uploads.states[row.state]} ·{" "}
                      {date(row.createdAt * 1000, {
                        dateStyle: "medium",
                        timeStyle: "short",
                      })}
                    </span>
                  </span>
                  <span className="row-actions">
                    {row.state === "uncertain" && (
                      <button
                        className="button small"
                        disabled={busy}
                        onClick={() => setAck(row.id)}
                      >
                        {t.uploads.newAttempt}
                      </button>
                    )}
                    <button
                      className="button small"
                      disabled={busy}
                      onClick={() =>
                        void open(row.assignmentId, {
                          siteId: row.siteId,
                          userId: row.userId,
                          courseId: row.courseId,
                        })
                      }
                    >
                      {t.uploads.openMoodle}
                    </button>
                  </span>
                </li>
              );
            })}
          </ul>
        </section>
      )}
    </div>
  );
}
