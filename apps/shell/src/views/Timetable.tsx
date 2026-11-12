import {
  CalendarDays,
  CalendarSync,
  ChevronLeft,
  ChevronRight,
  ListChecks,
  LoaderCircle,
  Share,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useI18n } from "../i18n/context";
import { message, type Site } from "../ipc";
import {
  timetable,
  type QuestionSession,
  type TimetableChoices,
  type TimetableOptions,
  type TimetableSource,
  type WeekSummary,
  type WeekView,
} from "../lib/timetable";
import {
  Confirm,
  EmptyState,
  Notice,
  PopupButton,
  Segmented,
} from "../components/ui";
import {
  addDays,
  mondayOf,
  WeekCalendar,
  WeekList,
} from "../components/timetable/WeekCalendar";
import {
  pendingUids,
  SessionDetails,
} from "../components/timetable/SessionDetails";

/**
 * A timetable to read (from a Moodle link, a download or a course file), or a
 * saved week to show with one session selected (from an assistant answer).
 */
export type TimetableTarget =
  { source: TimetableSource } | { show: { date: string; uid: string | null } };
export type TimetableRequest = {
  revision: number;
  siteId: string;
} & TimetableTarget;

/** The preview on screen when Agenda was left, to show again on return. */
let resumable: WeekView | null = null;

/** Today in Paris, where the school's wall times live. */
export function parisToday(now = new Date()): string {
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now);
}

function newRequestId() {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

type Phase =
  | { kind: "idle" }
  | { kind: "reading"; filename?: string }
  | {
      kind: "failed";
      error: string;
      source: TimetableSource;
      askMonday: boolean;
    };

/** Translate a timetable error code at render time. */
export function useTimetableError() {
  const { t } = useI18n();
  return (error: string) =>
    (t.timetable.errors as Record<string, string>)[error] ??
    // An unlisted code is still shown, so it can be reported.
    (/^[A-Z_]{3,40}$/.test(error)
      ? `${t.timetable.errors.unknown} (${error})`
      : t.timetable.errors.unknown);
}

export function Timetable({
  sites,
  request,
  onRequestHandled,
  onFindInMoodle,
}: {
  /** Organisations for which timetables are enabled (the INSP Moodle). */
  sites: Site[];
  request: TimetableRequest | null;
  onRequestHandled?: (request: TimetableRequest) => void;
  onFindInMoodle: (siteId: string) => void;
}) {
  const { t, locale } = useI18n();
  const errorText = useTimetableError();
  const [siteId, setSiteId] = useState(resumable?.siteId ?? sites[0]?.id ?? "");
  const site = sites.find((s) => s.id === siteId) ?? sites[0];
  const [monday, setMonday] = useState(() =>
    mondayOf(resumable?.weekStart ?? parisToday()),
  );
  const [weeks, setWeeks] = useState<WeekSummary[]>([]);
  // A preview survives a trip to Moodle (a session link) and back.
  const [week, setWeekState] = useState<WeekView | null>(() => resumable);
  const setWeek = (next: WeekView | null) => {
    resumable = next && next.mode !== "saved" ? next : null;
    setWeekState(next);
  };
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [busy, setBusy] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [calendarOn, setCalendarOn] = useState(false);
  // Which way the week slides in after Previous/Next.
  const [direction, setDirection] = useState<"previous" | "next" | "none">(
    "none",
  );
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [layout, setLayout] = useState<"grid" | "list">("grid");
  const [mondayInput, setMondayInput] = useState("");
  // Only the newest review answer is shown; older ones are ignored.
  const reviewTicket = useRef(0);
  const handled = useRef<TimetableRequest | null>(null);

  const draft = week && week.mode !== "saved" ? week : null;
  const savedHere = weeks.find((w) => mondayOf(w.weekStart) === monday) ?? null;
  const range = (start: string, end: string) => {
    const format = new Intl.DateTimeFormat(locale, {
      day: "numeric",
      month: "long",
      timeZone: "UTC",
    });
    return `${format.format(new Date(`${start}T00:00:00Z`))} – ${format.format(new Date(`${end}T00:00:00Z`))}`;
  };

  const reloadWeeks = useCallback(async () => {
    if (!site) return [];
    const list = await timetable.weeks(site.id, site.userId);
    setWeeks(list);
    return list;
  }, [site]);

  useEffect(() => {
    if (!site) return;
    let current = true;
    timetable
      .calendar(site.id, site.userId)
      .then((sync) => current && setCalendarOn(sync.enabled))
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [site]);

  // Saved weeks for the organisation; the shown week follows navigation.
  useEffect(() => {
    void reloadWeeks().catch((e) => setError(message(e)));
  }, [reloadWeeks]);

  useEffect(() => {
    if (draft || !site) return;
    if (!savedHere) {
      setWeek(null);
      return;
    }
    let current = true;
    timetable
      .week(site.id, site.userId, savedHere.weekKey)
      .then((view) => current && setWeek(view))
      .catch((e) => current && setError(message(e)));
    return () => {
      current = false;
    };
  }, [savedHere?.weekKey, savedHere?.revision, site?.id, draft === null]);

  async function prepare(
    source: TimetableSource,
    target = site,
    weekStart: string | null = null,
  ) {
    if (!target) return;
    setError(null);
    setNotice(null);
    setSelected(null);
    setPhase({ kind: "reading" });
    try {
      const view = await timetable.prepare(
        target.id,
        target.userId,
        source,
        weekStart,
      );
      setSiteId(target.id);
      setWeek(view);
      setMonday(mondayOf(view.weekStart));
      setPhase({ kind: "idle" });
      if (view.mode === "unchanged") setNotice(t.timetable.unchanged);
    } catch (e) {
      const code = message(e);
      setPhase({
        kind: "failed",
        error: code,
        source,
        askMonday: code === "WEEK_DATE_MISSING",
      });
    }
  }

  // A timetable chosen elsewhere (Moodle link, Downloads, course files).
  useEffect(() => {
    if (!request || request === handled.current) return;
    const target = sites.find((s) => s.id === request.siteId);
    if (!target) return;
    handled.current = request;
    // The native link token is single-use. Clear it in App as soon as the
    // request starts, so returning to Agenda cannot replay it after a remount.
    onRequestHandled?.(request);
    if ("show" in request) {
      // An unsaved preview on screen stays; it is never discarded from here.
      if (draft) return;
      setError(null);
      setNotice(null);
      setSiteId(target.id);
      setMonday(mondayOf(request.show.date));
      setSelected(request.show.uid);
      return;
    }
    void prepare(request.source, target);
  }, [request, sites]);

  async function review(choices: TimetableChoices, options: TimetableOptions) {
    if (!draft?.draftId || draft.draftRevision === null) return;
    const ticket = ++reviewTicket.current;
    setBusy(true);
    setError(null);
    try {
      const view = await timetable.review(
        draft.draftId,
        draft.draftRevision,
        choices,
        options,
      );
      if (ticket === reviewTicket.current) setWeek(view);
    } catch (e) {
      // The last valid preview stays.
      if (ticket === reviewTicket.current) setError(message(e));
    } finally {
      if (ticket === reviewTicket.current) setBusy(false);
    }
  }

  async function commit() {
    if (!draft?.draftId || draft.draftRevision === null || !site) return;
    if (draft.mode === "unchanged") {
      await discard();
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const saved = await timetable.commit(
        draft.draftId,
        draft.draftRevision,
        newRequestId(),
      );
      setWeek(saved);
      setNotice(t.timetable.saved);
      await reloadWeeks();
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }

  async function discard() {
    if (draft?.draftId) await timetable.discard(draft.draftId).catch(() => {});
    setWeek(null);
    setSelected(null);
    await reloadWeeks().catch(() => {});
  }

  async function edit() {
    if (!week || week.savedRevision === null || !site) return;
    setBusy(true);
    setError(null);
    try {
      setWeek(
        await timetable.edit(
          site.id,
          site.userId,
          week.weekKey,
          week.savedRevision,
        ),
      );
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }

  async function exportWeek(calendar: boolean) {
    if (!week || week.savedRevision === null || !site || exporting) return;
    setExporting(true);
    setError(null);
    setNotice(null);
    try {
      const done = await timetable.export(
        site.id,
        site.userId,
        week.weekKey,
        week.savedRevision,
        calendar,
      );
      setNotice(
        (done.opened
          ? t.timetable.openedInCalendar
          : t.timetable.exported(done.filename)) +
          (done.cancelled
            ? t.timetable.exportedCancelled(done.cancelled)
            : "") +
          (done.historySaved ? "" : t.timetable.exportHistoryFailed),
      );
    } catch (e) {
      setError(message(e));
    } finally {
      setExporting(false);
    }
  }

  async function syncCalendar() {
    if (!site || syncing) return;
    setSyncing(true);
    setError(null);
    setNotice(null);
    try {
      const done = await timetable.calendarEnable(site.id, site.userId);
      setCalendarOn(true);
      setNotice(t.timetable.syncDone(done.saved));
    } catch (e) {
      setError(message(e));
    } finally {
      setSyncing(false);
    }
  }

  async function remove() {
    if (!week || week.savedRevision === null || !site) return;
    setBusy(true);
    try {
      await timetable.remove(
        site.id,
        site.userId,
        week.weekKey,
        week.savedRevision,
      );
      setConfirmRemove(false);
      setWeek(null);
      setNotice(t.timetable.removed);
      await reloadWeeks();
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }

  const go = (target: string) => {
    if (draft) return;
    const next = mondayOf(target);
    setDirection(next < monday ? "previous" : next > monday ? "next" : "none");
    setSelected(null);
    setNotice(null);
    setMonday(next);
  };

  if (!site) return null;
  const sessions = week?.sessions ?? [];
  const pending = pendingUids(
    week?.questions ?? null,
    week?.choices ?? {
      groups: {},
      halfPromotion: {},
      rooms: {},
      times: {},
      sport: {},
    },
  );
  const selectedSession = sessions.find((s) => s.uid === selected) ?? null;
  const questionList: QuestionSession[] = week?.questions
    ? [
        ...week.questions.groups,
        ...week.questions.participation,
        ...week.questions.times,
        ...week.questions.lv1,
        ...week.questions.sport.flatMap((w) => w.sessions),
      ]
    : [];
  const excluded = questionList.filter(
    (q, index) =>
      !sessions.some((s) => s.uid === q.uid) &&
      questionList.findIndex((o) => o.uid === q.uid) === index,
  );
  const selectedQuestion = selectedSession
    ? null
    : (excluded.find((q) => q.uid === selected) ?? null);
  const primary =
    draft &&
    {
      new: t.timetable.add,
      update: t.timetable.update,
      edit: t.timetable.saveChanges,
      unchanged: t.timetable.showSaved,
      saved: "",
    }[draft.mode];
  const nextPending = () => {
    const uids = [
      ...pending,
      ...excluded.map((q) => q.uid).filter((uid) => pending.has(uid)),
    ];
    const after = uids.find((uid) => uid !== selected) ?? uids[0];
    if (after) setSelected(after);
  };
  const halfWeeks = draft?.questions?.halfPromotion ?? [];
  const today = parisToday();
  const shownRange = week
    ? range(week.weekStart, week.weekEnd)
    : range(monday, addDays(monday, 4));

  return (
    <div className="timetable">
      <div className="filter-bar week-toolbar">
        {sites.length > 1 && (
          <PopupButton
            label={t.timetable.organisation}
            value={site.id}
            disabled={Boolean(draft)}
            onChange={setSiteId}
          >
            {sites.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </PopupButton>
        )}
        <button
          className="button"
          disabled={Boolean(draft)}
          onClick={() => go(today)}
        >
          {t.timetable.today}
        </button>
        <span className="week-nav">
          <button
            className="button icon"
            title={t.timetable.previous}
            aria-label={t.timetable.previous}
            disabled={Boolean(draft)}
            onClick={() => go(addDays(monday, -7))}
          >
            <ChevronLeft size={14} strokeWidth={2} aria-hidden="true" />
          </button>
          <button
            className="button icon"
            title={t.timetable.next}
            aria-label={t.timetable.next}
            disabled={Boolean(draft)}
            onClick={() => go(addDays(monday, 7))}
          >
            <ChevronRight size={14} strokeWidth={2} aria-hidden="true" />
          </button>
        </span>
        <h2 className="week-title">{t.timetable.savedWeek(shownRange)}</h2>
        <PopupButton
          className="week-picker"
          label={t.timetable.savedWeeks}
          value={savedHere?.weekKey ?? ""}
          disabled={Boolean(draft) || weeks.length === 0}
          onChange={(key) => {
            const target = weeks.find((w) => w.weekKey === key);
            if (target) go(target.weekStart);
          }}
        >
          <option value="" disabled>
            {weeks.length ? t.timetable.savedWeeks : t.timetable.noSavedWeeks}
          </option>
          {weeks.map((w) => (
            <option key={w.weekKey} value={w.weekKey}>
              {t.timetable.savedWeek(range(w.weekStart, w.weekEnd))}
            </option>
          ))}
        </PopupButton>
        <Segmented
          name="timetable-layout"
          legend={t.timetable.layoutLegend}
          value={layout}
          options={[
            { value: "grid", label: t.timetable.week },
            { value: "list", label: t.timetable.list },
          ]}
          onChange={setLayout}
        />
      </div>

      {phase.kind === "reading" && (
        <p className="section-message info" role="status">
          <LoaderCircle
            size={14}
            strokeWidth={2}
            className="spin"
            aria-hidden="true"
          />
          {t.timetable.reading}
        </p>
      )}
      {phase.kind === "failed" && (
        <div className="section-message error" role="alert">
          <span className="error-text">
            {errorText(phase.error)}
            {phase.askMonday && (
              <span className="error-detail">
                {t.timetable.weekStartPrompt}
              </span>
            )}
          </span>
          {phase.askMonday && (
            <span className="text-field compact">
              <input
                type="date"
                aria-label={t.timetable.weekStartPrompt}
                value={mondayInput}
                onChange={(e) => setMondayInput(e.target.value)}
              />
            </span>
          )}
          <button
            className="button small"
            disabled={phase.askMonday && !mondayInput}
            onClick={() =>
              void prepare(
                phase.source,
                site,
                phase.askMonday ? mondayOf(mondayInput) : null,
              )
            }
          >
            {phase.askMonday ? t.timetable.readWithDate : t.timetable.retry}
          </button>
          <button
            className="button small"
            onClick={() => onFindInMoodle(site.id)}
          >
            {t.timetable.backToMoodle}
          </button>
        </div>
      )}
      {error && <Notice tone="error">{errorText(error)}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}
      {draft?.choicesReset && <Notice>{t.timetable.choicesReset}</Notice>}

      {draft && (
        <div
          className="week-review"
          role="region"
          aria-label={t.timetable.preview}
        >
          <span className="week-review-text">
            <span className="row-title">{t.timetable.preview}</span>
            <span className="row-subtitle">
              {[
                t.timetable.source(draft.filename),
                draft.diff &&
                  draft.mode !== "new" &&
                  t.timetable.diff(
                    draft.diff.moved.length,
                    draft.diff.added.length,
                    draft.diff.removed.length,
                    draft.diff.changed.length,
                  ),
              ]
                .filter(Boolean)
                .join(" · ")}
            </span>
          </span>
          <button
            className="button"
            disabled={busy}
            onClick={() => void discard()}
          >
            {t.timetable.discard}
          </button>
          <button
            className="button primary"
            disabled={busy}
            onClick={() => void commit()}
          >
            {primary}
          </button>
          <div className="week-review-controls">
            <span className="week-control">
              <span className="field-label" aria-hidden="true">
                {t.timetable.cohort}
              </span>
              <PopupButton
                label={t.timetable.cohort}
                value={draft.options.cohort}
                disabled={busy}
                onChange={(cohort) =>
                  void review(draft.choices, {
                    ...draft.options,
                    cohort: cohort as TimetableOptions["cohort"],
                  })
                }
              >
                {(["ALL", "FI", "CIL", "OFF"] as const).map((c) => (
                  <option key={c} value={c}>
                    {t.timetable.cohorts[c]}
                  </option>
                ))}
              </PopupButton>
            </span>
            {halfWeeks.map((half) =>
              half.weekStart ? (
                <span className="week-control" key={half.weekStart}>
                  <span className="field-label" aria-hidden="true">
                    {t.timetable.session.halfPromotion}
                  </span>
                  <Segmented
                    key={half.weekStart}
                    name={`half-${half.weekStart}`}
                    legend={t.timetable.session.halfPromotion}
                    disabled={busy}
                    value={
                      draft.choices.halfPromotion[half.weekStart] ?? "none"
                    }
                    options={[
                      { value: "none", label: t.timetable.session.groupNone },
                      { value: "A", label: "A" },
                      { value: "B", label: "B" },
                    ]}
                    onChange={(value) => {
                      const next = { ...draft.choices.halfPromotion };
                      if (value === "none") delete next[half.weekStart!];
                      else next[half.weekStart!] = value as "A" | "B";
                      void review(
                        { ...draft.choices, halfPromotion: next },
                        draft.options,
                      );
                    }}
                  />
                </span>
              ) : null,
            )}
            {pending.size > 0 && (
              <button className="button" onClick={nextPending}>
                <ListChecks size={13} strokeWidth={2} aria-hidden="true" />
                {t.timetable.review(pending.size)}
              </button>
            )}
          </div>
        </div>
      )}

      {week && !draft && (
        <div className="week-review">
          <span className="week-review-text">
            <span className="row-title">
              {[
                t.timetable.sessions(sessions.length),
                week.pending > 0
                  ? `${week.pending} ${t.timetable.tentative.toLowerCase()}`
                  : null,
              ]
                .filter(Boolean)
                .join(" · ")}
            </span>
            <span className="row-subtitle" title={week.filename}>
              {week.filename}
            </span>
          </span>
          <button
            className="button"
            disabled={busy}
            onClick={() => void edit()}
          >
            {t.timetable.edit}
          </button>
          <button
            className="button"
            disabled={busy || exporting}
            onClick={() => void exportWeek(false)}
          >
            <Share size={13} strokeWidth={2} aria-hidden="true" />
            {exporting ? t.timetable.exporting : t.timetable.exportIcs}
          </button>
          <button
            className="button icon"
            title={calendarOn ? t.timetable.synced : t.timetable.sync}
            aria-label={calendarOn ? t.timetable.synced : t.timetable.sync}
            aria-pressed={calendarOn}
            disabled={busy || syncing || calendarOn}
            onClick={() => void syncCalendar()}
          >
            {syncing ? (
              <LoaderCircle
                size={14}
                strokeWidth={2}
                className="spin"
                aria-hidden="true"
              />
            ) : (
              <CalendarSync size={14} strokeWidth={2} aria-hidden="true" />
            )}
          </button>
          <button
            className="button icon"
            title={t.timetable.remove}
            aria-label={t.timetable.remove}
            disabled={busy}
            onClick={() => setConfirmRemove(true)}
          >
            <Trash2 size={14} strokeWidth={2} aria-hidden="true" />
          </button>
        </div>
      )}
      {confirmRemove && week && (
        <Confirm
          id="timetable-remove"
          tone="destructive"
          title={t.timetable.removeTitle(range(week.weekStart, week.weekEnd))}
          body={t.timetable.removeBody}
          cancel={t.bookmarks.cancel}
          confirm={t.timetable.confirmRemove}
          busy={busy}
          onCancel={() => setConfirmRemove(false)}
          onConfirm={() => void remove()}
        />
      )}

      {!week && phase.kind !== "reading" ? (
        <div className="week-empty">
          <EmptyState
            icon={<CalendarDays size={22} strokeWidth={1.75} />}
            title={t.timetable.emptyTitle}
            hint={t.timetable.emptyHint}
          />
          <button className="button" onClick={() => onFindInMoodle(site.id)}>
            {t.timetable.findInMoodle}
          </button>
        </div>
      ) : week ? (
        <div className="week-layout" data-details={selected ? true : undefined}>
          <div
            className="week-main"
            aria-label={t.timetable.weekGrid(shownRange)}
            role="region"
          >
            {layout === "grid" ? (
              <div
                className="week-stage"
                key={`${week.weekKey}:${week.mode === "saved" ? "saved" : "draft"}`}
                data-direction={direction}
              >
                <WeekCalendar
                  monday={mondayOf(week.weekStart)}
                  sessions={sessions}
                  selected={selected}
                  pending={pending}
                  today={today}
                  onSelect={setSelected}
                />
              </div>
            ) : (
              <WeekList
                sessions={sessions}
                selected={selected}
                pending={pending}
                onSelect={setSelected}
              />
            )}
            {draft && excluded.length > 0 && (
              <section className="week-excluded">
                <h3 className="day-title">{t.timetable.session.excluded}</h3>
                <ul className="group" role="list">
                  {excluded.map((q) => (
                    <li key={q.uid} className="list-row">
                      <span className="row-text">
                        <span className="row-title">{q.title}</span>
                        <span className="row-subtitle">
                          {[q.date, q.start && `${q.start}–${q.end ?? ""}`]
                            .filter(Boolean)
                            .join(" · ")}
                        </span>
                      </span>
                      <button
                        className="button small"
                        onClick={() => setSelected(q.uid)}
                      >
                        {t.common.details}
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            )}
            <footer className="agenda-footer">
              <span>{t.timetable.timeZone}</span>
              <details className="help-disclosure">
                <summary>{t.common.details}</summary>
                <p>{t.timetable.exportHint}</p>
              </details>
            </footer>
          </div>
          {(selectedSession || selectedQuestion) && (
            <SessionDetails
              session={selectedSession}
              question={selectedQuestion}
              questions={draft?.questions ?? null}
              choices={week.choices}
              editable={Boolean(draft)}
              busy={busy}
              onChoices={(choices) => void review(choices, week.options)}
              onOpenLink={(index) =>
                void timetable
                  .openLink(
                    site.id,
                    site.userId,
                    week.weekKey,
                    draft?.draftId ?? null,
                    selected!,
                    index,
                  )
                  .catch((e) => setError(message(e)))
              }
              onClose={() => setSelected(null)}
            />
          )}
        </div>
      ) : null}
    </div>
  );
}
