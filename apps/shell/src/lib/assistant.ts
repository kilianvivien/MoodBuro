import type { AssistantNote, AssistantProvider } from "../ipc";
import type { Locale } from "../i18n/locale";
import type { Messages } from "../i18n/messages";
import { ago } from "./time";

/** Product names: the same in every language, so not in the catalogues. */
export const providerNames: Record<AssistantProvider, string> = {
  albert: "Albert API",
  mistral: "Mistral AI",
};
export const providers: readonly AssistantProvider[] = ["albert", "mistral"];

/**
 * Text as it streams still carries the model's source markers ("[course:12]").
 * Rust replaces them with verified source links once the answer is complete;
 * until then they are simply hidden, including one cut off mid-stream.
 */
export function withoutMarkers(text: string) {
  return text
    .replace(
      / ?\[(?:course|activity|deadline|notification|file|session):\s*\d+\]/gu,
      "",
    )
    .replace(/ ?\[(?:c|a|d|n|f|s)[a-z]*(?::\s*\d*)?$/u, "");
}

/** The context pop-up's value: nothing, the organisation, or one of its courses. */
export type ContextChoice = "none" | "site" | `course:${number}`;
export function contextFrom(choice: ContextChoice) {
  if (choice === "none") return null;
  if (choice === "site") return { kind: "site" as const, courseId: null };
  return { kind: "course" as const, courseId: Number(choice.slice(7)) };
}

/** "1.4 MB" / "1,4 Mo": a size for a confirmation, in the reader's language. */
export function fileSize(locale: Locale, bytes: number) {
  const [unit, value] =
    bytes >= 1_000_000
      ? (["megabyte", bytes / 1_000_000] as const)
      : (["kilobyte", Math.max(1, bytes / 1000)] as const);
  return new Intl.NumberFormat(locale, {
    style: "unit",
    unit,
    maximumFractionDigits: value < 10 ? 1 : 0,
  }).format(value);
}

/** One line saying what the assistant looked up or did, and how fresh it was. */
export function noteText(
  t: Messages["assistant"]["notes"],
  locale: Locale,
  note: AssistantNote,
  now = Date.now(),
) {
  const label = note.label ?? "";
  switch (note.code) {
    case "courses":
    case "activities":
    case "agenda":
    case "files":
    case "notifications": {
      const detail = note.stale
        ? t.stale
        : note.age === null
          ? null
          : (() => {
              const when = ago(locale, Math.floor(now / 1000) - note.age, now);
              return when ? t.updated(when) : t.updatedNow;
            })();
      return detail ? `${t[note.code]} (${detail})` : t[note.code];
    }
    case "opened":
      return t.opened(label);
    case "opened-notifications":
      return t.openedNotifications;
    case "opened-calendar":
      return t.openedCalendar;
    case "saved":
      return t.saved(label);
    case "save-failed":
      return t.saveFailed(label);
    case "save-declined":
      return t.saveDeclined(label);
    case "reminder-day":
      return t.reminderDay(label);
    case "reminder-hour":
      return t.reminderHour(label);
    case "reminder-failed":
      return t.reminderFailed(label);
    case "reminder-declined":
      return t.reminderDeclined(label);
    case "marked":
      return t.marked(label);
    case "declined":
      return t.declined(label);
    case "write-failed":
      return t.writeFailed(label);
    case "timetable":
      return t.timetable;
    case "refused":
      return t.refused;
    case "stopped":
      return t.stopped;
  }
}
