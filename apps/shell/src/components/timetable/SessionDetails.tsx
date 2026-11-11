import { ExternalLink, X } from "lucide-react";
import { useI18n } from "../../i18n/context";
import type {
  QuestionSession,
  Questions,
  Session,
  TimetableChoices,
} from "../../lib/timetable";
import { PopupButton, Segmented } from "../ui";

/** Which review questions concern a session. */
export function questionsFor(questions: Questions | null, uid: string) {
  if (!questions) return {};
  const find = (list: QuestionSession[]) => list.find((q) => q.uid === uid);
  return {
    group: find(questions.groups),
    participation: find(questions.participation),
    time: find(questions.times),
    lv1: find(questions.lv1),
    sport: questions.sport
      .flatMap((w) => w.sessions)
      .find((q) => q.uid === uid),
  };
}

/** UIDs of sessions with an unanswered choice. */
export function pendingUids(
  questions: Questions | null,
  choices: TimetableChoices,
): Set<string> {
  const pending = new Set<string>();
  if (!questions) return pending;
  for (const q of [...questions.groups, ...questions.participation])
    if (!(q.uid in choices.groups)) pending.add(q.uid);
  for (const q of questions.times)
    if (!(q.uid in choices.times)) pending.add(q.uid);
  for (const q of questions.lv1)
    if (!(q.uid in choices.rooms)) pending.add(q.uid);
  for (const week of questions.sport)
    for (const q of week.sessions)
      if (!(q.uid in choices.sport)) pending.add(q.uid);
  return pending;
}

export function SessionDetails({
  session,
  question,
  questions,
  choices,
  editable,
  busy,
  onChoices,
  onOpenLink,
  onClose,
}: {
  /** The session as it appears in the week, or null when it is excluded. */
  session: Session | null;
  /** Shown for an excluded session, from the review questions. */
  question: QuestionSession | null;
  questions: Questions | null;
  choices: TimetableChoices;
  editable: boolean;
  busy: boolean;
  onChoices: (next: TimetableChoices) => void;
  onOpenLink?: (index: number) => void;
  onClose: () => void;
}) {
  const { t, locale } = useI18n();
  const s = t.timetable.session;
  const uid = session?.uid ?? question?.uid ?? "";
  const title = session?.title ?? question?.title ?? "";
  const date = session?.date ?? question?.date;
  const start = session?.start ?? question?.start;
  const end = session?.end ?? question?.end;
  const asked = questionsFor(questions, uid);
  const day = date
    ? new Intl.DateTimeFormat(locale, {
        weekday: "long",
        day: "numeric",
        month: "long",
        timeZone: "UTC",
      }).format(new Date(`${date}T00:00:00Z`))
    : "";
  const when = [day, start ? `${start}–${end ?? ""}` : t.timetable.allDay]
    .filter(Boolean)
    .join(", ");
  const set = (change: Partial<TimetableChoices>) =>
    onChoices({ ...choices, ...change });
  const without = <T,>(map: Record<string, T>) => {
    const next = { ...map };
    delete next[uid];
    return next;
  };
  const row = (label: string, value: React.ReactNode) =>
    value ? (
      <div className="session-field">
        <dt>{label}</dt>
        <dd>{value}</dd>
      </div>
    ) : null;
  return (
    <aside className="session-details" aria-labelledby="session-title">
      <header className="session-details-header">
        <h3 id="session-title">{title}</h3>
        <button
          className="button icon small"
          title={s.close}
          aria-label={s.close}
          onClick={onClose}
        >
          <X size={13} strokeWidth={2} aria-hidden="true" />
        </button>
      </header>
      {!session && <p className="session-excluded">{s.excluded}</p>}
      {session?.tentative && <p className="tag">{t.timetable.tentative}</p>}
      <dl className="session-fields">
        {row(s.when, when)}
        {row(
          s.room,
          session?.location ?? session?.locationNote ?? question?.location,
        )}
        {row(s.speakers, session?.speakers.join(", "))}
        {row(s.module, session?.module ?? question?.module)}
        {row(s.cohort, session?.cohortLabel ?? question?.cohortLabel)}
        {/* Preparation that is only a link's label is shown as the link. */}
        {!session?.links.includes(session?.preparation ?? "") &&
          row(s.preparation, session?.preparation)}
        {session?.remote && row(s.remote, "✓")}
        {session &&
          session.notes.length > 0 &&
          row(
            s.notes,
            <ul className="session-notes">
              {session.notes.map((note, i) => (
                <li key={i}>{note}</li>
              ))}
            </ul>,
          )}
      </dl>
      {session && session.links.length > 0 && onOpenLink && (
        <div className="session-links">
          {session.links.map((label, index) => (
            <button
              key={index}
              className="button small session-link"
              title={label}
              aria-label={s.openLink(label)}
              onClick={() => onOpenLink(index)}
            >
              <ExternalLink size={12} strokeWidth={2} aria-hidden="true" />
              <span className="session-link-label">{label}</span>
            </button>
          ))}
        </div>
      )}
      {editable && asked.group && (
        <div className="session-choice">
          <span className="field-label" aria-hidden="true">
            {s.group}
          </span>
          <PopupButton
            label={s.group}
            value={choices.groups[uid] ?? ""}
            disabled={busy}
            onChange={(value) =>
              set({
                groups: value
                  ? { ...choices.groups, [uid]: value }
                  : without(choices.groups),
              })
            }
          >
            <option value="">{s.groupNone}</option>
            {asked.group.options?.map((option) => (
              <option key={option.label ?? ""} value={option.label ?? ""}>
                {[
                  option.label,
                  option.location,
                  option.start && `${option.start}–${option.end ?? ""}`,
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </option>
            ))}
          </PopupButton>
        </div>
      )}
      {editable && asked.participation && (
        <Segmented
          name={`participation-${uid}`}
          legend={s.participation}
          disabled={busy}
          value={
            !(uid in choices.groups)
              ? "unanswered"
              : choices.groups[uid] === "__exclude__"
                ? "decline"
                : "attend"
          }
          options={[
            { value: "attend", label: s.attend },
            { value: "decline", label: s.decline },
            { value: "unanswered", label: s.unanswered },
          ]}
          onChange={(value) =>
            set({
              groups:
                value === "unanswered"
                  ? without(choices.groups)
                  : {
                      ...choices.groups,
                      [uid]: value === "decline" ? "__exclude__" : "",
                    },
            })
          }
        />
      )}
      {editable && asked.lv1 && questions && (
        <div className="session-choice">
          <span className="field-label" aria-hidden="true">
            {s.room}
          </span>
          <PopupButton
            label={s.room}
            value={choices.rooms[uid] ?? ""}
            disabled={busy}
            onChange={(value) =>
              set({
                rooms: value
                  ? { ...choices.rooms, [uid]: value }
                  : without(choices.rooms),
              })
            }
          >
            <option value="">{s.groupNone}</option>
            {questions.lv1Rooms.map((room) => (
              <option key={room} value={room}>
                {room}
              </option>
            ))}
          </PopupButton>
        </div>
      )}
      {editable && asked.sport && (
        <Segmented
          name={`sport-${uid}`}
          legend={s.participation}
          disabled={busy}
          value={choices.sport[uid] ?? "unanswered"}
          options={[
            { value: "keep", label: s.attend },
            { value: "exclude", label: s.decline },
            { value: "unanswered", label: s.unanswered },
          ]}
          onChange={(value) =>
            set({
              sport:
                value === "unanswered"
                  ? without(choices.sport)
                  : { ...choices.sport, [uid]: value as "keep" | "exclude" },
            })
          }
        />
      )}
      {editable && asked.time && (
        <fieldset className="session-time" disabled={busy}>
          <legend>{s.when}</legend>
          {(["start", "end"] as const).map((edge) => (
            <span key={edge} className="text-field compact">
              <input
                type="time"
                aria-label={`${s.when} ${edge}`}
                value={
                  choices.times[uid]?.[edge] ??
                  (edge === "start" ? start : end) ??
                  ""
                }
                onChange={(e) => {
                  const current = choices.times[uid] ?? {
                    start: start ?? "",
                    end: end ?? "",
                  };
                  const next = { ...current, [edge]: e.target.value };
                  if (next.start && next.end && next.start < next.end)
                    set({ times: { ...choices.times, [uid]: next } });
                }}
              />
            </span>
          ))}
        </fieldset>
      )}
    </aside>
  );
}
