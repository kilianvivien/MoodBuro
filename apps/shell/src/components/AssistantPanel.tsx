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
  ChevronsUpDown,
  CircleAlert,
  LoaderCircle,
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
  noteText,
  providerNames,
  withoutMarkers,
  type ContextChoice,
} from "../lib/assistant";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";

const icon = { size: 14, strokeWidth: 2, "aria-hidden": true } as const;
/** Arrow keys move the separator by this many points. */
const STEP = 16;

function Message({
  entry,
  onSource,
}: {
  entry: AssistantEntry;
  onSource: (source: AssistantSource) => void;
}) {
  const { t, locale } = useI18n();
  const user = entry.role === "user";
  return (
    <li className="assistant-message" data-role={entry.role}>
      <span className="visually-hidden">
        {user ? t.assistant.you : t.assistant.answer}
      </span>
      {entry.text && <p className="assistant-text">{entry.text}</p>}
      {entry.context && (
        <p className="assistant-meta">{t.assistant.attached(entry.context)}</p>
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
      {entry.sources.length > 0 && (
        <div className="assistant-sources">
          <span className="assistant-meta">{t.assistant.sources}</span>
          <ul role="list">
            {entry.sources.map((source) => (
              <li key={`${source.kind}:${source.id}`}>
                <button
                  className="button small"
                  title={t.assistant.openSource(source.label)}
                  aria-label={t.assistant.openSource(source.label)}
                  onClick={() => onSource(source)}
                >
                  <span>{source.label}</span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </li>
  );
}

/** A write the model asked for: the exact target, and nothing happens without Yes. */
function WriteConfirmation({
  confirmation,
  onAnswer,
}: {
  confirmation: AssistantConfirmation;
  onAnswer: (approve: boolean) => void;
}) {
  const { t } = useI18n();
  const decline = useRef<HTMLButtonElement>(null);
  useEffect(() => decline.current?.focus(), [confirmation.id]);
  return (
    <div
      className="confirm standalone"
      role="alertdialog"
      aria-labelledby="assistant-confirm-title"
      aria-describedby="assistant-confirm-body"
    >
      <p id="assistant-confirm-title" className="confirm-title">
        {t.assistant.confirmTitle}
      </p>
      <div id="assistant-confirm-body">
        <p className="assistant-target">{confirmation.subject}</p>
        <p className="confirm-body">
          {t.assistant.confirmBody(confirmation.site)}
        </p>
      </div>
      <div className="confirm-actions">
        <button
          ref={decline}
          className="button"
          onClick={() => onAnswer(false)}
        >
          {t.assistant.confirmDecline}
        </button>
        <button className="button" onClick={() => onAnswer(true)}>
          {t.assistant.confirmApprove}
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
          className="button icon small"
          title={t.assistant.clear}
          aria-label={t.assistant.clear}
          disabled={entries.length === 0 || props.clearing}
          onClick={() => props.onAskClear(true)}
        >
          <Trash2 {...icon} />
        </button>
        <button
          className="button icon small"
          title={t.assistant.close}
          aria-label={t.assistant.close}
          onClick={props.onClose}
        >
          <X {...icon} />
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
                {draft && <p className="assistant-text">{draft}</p>}
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
            <label className="popup-button">
              <span className="visually-hidden">
                {t.assistant.contextLabel}
              </span>
              <select
                value={props.context}
                disabled={busy || !site || !props.consent}
                aria-describedby="assistant-context-hint"
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
                          <option key={course.id} value={`course:${course.id}`}>
                            {course.name}
                          </option>
                        ))}
                      </optgroup>
                    )}
                  </>
                )}
              </select>
              <ChevronsUpDown size={12} strokeWidth={2} aria-hidden="true" />
            </label>
            <p className="assistant-meta" id="assistant-context-hint">
              {!site
                ? t.assistant.contextNeedsSite
                : !props.consent
                  ? t.assistant.contextNeedsConsent
                  : props.context === "none"
                    ? t.assistant.inputHint
                    : t.assistant.contextHint}
            </p>
            <div className="assistant-field">
              <textarea
                ref={props.input}
                rows={2}
                maxLength={8000}
                value={props.text}
                placeholder={t.assistant.placeholder}
                aria-label={t.assistant.inputLabel}
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
  onOpenPage,
  onSettings,
  onClose,
}: {
  site: Site | undefined;
  focus: number;
  onOpenPage: (source: AssistantSource, siteId: string) => void;
  onSettings: () => void;
  onClose: () => void;
}) {
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
  useEffect(() => {
    const list = log.current;
    if (list) list.scrollTop = list.scrollHeight;
  }, [entries, draft, step, confirmation]);
  // Consent withdrawn in Settings: nothing stays attached.
  const attached = consent && site ? context : "none";

  async function run(retry: boolean) {
    const prompt = text.trim();
    if (current.current || (!retry && !prompt)) return;
    const id = crypto.randomUUID();
    current.current = id;
    setRequest(id);
    setError(null);
    setDraft("");
    setStep(null);
    if (!retry) {
      setText("");
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
        context: contextFrom(attached),
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
      input={input}
      log={log}
      onText={setText}
      onContext={setContext}
      onSend={() => void run(false)}
      onStop={() => void ipc.assistantCancel().catch(() => {})}
      onRetry={() => void run(true)}
      onAnswer={(approve) => void answer(approve)}
      onAskClear={setClearing}
      onClear={() => void clear()}
      onSource={(source) => {
        if (siteId) onOpenPage(source, siteId);
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
  onOpenPage,
  onSettings,
  onClose,
}: {
  site: Site | undefined;
  /** Bumped each time the panel is asked to take keyboard focus. */
  focus: number;
  onOpenPage: (source: AssistantSource, siteId: string) => void;
  onSettings: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const width = useAssistant((state) => state.panel.width);
  const setWidth = useAssistant((state) => state.setWidth);
  const drag = useRef<{ x: number; width: number; frame: number } | null>(null);
  const latest = useRef(width);
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
    <aside className="assistant" aria-label={t.assistant.title}>
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
        onOpenPage={onOpenPage}
        onSettings={onSettings}
        onClose={onClose}
      />
    </aside>
  );
}
