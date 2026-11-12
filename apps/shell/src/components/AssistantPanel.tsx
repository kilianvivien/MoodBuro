import {
  useEffect,
  useEffectEvent,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ArrowUp,
  Bell,
  BookOpen,
  CalendarClock,
  CalendarDays,
  ChevronsUpDown,
  CircleAlert,
  File,
  FileText,
  LoaderCircle,
  Paperclip,
  Sparkles,
  Square,
  Trash2,
  X,
} from "lucide-react";
import {
  ASSISTANT_WIDTH,
  desktop,
  ipc,
  message,
  type AssistantConfirmation,
  type AssistantEntry,
  type AssistantProvider,
  type AssistantSource,
  type AssistantStep,
  type Course,
  type Site,
} from "../ipc";
import { useAssistant } from "../stores/assistant";
import {
  contextFrom,
  fileSize,
  noteText,
  providerNames,
  withoutMarkers,
  type ContextChoice,
} from "../lib/assistant";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import { AssistantText } from "./AssistantText";

const icon = { size: 14, strokeWidth: 2, "aria-hidden": true } as const;
/** Sources shown under an answer before "Show more". */
const SOURCES_SHOWN = 3;
const tool = { size: 16, strokeWidth: 1.75, "aria-hidden": true } as const;
const sourceIcons = {
  course: BookOpen,
  activity: FileText,
  deadline: CalendarClock,
  notification: Bell,
  file: File,
  session: CalendarDays,
} as const;
/** Arrow keys move the separator by this many points. */
const STEP = 16;

/** A saved timetable session an answer relied on: opens its week in Agenda. */
function SessionCard({
  source,
  onOpen,
}: {
  source: AssistantSource;
  onOpen: () => void;
}) {
  const { t, date } = useI18n();
  const card = source.session!;
  // Wall dates: format them as written, whatever the Mac's time zone.
  const day = (iso: string) =>
    date(new Date(`${iso}T00:00:00Z`), {
      weekday: "short",
      day: "numeric",
      month: "short",
      timeZone: "UTC",
    });
  const days =
    card.lastDate && card.lastDate !== card.date
      ? `${day(card.date)} – ${day(card.lastDate)}`
      : day(card.date);
  const hours =
    card.allDay || !card.start
      ? t.timetable.allDay
      : card.end
        ? `${card.start}–${card.end}`
        : card.start;
  const when = `${days} · ${hours}`;
  const details = [
    card.location,
    card.remote && t.timetable.session.remote,
    card.tentative && t.timetable.tentative,
  ].filter(Boolean) as string[];
  const action = t.assistant.openSession(source.label);
  return (
    <button
      className="assistant-session"
      data-tentative={card.tentative || undefined}
      title={action}
      aria-label={[action, when, ...details].join(", ")}
      onClick={onOpen}
    >
      <span className="assistant-session-when">{when}</span>
      <span className="assistant-session-title">{source.label}</span>
      {details.length > 0 && (
        <span className="assistant-session-place">{details.join(" · ")}</span>
      )}
    </button>
  );
}

function Message({
  entry,
  onSource,
}: {
  entry: AssistantEntry;
  onSource: (source: AssistantSource) => void;
}) {
  const { t, locale } = useI18n();
  const user = entry.role === "user";
  // Timetable sessions get cards of their own; everything else is a chip.
  const sessions = entry.sources.filter((source) => source.session);
  const chips = entry.sources.filter((source) => !source.session);
  // A long list of sources stays out of the way until asked for.
  const [all, setAll] = useState(false);
  const more = chips.length - SOURCES_SHOWN;
  // One hidden chip is not worth a control of the same size.
  const sources = all || more < 2 ? chips : chips.slice(0, SOURCES_SHOWN);
  return (
    <li className="assistant-message" data-role={entry.role}>
      <span className="visually-hidden">
        {user ? t.assistant.you : t.assistant.answer}
      </span>
      {entry.text &&
        (user ? (
          <p className="assistant-text">{entry.text}</p>
        ) : (
          <AssistantText text={entry.text} />
        ))}
      {entry.context && (
        <p className="assistant-attached">
          <Paperclip size={12} strokeWidth={2} aria-hidden="true" />
          <span>{t.assistant.attached(entry.context)}</span>
        </p>
      )}
      {entry.notes.length > 0 && (
        <ul className="assistant-notes" role="list">
          {entry.notes.map((note, index) => (
            <li key={index} data-stale={note.stale || undefined}>
              {noteText(t.assistant.notes, locale, note)}
            </li>
          ))}
        </ul>
      )}
      {sessions.length > 0 && (
        <ul
          className="assistant-sessions"
          role="list"
          aria-label={t.assistant.sessionCards}
        >
          {sessions.map((source) => (
            <li key={`session:${source.id}`}>
              <SessionCard source={source} onOpen={() => onSource(source)} />
            </li>
          ))}
        </ul>
      )}
      {chips.length > 0 && (
        <div className="assistant-sources">
          <span className="assistant-meta">{t.assistant.sources}</span>
          <ul role="list">
            {sources.map((source) => {
              const Icon = sourceIcons[source.kind];
              const action =
                source.kind === "file"
                  ? t.assistant.openFile(source.label)
                  : t.assistant.openSource(source.label);
              return (
                <li key={`${source.kind}:${source.id}`}>
                  <button
                    className="assistant-source"
                    title={action}
                    aria-label={action}
                    onClick={() => onSource(source)}
                  >
                    <Icon size={12} strokeWidth={2} aria-hidden="true" />
                    <span>{source.label}</span>
                  </button>
                </li>
              );
            })}
            {more >= 2 && (
              <li>
                <button
                  className="assistant-source more"
                  aria-expanded={all}
                  onClick={() => setAll(!all)}
                >
                  {all
                    ? t.assistant.fewerSources
                    : t.assistant.moreSources(more)}
                </button>
              </li>
            )}
          </ul>
        </div>
      )}
    </li>
  );
}

/** An action the model asked for: the exact target, and nothing happens without Yes. */
function WriteConfirmation({
  confirmation,
  onAnswer,
}: {
  confirmation: AssistantConfirmation;
  onAnswer: (approve: boolean) => void;
}) {
  const { t, locale } = useI18n();
  const decline = useRef<HTMLButtonElement>(null);
  useEffect(() => decline.current?.focus(), [confirmation.id]);
  const words = t.assistant.confirm[confirmation.kind];
  const bytes = Number(confirmation.detail);
  const size =
    confirmation.kind === "save-file" && confirmation.detail && bytes > 0
      ? fileSize(locale, bytes)
      : null;
  return (
    <div
      className="confirm standalone"
      role="alertdialog"
      aria-labelledby="assistant-confirm-title"
      aria-describedby="assistant-confirm-body"
    >
      <p id="assistant-confirm-title" className="confirm-title">
        {words.title(confirmation.detail)}
      </p>
      <div id="assistant-confirm-body">
        <p className="assistant-target">{confirmation.subject}</p>
        <p className="confirm-body">{words.body(confirmation.site, size)}</p>
      </div>
      <div className="confirm-actions">
        <button
          ref={decline}
          className="button"
          onClick={() => onAnswer(false)}
        >
          {words.decline}
        </button>
        <button className="button" onClick={() => onAnswer(true)}>
          {words.approve}
        </button>
      </div>
    </div>
  );
}

export interface AssistantViewProps {
  site: Site | undefined;
  provider: AssistantProvider;
  hasKey: boolean;
  consent: boolean;
  courses: readonly Course[];
  entries: readonly AssistantEntry[];
  /** Text of the answer in progress. */
  draft: string;
  busy: boolean;
  step: AssistantStep | null;
  /** Raw backend message; translated here. */
  error: string | null;
  canRetry: boolean;
  confirmation: AssistantConfirmation | null;
  clearing: boolean;
  context: ContextChoice;
  text: string;
  /** Ready-made questions for an empty conversation; sent with a context attached. */
  suggestions?: readonly string[];
  onSuggest?: (text: string) => void;
  /** The composer took focus: a good moment to look at what is on screen. */
  onActivate?: () => void;
  onText: (text: string) => void;
  onContext: (context: ContextChoice) => void;
  onSend: () => void;
  onStop: () => void;
  onRetry: () => void;
  onAnswer: (approve: boolean) => void;
  onAskClear: (asking: boolean) => void;
  onClear: () => void;
  onSource: (source: AssistantSource) => void;
  onSettings: () => void;
  onClose: () => void;
  input?: React.Ref<HTMLTextAreaElement>;
  log?: React.Ref<HTMLOListElement>;
}

/** Everything inside the panel, driven by props so each state can be rendered alone. */
export function AssistantView(props: AssistantViewProps) {
  const { t } = useI18n();
  const { site, busy, entries, confirmation } = props;
  const cancelClear = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (props.clearing) cancelClear.current?.focus();
  }, [props.clearing]);
  const draft = withoutMarkers(props.draft);
  const contextDisabled = busy || !site || !props.consent;
  // Why nothing can be attached, or how to send. Nothing once a context is attached:
  // what it shares is explained where sharing is turned on, in Settings.
  const hint = !site
    ? t.assistant.contextNeedsSite
    : !props.consent
      ? t.assistant.contextNeedsConsent
      : props.context === "none"
        ? t.assistant.inputHint
        : null;
  const contextName =
    props.context === "none" || !site
      ? t.assistant.contextNone
      : props.context === "site"
        ? t.assistant.contextSite(site.name)
        : (props.courses.find((c) => `course:${c.id}` === props.context)
            ?.name ?? t.assistant.contextNone);
  function keys(event: KeyboardEvent<HTMLElement>) {
    if (event.key !== "Escape") return;
    // Escape belongs to the panel: it never leaves the view behind it.
    event.preventDefault();
    event.stopPropagation();
    if (props.clearing) props.onAskClear(false);
    else if (confirmation) props.onAnswer(false);
    else if (busy) props.onStop();
    else props.onClose();
  }
  return (
    <div className="assistant-body" onKeyDown={keys}>
      <header className="assistant-header">
        <div className="assistant-heading">
          <h2>{t.assistant.title}</h2>
          <span title={site ? t.assistant.scope(site.name) : undefined}>
            {site ? site.name : t.assistant.general}
          </span>
        </div>
        <button
          className="tool"
          title={t.assistant.clear}
          aria-label={t.assistant.clear}
          disabled={entries.length === 0 || props.clearing}
          onClick={() => props.onAskClear(true)}
        >
          <Trash2 {...tool} />
        </button>
        <button
          className="tool"
          title={t.assistant.close}
          aria-label={t.assistant.close}
          onClick={props.onClose}
        >
          <X {...tool} />
        </button>
      </header>
      {props.clearing && (
        <div
          className="confirm standalone"
          role="alertdialog"
          aria-labelledby="assistant-clear-title"
          aria-describedby="assistant-clear-body"
        >
          <p id="assistant-clear-title" className="confirm-title">
            {t.assistant.clearTitle}
          </p>
          <p id="assistant-clear-body" className="confirm-body">
            {t.assistant.clearBody}
          </p>
          <div className="confirm-actions">
            <button
              ref={cancelClear}
              className="button"
              onClick={() => props.onAskClear(false)}
            >
              {t.assistant.cancel}
            </button>
            <button className="button destructive" onClick={props.onClear}>
              {t.assistant.clearConfirm}
            </button>
          </div>
        </div>
      )}
      {!props.hasKey ? (
        <div className="assistant-empty">
          <span className="empty-icon" aria-hidden="true">
            <Sparkles size={22} strokeWidth={1.75} />
          </span>
          <p className="empty-title">{t.assistant.setupTitle}</p>
          <p className="empty-hint">{t.assistant.setupHint}</p>
          <button className="button" onClick={props.onSettings}>
            {t.assistant.openSettings}
          </button>
        </div>
      ) : (
        <>
          <ol
            className="assistant-log"
            role="log"
            aria-label={t.assistant.log}
            aria-live="polite"
            ref={props.log}
            tabIndex={entries.length > 0 ? 0 : undefined}
          >
            {entries.length === 0 && !busy && (
              <li className="assistant-empty">
                <span className="empty-icon" aria-hidden="true">
                  <Sparkles size={22} strokeWidth={1.75} />
                </span>
                <p className="empty-title">{t.assistant.emptyTitle}</p>
                <p className="empty-hint">
                  {t.assistant.emptyHint(providerNames[props.provider])}
                </p>
                {props.suggestions && props.suggestions.length > 0 && (
                  <ul
                    className="assistant-suggestions"
                    role="list"
                    aria-label={t.assistant.suggestions}
                  >
                    {props.suggestions.map((suggestion) => (
                      <li key={suggestion}>
                        <button
                          className="assistant-suggestion"
                          onClick={() => props.onSuggest?.(suggestion)}
                        >
                          {suggestion}
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            )}
            {entries.map((entry, index) => (
              <Message key={index} entry={entry} onSource={props.onSource} />
            ))}
            {busy && (
              <li
                className="assistant-message"
                data-role="assistant"
                aria-busy="true"
              >
                {draft && <AssistantText text={draft} />}
                <p className="assistant-status" role="status">
                  <LoaderCircle className="spin" {...icon} />
                  {props.step
                    ? t.assistant.steps[props.step]
                    : t.assistant.thinking}
                </p>
              </li>
            )}
          </ol>
          {confirmation && (
            <WriteConfirmation
              confirmation={confirmation}
              onAnswer={props.onAnswer}
            />
          )}
          {props.error && !busy && (
            <div className="assistant-error">
              <p className="section-message error" role="alert">
                <CircleAlert {...icon} />
                <ErrorText error={props.error} context="assistant" />
              </p>
              {props.canRetry && (
                <button className="button small" onClick={props.onRetry}>
                  {t.assistant.retry}
                </button>
              )}
            </div>
          )}
          <form
            className="assistant-composer"
            onSubmit={(event) => {
              event.preventDefault();
              props.onSend();
            }}
          >
            <div className="assistant-field">
              <textarea
                ref={props.input}
                rows={1}
                maxLength={8000}
                value={props.text}
                placeholder={t.assistant.placeholder}
                aria-label={t.assistant.inputLabel}
                onFocus={props.onActivate}
                onChange={(event) => props.onText(event.target.value)}
                onKeyDown={(event) => {
                  // Return sends; Shift-Return and IME composition do not.
                  if (
                    event.key === "Enter" &&
                    !event.shiftKey &&
                    !event.nativeEvent.isComposing
                  ) {
                    event.preventDefault();
                    props.onSend();
                  }
                }}
              />
              <div className="assistant-field-bar">
                {/* The native select lies over its own label, so the control is
                    as wide as the current choice, not the longest course name. */}
                <label
                  className="assistant-context"
                  data-attached={props.context !== "none" || undefined}
                  data-disabled={contextDisabled || undefined}
                  title={contextName}
                >
                  <Paperclip size={13} strokeWidth={2} aria-hidden="true" />
                  <span aria-hidden="true">{contextName}</span>
                  <ChevronsUpDown
                    size={11}
                    strokeWidth={2}
                    aria-hidden="true"
                  />
                  <select
                    value={props.context}
                    disabled={contextDisabled}
                    aria-label={t.assistant.contextLabel}
                    aria-describedby={
                      hint ? "assistant-context-hint" : undefined
                    }
                    onChange={(event) =>
                      props.onContext(event.target.value as ContextChoice)
                    }
                  >
                    <option value="none">{t.assistant.contextNone}</option>
                    {site && props.consent && (
                      <>
                        <option value="site">
                          {t.assistant.contextSite(site.name)}
                        </option>
                        {props.courses.length > 0 && (
                          <optgroup label={t.assistant.contextCourses}>
                            {props.courses.map((course) => (
                              <option
                                key={course.id}
                                value={`course:${course.id}`}
                              >
                                {course.name}
                              </option>
                            ))}
                          </optgroup>
                        )}
                      </>
                    )}
                  </select>
                </label>
                {busy ? (
                  <button
                    type="button"
                    className="button icon"
                    title={t.assistant.stop}
                    aria-label={t.assistant.stop}
                    onClick={props.onStop}
                  >
                    <Square size={11} strokeWidth={2} aria-hidden="true" />
                  </button>
                ) : (
                  <button
                    type="submit"
                    className="button icon primary"
                    title={t.assistant.send}
                    aria-label={t.assistant.send}
                    disabled={props.text.trim() === ""}
                  >
                    <ArrowUp {...icon} />
                  </button>
                )}
              </div>
            </div>
            {hint && (
              <p className="assistant-meta" id="assistant-context-hint">
                {hint}
              </p>
            )}
          </form>
        </>
      )}
    </div>
  );
}

/** One account's conversation. Remounted (keyed) when the account changes. */
function Conversation({
  site,
  focus,
  timetable,
  onOpenPage,
  onTimetable,
  onSettings,
  onClose,
}: {
  site: Site | undefined;
  focus: number;
  timetable: boolean;
  onOpenPage: (source: AssistantSource, siteId: string) => void;
  onTimetable?: (siteId: string, date: string, uid: string | null) => void;
  onSettings: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const { provider, hasKey, consent } = useAssistant();
  const siteId = site?.id ?? null;
  const userId = site?.userId ?? null;
  const [entries, setEntries] = useState<AssistantEntry[]>([]);
  const [courses, setCourses] = useState<Course[]>([]);
  const [draft, setDraft] = useState("");
  const [step, setStep] = useState<AssistantStep | null>(null);
  const [request, setRequest] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmation, setConfirmation] =
    useState<AssistantConfirmation | null>(null);
  const [clearing, setClearing] = useState(false);
  const [context, setContext] = useState<ContextChoice>("none");
  const [text, setText] = useState("");
  const current = useRef<string | null>(null);
  // The user chose a context (even "none") in this conversation: it is never
  // replaced by the course on screen after that.
  const chosen = useRef(false);
  const alive = useRef(true);
  const input = useRef<HTMLTextAreaElement>(null);
  const log = useRef<HTMLOListElement>(null);

  const reload = useEffectEvent(async () => {
    try {
      const next = await ipc.assistantHistory(siteId, userId);
      if (alive.current) setEntries(next);
      return next;
    } catch (cause) {
      if (alive.current) setError(message(cause));
      return null;
    }
  });
  useEffect(() => {
    alive.current = true;
    void reload();
    const stops: (() => void)[] = [];
    let listening = true;
    const on = <T,>(event: string, handler: (payload: T) => void) => {
      void listen<T>(event, (e) => handler(e.payload)).then((stop) => {
        if (listening) stops.push(stop);
        else stop();
      });
    };
    if (desktop) {
      on<{ request: string; text: string }>("assistant://delta", (delta) => {
        if (delta.request === current.current)
          setDraft((draft) => draft + delta.text);
      });
      on<{ request: string; tool: AssistantStep }>(
        "assistant://step",
        (next) => {
          if (next.request !== current.current) return;
          // Text before a lookup is not the answer.
          setDraft("");
          setStep(next.tool);
        },
      );
      on<AssistantConfirmation>("assistant://confirm", (pending) => {
        if (pending.request === current.current) setConfirmation(pending);
      });
      // The account behind this panel changed, or every conversation was deleted.
      on("assistant://reset", () => {
        setConfirmation(null);
        chosen.current = false;
        setContext("none");
        void reload();
      });
    }
    return () => {
      alive.current = false;
      listening = false;
      for (const stop of stops) stop();
      // Nothing may keep running for a conversation that is no longer shown.
      if (current.current) void ipc.assistantCancel().catch(() => {});
    };
  }, []);
  useEffect(() => {
    if (!siteId || !consent) return;
    let wanted = true;
    void ipc
      .courses()
      .then(({ items }) => {
        if (wanted) setCourses(items.filter((c) => c.siteId === siteId));
      })
      .catch(() => {});
    return () => {
      wanted = false;
    };
  }, [siteId, consent]);
  useEffect(() => input.current?.focus(), [focus, hasKey]);
  /** Pre-selects the course shown in the Moodle view, until the user chooses. */
  const follow = useEffectEvent(async () => {
    if (!siteId || !consent || chosen.current || current.current) return;
    try {
      const id = await ipc.assistantCurrentCourse(siteId);
      if (!alive.current || chosen.current || current.current) return;
      setContext(
        id !== null && courses.some((course) => course.id === id)
          ? `course:${id}`
          : "none",
      );
    } catch {
      // Only a convenience: the pop-up still works.
    }
  });
  useEffect(() => {
    void follow();
  }, [focus, siteId, consent, courses]);
  useEffect(() => {
    const list = log.current;
    if (list) list.scrollTop = list.scrollHeight;
  }, [entries, draft, step, confirmation]);
  // Consent withdrawn in Settings: nothing stays attached.
  const attached = consent && site ? context : "none";

  async function run(
    retry: boolean,
    // A suggestion: its own text and context, not what the composer holds.
    preset?: { text: string; context: ContextChoice },
  ) {
    const prompt = (preset?.text ?? text).trim();
    if (current.current || (!retry && !prompt)) return;
    const id = crypto.randomUUID();
    current.current = id;
    setRequest(id);
    setError(null);
    setDraft("");
    setStep(null);
    if (!retry) {
      if (!preset) setText("");
      setEntries((entries) => [
        ...entries,
        { role: "user", text: prompt, context: null, sources: [], notes: [] },
      ]);
    }
    try {
      const next = await ipc.assistantSend({
        request: id,
        siteId,
        userId,
        text: retry ? null : prompt,
        context: contextFrom(preset?.context ?? attached),
        utcOffset: -new Date().getTimezoneOffset(),
        retry,
      });
      if (alive.current) setEntries(next);
    } catch (cause) {
      if (alive.current) {
        setError(message(cause));
        // Rust kept the question if it was accepted; show what it has, and
        // give back the text of a question it refused.
        const kept = await reload();
        const last = kept?.[kept.length - 1];
        if (!retry && alive.current && last?.role !== "user") setText(prompt);
      }
    } finally {
      current.current = null;
      if (alive.current) {
        setRequest(null);
        setDraft("");
        setStep(null);
        setConfirmation(null);
        input.current?.focus();
      }
    }
  }
  async function answer(approve: boolean) {
    const pending = confirmation;
    if (!pending) return;
    setConfirmation(null);
    try {
      await ipc.assistantConfirm(pending.id, approve);
    } catch (cause) {
      setError(message(cause));
    }
    input.current?.focus();
  }
  async function clear() {
    setClearing(false);
    try {
      await ipc.clearAssistant(siteId, userId);
      setEntries([]);
      setError(null);
    } catch (cause) {
      setError(message(cause));
    }
    input.current?.focus();
  }
  return (
    <AssistantView
      site={site}
      provider={provider}
      hasKey={hasKey}
      consent={consent}
      courses={courses}
      entries={entries}
      draft={draft}
      busy={request !== null}
      step={step}
      error={error}
      canRetry={entries[entries.length - 1]?.role === "user"}
      confirmation={confirmation}
      clearing={clearing}
      context={attached}
      text={text}
      suggestions={
        site && consent
          ? [
              t.assistant.suggest.due,
              timetable ? t.assistant.suggest.today : t.assistant.suggest.news,
              attached.startsWith("course:")
                ? t.assistant.suggest.files
                : t.assistant.suggest.courses,
            ]
          : []
      }
      onSuggest={(suggestion) => {
        // Moodle questions need a context: the course on screen, else the organisation.
        const context = attached === "none" ? "site" : attached;
        chosen.current = true;
        setContext(context);
        void run(false, { text: suggestion, context });
      }}
      onActivate={() => void follow()}
      input={input}
      log={log}
      onText={setText}
      onContext={(context) => {
        chosen.current = true;
        setContext(context);
      }}
      onSend={() => void run(false)}
      onStop={() => void ipc.assistantCancel().catch(() => {})}
      onRetry={() => void run(true)}
      onAnswer={(approve) => void answer(approve)}
      onAskClear={setClearing}
      onClear={() => void clear()}
      onSource={(source) => {
        if (!siteId) return;
        // A timetable session opens its saved week in Agenda, on this Mac.
        if (source.session)
          return onTimetable?.(siteId, source.session.date, source.key ?? null);
        if (
          source.kind !== "file" ||
          source.course === undefined ||
          !source.key ||
          userId === null
        )
          return onOpenPage(source, siteId);
        // A saved copy is shown on this Mac; otherwise the activity opens in Moodle.
        void ipc
          .assistantOpenFile(
            { siteId, courseId: source.course, userId },
            source.key,
          )
          .then((shown) => {
            if (!shown) onOpenPage(source, siteId);
          })
          .catch((cause: unknown) => setError(message(cause)));
      }}
      onSettings={onSettings}
      onClose={onClose}
    />
  );
}

/** The trailing panel beside the content area. Its width is owned by Rust. */
export function AssistantPanel({
  site,
  focus,
  closing = false,
  onClosed,
  timetable = false,
  onOpenPage,
  onTimetable,
  onSettings,
  onClose,
}: {
  site: Site | undefined;
  /** Bumped each time the panel is asked to take keyboard focus. */
  focus: number;
  /** The organisation has saved timetables the assistant can read. */
  timetable?: boolean;
  onTimetable?: (siteId: string, date: string, uid: string | null) => void;
  /** The panel is leaving: it plays its exit, then calls `onClosed` once. */
  closing?: boolean;
  onClosed?: () => void;
  onOpenPage: (source: AssistantSource, siteId: string) => void;
  onSettings: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const width = useAssistant((state) => state.panel.width);
  const setWidth = useAssistant((state) => state.setWidth);
  const drag = useRef<{ x: number; width: number; frame: number } | null>(null);
  const latest = useRef(width);
  const closed = useEffectEvent(() => onClosed?.());
  useEffect(() => {
    if (!closing) return;
    // With Reduce Motion there is no exit to wait for; otherwise the timer only
    // covers an animation that never reports its end.
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const timer = setTimeout(closed, still ? 0 : 400);
    return () => clearTimeout(timer);
  }, [closing]);
  function move(event: PointerEvent<HTMLDivElement>) {
    const start = drag.current;
    if (!start) return;
    // The panel is on the trailing edge: dragging left makes it wider.
    latest.current = start.width + (start.x - event.clientX);
    if (start.frame) return;
    start.frame = requestAnimationFrame(() => {
      start.frame = 0;
      void setWidth(latest.current, false);
    });
  }
  function release() {
    const start = drag.current;
    if (!start) return;
    drag.current = null;
    cancelAnimationFrame(start.frame);
    void setWidth(latest.current, true);
  }
  function keys(event: KeyboardEvent<HTMLDivElement>) {
    const next =
      event.key === "ArrowLeft"
        ? width + STEP
        : event.key === "ArrowRight"
          ? width - STEP
          : event.key === "Home"
            ? ASSISTANT_WIDTH.min
            : event.key === "End"
              ? ASSISTANT_WIDTH.max
              : null;
    if (next === null) return;
    event.preventDefault();
    void setWidth(next, true);
  }
  return (
    <aside
      className="assistant"
      aria-label={t.assistant.title}
      data-closing={closing || undefined}
      inert={closing}
      onAnimationEnd={(event) => {
        // Messages inside animate too; only the panel's own exit counts.
        if (closing && event.target === event.currentTarget) onClosed?.();
      }}
    >
      <div
        className="assistant-resizer"
        role="separator"
        aria-orientation="vertical"
        aria-label={t.assistant.resize}
        aria-valuemin={ASSISTANT_WIDTH.min}
        aria-valuemax={ASSISTANT_WIDTH.max}
        aria-valuenow={Math.round(width)}
        tabIndex={0}
        onKeyDown={keys}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.currentTarget.setPointerCapture(event.pointerId);
          drag.current = { x: event.clientX, width, frame: 0 };
          latest.current = width;
        }}
        onPointerMove={move}
        onPointerUp={release}
        onPointerCancel={release}
      />
      <Conversation
        key={site ? `${site.id}:${site.userId}` : "general"}
        site={site}
        focus={focus}
        timetable={timetable}
        onOpenPage={onOpenPage}
        onTimetable={onTimetable}
        onSettings={onSettings}
        onClose={onClose}
      />
    </aside>
  );
}
