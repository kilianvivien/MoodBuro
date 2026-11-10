import { invoke, isTauri } from "@tauri-apps/api/core";
import { previewFlags } from "./lib/window";

export interface Site {
  id: string;
  wwwroot: string;
  name: string;
  userId: number;
  fullName: string;
  shortLabel?: string | null;
}

export interface PopupContext extends Settings {
  siteName: string;
  canGoBack: boolean;
  canGoForward: boolean;
}

export interface Download {
  id: string;
  siteId: string;
  filename: string;
  state: "downloading" | "complete" | "failed";
  startedAt: number;
  needsConfirmation: boolean;
  canPreview?: boolean;
  courseId?: number | null;
  courseName?: string | null;
}

export type DownloadState = Download["state"];
export interface DownloadQuery {
  query: string;
  siteId: string | null;
  state: DownloadState | null;
  offset: number;
  courseId?: number | null;
}
export interface DownloadPage {
  items: Download[];
  total: number;
}
/** Rows per page of download history; matches `downloads::PAGE_SIZE` in Rust. */
export const DOWNLOAD_PAGE_SIZE = 25;

export interface InboxItem {
  siteId: string;
  id: number;
  subject: string;
  /** Unix seconds. */
  timeCreated: number;
  read: boolean;
}
export interface Course {
  siteId: string;
  id: number;
  name: string;
  shortName: string;
  pinned: boolean;
}
export interface Activity {
  siteId: string;
  courseId: number;
  id: number;
  name: string;
  module: string;
  refreshedAt: number;
}
export interface OfflineResource {
  key: string;
  moduleId: number;
  sectionId: number;
  sectionName: string;
  activityName: string;
  filename: string;
  bytes: number | null;
}
export interface OfflineCopy {
  id: string;
  siteId: string;
  courseId: number;
  resourceKey: string;
  moduleId: number;
  filename: string;
  sectionName: string;
  bytes: number;
  savedAt: number;
  available: boolean;
  needsConfirmation: boolean;
}
export interface OfflineOutcome {
  key: string;
  reused: boolean;
  error: string | null;
}
export interface OfflineProgress {
  siteId: string;
  courseId: number;
  key: string;
  received: number;
  total: number | null;
}
export interface OfflineCourse {
  siteId: string;
  courseId: number;
  userId: number;
}

export type EventKind = "due" | "open" | "user" | "site" | "course" | "other";
export interface ReminderSettings {
  siteId: string;
  userId: number;
  hour: boolean;
  day: boolean;
  enabledAt: number;
  refreshedAt: number | null;
  refreshError: string | null;
  deliveryFailed: boolean;
}
export interface AgendaEvent {
  siteId: string;
  id: number;
  name: string;
  /** Unix seconds. */
  timeStart: number;
  /** Seconds; 0 for a deadline. */
  duration: number;
  kind: EventKind;
  courseId: number | null;
  courseName: string | null;
}
/** Cached metadata plus when each organisation last refreshed it (Unix seconds). */
export interface Listing<T> {
  items: T[];
  refreshed: { siteId: string; refreshedAt: number }[];
}
/** Fixed Moodle destinations; Rust builds the URL from the saved site. */
export type MoodlePage =
  | "notifications"
  | "messages"
  | "calendar"
  | "calendar-day"
  | "course"
  | "activity";

export type DeepLinkOutcome =
  { kind: "site"; siteId: string } | { kind: "onboarding"; url: string };

export interface UnreadStatus {
  siteId: string;
  unread: number;
  /** Unread conversations included in `unread`; the inbox lists notifications only. */
  messages?: number;
  connection: "checking" | "online" | "offline" | "reauth" | "error";
}

export type Appearance = "system" | "light" | "dark";
/** UI language; "system" follows the Mac's preferred languages. */
export type Language = "system" | "fr" | "en";
/** A PDF or image a Moodle page opens directly: shown there, saved, or saved then opened. */
export type FileHandling = "inline" | "save" | "open";
export interface Settings {
  appearance: Appearance;
  language: Language;
  pdf: FileHandling;
  images: FileHandling;
  /** Opt-in system-wide ⌘⇧M for Quick Open. */
  globalShortcut: boolean;
}
/** What Rust returns: the saved values and whether the saved shortcut is registered. */
export interface SavedSettings extends Settings {
  shortcutUnavailable: boolean;
}
export type ShellView =
  | "onboarding"
  | "settings"
  | "downloads"
  | "inbox"
  | "courses"
  | "agenda"
  | "quick-open";
export type NavAction =
  "back" | "forward" | "reload" | "home" | "open_in_browser";

/** Live transfer feedback contains only display metadata, never a path or URL. */
export interface DownloadFeedback {
  siteId: string;
  userId: number;
  filename: string;
  state: DownloadState;
}

export type AssistantProvider = "albert" | "mistral";
/** Panel width in points; Rust clamps it and positions the Moodle view beside it. */
export interface AssistantPanelState {
  open: boolean;
  width: number;
}
export interface AssistantStatus {
  provider: AssistantProvider;
  model: string;
  /** The key itself never leaves Rust and the OS credential store. */
  hasKey: boolean;
  /** Moodle context may be sent to the current provider. */
  consent: boolean;
  panel: AssistantPanelState;
}
/** A Moodle item an answer relied on; opened through `openPage`. */
export interface AssistantSource {
  kind: "course" | "activity" | "deadline" | "notification" | "file";
  id: number;
  page: MoodlePage;
  value: number | null;
  label: string;
  /** A course file: its course and resource key, to find the copy saved on this Mac. */
  course?: number;
  key?: string;
}
export type AssistantNoteCode =
  | "courses"
  | "activities"
  | "agenda"
  | "notifications"
  | "files"
  | "saved"
  | "save-failed"
  | "save-declined"
  | "reminder-day"
  | "reminder-hour"
  | "reminder-failed"
  | "reminder-declined"
  | "opened"
  | "opened-notifications"
  | "opened-calendar"
  | "marked"
  | "declined"
  | "write-failed"
  | "refused"
  | "stopped";
/** What the assistant looked up or did; worded by the shell. */
export interface AssistantNote {
  code: AssistantNoteCode;
  label: string | null;
  /** Seconds since the data was refreshed. */
  age: number | null;
  stale: boolean;
}
export interface AssistantEntry {
  role: "user" | "assistant";
  text: string;
  /** Name of the Moodle context attached to a user message. */
  context: string | null;
  sources: AssistantSource[];
  notes: AssistantNote[];
}
export interface AssistantContext {
  kind: "site" | "course";
  courseId: number | null;
}
export interface AssistantPrompt {
  /** Tags this turn's `assistant://` events. */
  request: string;
  siteId: string | null;
  userId: number | null;
  text: string | null;
  context: AssistantContext | null;
  /** Minutes east of UTC. */
  utcOffset: number;
  retry: boolean;
}
export type AssistantStep =
  | "courses"
  | "activities"
  | "agenda"
  | "notifications"
  | "files"
  | "open"
  | "save"
  | "confirm";
/** An action the model asked for, described by Rust from this account's saved data. */
export interface AssistantConfirmation {
  request: string;
  id: string;
  kind: "mark-read" | "save-file" | "reminder";
  site: string;
  subject: string;
  /** `day` or `hour` for a reminder; a size in bytes for a file. */
  detail: string | null;
}
/** A one-off reminder on a single deadline. */
export interface ReminderPick {
  siteId: string;
  eventId: number;
  /** Seconds before the deadline. */
  offset: 3600 | 86400;
  due: number;
  name: string;
}
export const ASSISTANT_WIDTH = { min: 280, max: 520, initial: 340 } as const;

export const desktop = isTauri();

// Dev-only `?demo`: neutral sample organisations so the browser preview can
// show the sidebar, toolbar and Settings rows. Never shipped in release builds.
const demo = !desktop && import.meta.env.DEV && previewFlags().demo;
const demoSites: Site[] = [
  {
    id: "demo-1",
    wwwroot: "https://moodle.example.edu/",
    name: "Example University",
    userId: 1,
    fullName: "Alex Martin",
  },
  {
    id: "demo-2",
    wwwroot: "https://moodle.exemple.fr/",
    name: "Campus Exemple",
    userId: 2,
    fullName: "Alex Martin",
    shortLabel: "CX",
  },
];

// Dev-only sample data for the browser preview. Neutral names, in memory, and
// mutable so Mark Read, pins and Clear History can be tried without a Mac app.
function buildDemoData() {
  const demoNow = Math.floor(Date.now() / 1000);
  const hour = 3600;
  const day = 24 * hour;
  return {
    unread: [
      { siteId: "demo-1", unread: 4, messages: 1, connection: "online" },
      { siteId: "demo-2", unread: 1, messages: 0, connection: "online" },
    ] as UnreadStatus[],
    inbox: [
      ["demo-1", 41, "Assignment graded: Policy brief", 25 * 60, false],
      ["demo-1", 40, "New forum post in Public Economics", 3 * hour, false],
      ["demo-2", 12, "Reminder: seminar room changed", 5 * hour, false],
      ["demo-1", 39, "Quiz opens tomorrow: Statistics 2", day + hour, false],
      ["demo-1", 38, "You were added to Group B", day + 6 * hour, true],
      ["demo-2", 11, "Course announcement: reading list", 3 * day, true],
      ["demo-1", 37, "Feedback available for your essay", 6 * day, true],
    ].map(([siteId, id, subject, ago, read]) => ({
      siteId,
      id,
      subject,
      timeCreated: demoNow - (ago as number),
      read,
    })) as InboxItem[],
    courses: [
      ["demo-1", 101, "Public Economics", "ECO-201", true],
      ["demo-1", 102, "Statistics 2", "STA-202", false],
      ["demo-1", 103, "Comparative Public Law", "LAW-110", true],
      ["demo-1", 104, "Negotiation Workshop", "NEG-050", false],
      ["demo-1", 105, "European Institutions", "EUR-120", false],
      ["demo-2", 201, "Gestion de projet", "GDP-1", false],
      ["demo-2", 202, "Finances publiques", "FIP-2", true],
      ["demo-2", 203, "Anglais professionnel", "ANG-3", false],
    ].map(([siteId, id, name, shortName, pinned]) => ({
      siteId,
      id,
      name,
      shortName,
      pinned,
    })) as Course[],
    events: [
      [
        "demo-1",
        1,
        "Policy brief due",
        4 * hour,
        0,
        "due",
        101,
        "Public Economics",
      ],
      [
        "demo-2",
        2,
        "Séminaire : budget local",
        6 * hour,
        2 * hour,
        "course",
        202,
        "Finances publiques",
      ],
      [
        "demo-1",
        3,
        "Quiz opens: Statistics 2",
        day + 2 * hour,
        0,
        "open",
        102,
        "Statistics 2",
      ],
      [
        "demo-1",
        4,
        "Moot court rehearsal",
        day + 5 * hour,
        90 * 60,
        "course",
        103,
        "Comparative Public Law",
      ],
      ["demo-1", 5, "Library tour", 3 * day, hour, "site", null, null],
      [
        "demo-2",
        6,
        "Rendu du dossier de groupe",
        5 * day,
        0,
        "due",
        201,
        "Gestion de projet",
      ],
      ["demo-1", 7, "Revise chapter 4", 8 * day, 0, "user", null, null],
    ].map(
      ([
        siteId,
        id,
        name,
        inSeconds,
        duration,
        kind,
        courseId,
        courseName,
      ]) => ({
        siteId,
        id,
        name,
        timeStart: demoNow + (inSeconds as number),
        duration,
        kind,
        courseId,
        courseName,
      }),
    ) as AgendaEvent[],
    refreshed: { courses: demoNow - 4 * 60, agenda: demoNow - 9 * 60 },
    downloads: [
      ["d1", "demo-1", "Policy brief — feedback.pdf", "complete", 20 * 60],
      ["d2", "demo-1", "Lecture 7 slides.key", "downloading", 2 * 60],
      ["d3", "demo-2", "Budget-local-2026.xlsx", "complete", 5 * hour],
      ["d4", "demo-1", "statistics-dataset.zip", "complete", day],
      ["d5", "demo-2", "Guide du stagiaire.pdf", "failed", 2 * day],
      ["d6", "demo-1", "Reading list week 3.docx", "complete", 4 * day],
    ].map(([id, siteId, filename, state, ago]) => ({
      id,
      siteId,
      filename,
      state,
      startedAt: demoNow - (ago as number),
      canPreview: /\.(pdf|docx|xlsx|pptx)$/iu.test(filename as string),
      courseId: null,
      courseName: null,
      needsConfirmation: /\.(zip|key)$/u.test(filename as string),
    })) as Download[],
  };
}
// Built only in development: release bundles carry no sample data.
const demoData = import.meta.env.DEV ? buildDemoData() : (null as never);

const pause = (ms: number) =>
  new Promise<void>((resolve) => setTimeout(resolve, ms));
const demoListing = <T>(
  items: T[],
  kind: "courses" | "agenda",
): Listing<T> => ({
  items: items.map((item) => ({ ...item })),
  refreshed: demoSites.map((site) => ({
    siteId: site.id,
    refreshedAt: demoData.refreshed[kind],
  })),
});
const empty = <T>(): Listing<T> => ({ items: [], refreshed: [] });

const desktopOnly = () =>
  Promise.reject(
    new Error("Open the desktop app with pnpm tauri dev to manage sign-in."),
  );

const desktopFeature = () =>
  Promise.reject(
    new Error("Open the desktop app with pnpm tauri dev to use this feature."),
  );

const appearances: readonly Appearance[] = ["system", "light", "dark"];
const languages: readonly Language[] = ["system", "fr", "en"];

const handlings: readonly FileHandling[] = ["inline", "save", "open"];
const handling = (value: unknown): FileHandling =>
  handlings.includes(value as FileHandling)
    ? (value as FileHandling)
    : "inline";

/** Fills values an older settings store does not return yet. */
export function settingsFrom(raw: Partial<SavedSettings> | null | undefined) {
  return {
    appearance: appearances.includes(raw?.appearance as Appearance)
      ? (raw!.appearance as Appearance)
      : "system",
    language: languages.includes(raw?.language as Language)
      ? (raw!.language as Language)
      : "system",
    pdf: handling(raw?.pdf),
    images: handling(raw?.images),
    globalShortcut: raw?.globalShortcut === true,
    shortcutUnavailable: raw?.shortcutUnavailable === true,
  } satisfies SavedSettings;
}

// The browser preview has no Rust side; settings live in memory only and the
// UI says so instead of pretending they were saved.
let previewSettings: SavedSettings = settingsFrom(null);

// Dev-only assistant: canned answers so the panel can be tried in the browser preview.
let demoAssistant: AssistantStatus = {
  provider: "albert",
  model: "DeepSeek-V4-Flash-0731",
  hasKey: demo,
  consent: false,
  panel: { open: false, width: ASSISTANT_WIDTH.initial },
};
const demoModels: Record<AssistantProvider, string[]> = {
  albert: ["openweight-large", "DeepSeek-V4-Flash-0731", "openweight-small"],
  mistral: ["mistral-small-latest", "mistral-large-4"],
};
const demoChats = new Map<string, AssistantEntry[]>();
const demoChat = (siteId: string | null) => {
  const key = siteId ?? "general";
  if (!demoChats.has(key)) demoChats.set(key, []);
  return demoChats.get(key)!;
};
const assistantWidth = (width: number) =>
  Number.isFinite(width)
    ? Math.min(ASSISTANT_WIDTH.max, Math.max(ASSISTANT_WIDTH.min, width))
    : ASSISTANT_WIDTH.initial;

const demoReminders = new Map<string, ReminderSettings>();
let demoPicks: ReminderPick[] = [];
const demoOfflineCopies: OfflineCopy[] = [];
const demoResources: OfflineResource[] = [
  {
    key: "a".repeat(64),
    moduleId: 31,
    sectionId: 1,
    sectionName: "Week 1",
    activityName: "Reading",
    filename: "reading.pdf",
    bytes: 524288,
  },
  {
    key: "b".repeat(64),
    moduleId: 32,
    sectionId: 2,
    sectionName: "Week 2",
    activityName: "Notes",
    filename: "notes.txt",
    bytes: null,
  },
];
export const ipc = {
  assistant: (): Promise<AssistantStatus> =>
    desktop
      ? invoke<AssistantStatus>("assistant_status")
      : Promise.resolve({ ...demoAssistant }),
  configureAssistant: (change: {
    provider?: AssistantProvider;
    model?: string;
    consent?: boolean;
  }): Promise<AssistantStatus> => {
    if (desktop)
      return invoke<AssistantStatus>("assistant_configure", { ...change });
    if (!demo) return desktopFeature();
    if (
      change.consent === false ||
      (change.provider && change.provider !== demoAssistant.provider)
    )
      demoChats.clear();
    if (change.provider && change.provider !== demoAssistant.provider)
      demoAssistant = {
        ...demoAssistant,
        provider: change.provider,
        model: demoModels[change.provider][1],
        consent: false,
      };
    demoAssistant = {
      ...demoAssistant,
      ...(change.model ? { model: change.model } : {}),
      ...(change.consent === undefined ? {} : { consent: change.consent }),
    };
    return Promise.resolve({ ...demoAssistant });
  },
  setAssistantKey: (provider: AssistantProvider, key: string) => {
    if (desktop)
      return invoke<AssistantStatus>("assistant_key_set", { provider, key });
    if (!demo) return desktopFeature();
    demoAssistant = { ...demoAssistant, hasKey: true };
    return Promise.resolve({ ...demoAssistant });
  },
  removeAssistantKey: (provider: AssistantProvider) => {
    if (desktop)
      return invoke<AssistantStatus>("assistant_key_remove", { provider });
    if (!demo) return desktopFeature();
    demoAssistant = { ...demoAssistant, hasKey: false };
    return Promise.resolve({ ...demoAssistant });
  },
  /** Checks the saved key with the provider and lists its chat models. */
  assistantModels: async (): Promise<string[]> => {
    if (desktop) return invoke<string[]>("assistant_models");
    if (!demo) return desktopFeature();
    await pause(500);
    return demoModels[demoAssistant.provider];
  },
  /** `commit` false while dragging: the Moodle view follows, nothing is saved. */
  assistantPanel: (
    open: boolean,
    width: number,
    commit = true,
  ): Promise<AssistantPanelState> => {
    if (desktop)
      return invoke<AssistantPanelState>("assistant_panel", {
        open,
        width,
        commit,
      });
    demoAssistant = {
      ...demoAssistant,
      panel: { open, width: assistantWidth(width) },
    };
    return Promise.resolve({ ...demoAssistant.panel });
  },
  assistantHistory: (siteId: string | null, userId: number | null) =>
    desktop
      ? invoke<AssistantEntry[]>("assistant_history", { siteId, userId })
      : Promise.resolve(demo ? [...demoChat(siteId)] : []),
  /** Resolves with the whole conversation once the answer is complete. */
  assistantSend: async (prompt: AssistantPrompt): Promise<AssistantEntry[]> => {
    if (desktop) return invoke<AssistantEntry[]>("assistant_send", { prompt });
    if (!demo) return desktopFeature();
    const chat = demoChat(prompt.siteId);
    const course = demoData.courses.find(
      (c) =>
        c.siteId === prompt.siteId &&
        (prompt.context?.courseId == null || c.id === prompt.context.courseId),
    );
    if (!prompt.retry)
      chat.push({
        role: "user",
        text: prompt.text ?? "",
        context: prompt.context
          ? prompt.context.kind === "course"
            ? (course?.name ?? null)
            : (demoSites.find((s) => s.id === prompt.siteId)?.name ?? null)
          : null,
        sources: [],
        notes: [],
      });
    await pause(900);
    chat.push(
      prompt.context && course
        ? {
            role: "assistant",
            text: `Sample answer from the browser preview. In **${course.name}** I can list:\n\n- Two feedback forms: "Tell us what you thought of the course" and a self-assessment quiz\n- A course sheet and a summary to download\n- Four content parts:\n  - Part 1: Public service ethics as a shared professional framework\n  - Part 2: Values under tension\n\nI cannot read the *contents* of pages or files, only their names.`,
            context: null,
            sources: [
              {
                kind: "course",
                id: course.id,
                page: "course",
                value: course.id,
                label: course.name,
              },
              ...[
                "Tell us what you thought of the course – end-of-term survey",
                "Course sheet",
                "Part 1: Public service ethics as a shared professional framework",
                "Part 2: Values under tension",
              ].map((label, index) => ({
                kind: "activity" as const,
                id: 9000 + index,
                page: "course" as const,
                value: course.id,
                label,
              })),
              {
                kind: "file" as const,
                id: 1,
                page: "course" as const,
                value: course.id,
                label: "course-summary.pdf",
                course: course.id,
                key: "c".repeat(64),
              },
            ],
            notes: [
              { code: "courses", label: null, age: 240, stale: false },
              { code: "files", label: null, age: 0, stale: false },
            ],
          }
        : {
            role: "assistant",
            text: "Sample answer from the browser preview. Connect a provider in the desktop app for real answers.",
            context: null,
            sources: [],
            notes: [],
          },
    );
    return [...chat];
  },
  assistantCancel: () =>
    desktop ? invoke<void>("assistant_cancel") : Promise.resolve(),
  assistantConfirm: (id: string, approve: boolean) =>
    desktop
      ? invoke<void>("assistant_confirm", { id, approve })
      : Promise.resolve(),
  /** Forgets one account's conversation, or all of them. */
  clearAssistant: (
    siteId: string | null,
    userId: number | null,
    all = false,
  ) => {
    if (desktop)
      return invoke<void>("assistant_clear", { siteId, userId, all });
    if (all) demoChats.clear();
    else demoChats.delete(siteId ?? "general");
    return Promise.resolve();
  },
  reminders: (): Promise<ReminderSettings[]> =>
    desktop
      ? invoke<ReminderSettings[]>("reminders_list")
      : Promise.resolve(
          demo
            ? demoSites.map((site) => ({
                siteId: site.id,
                userId: site.userId,
                hour: false,
                day: false,
                enabledAt: 0,
                refreshedAt: demoData.refreshed.agenda,
                refreshError: null,
                deliveryFailed: false,
                ...demoReminders.get(site.id),
              }))
            : [],
        ),
  setReminders: async (
    siteId: string,
    userId: number,
    hour: boolean,
    day: boolean,
  ) => {
    if (desktop)
      return invoke<void>("reminders_set", { siteId, userId, hour, day });
    if (!demo) return desktopFeature();
    demoReminders.set(siteId, {
      siteId,
      userId,
      hour,
      day,
      enabledAt: Math.floor(Date.now() / 1000),
      refreshedAt: demoData.refreshed.agenda,
      refreshError: null,
      deliveryFailed: false,
    });
  },
  reminderPicks: (): Promise<ReminderPick[]> =>
    desktop
      ? invoke<ReminderPick[]>("reminders_picks")
      : Promise.resolve(demo ? demoPicks.map((pick) => ({ ...pick })) : []),
  /** Adds or removes a one-off reminder before one deadline. */
  setReminderPick: async (
    event: { siteId: string; id: number; timeStart: number; name: string },
    userId: number,
    offset: ReminderPick["offset"],
    on: boolean,
  ) => {
    if (desktop)
      return invoke<void>("reminders_pick", {
        siteId: event.siteId,
        userId,
        eventId: event.id,
        offset,
        on,
      });
    if (!demo) return desktopFeature();
    demoPicks = demoPicks.filter(
      (pick) =>
        pick.siteId !== event.siteId ||
        pick.eventId !== event.id ||
        pick.offset !== offset,
    );
    if (on)
      demoPicks.push({
        siteId: event.siteId,
        eventId: event.id,
        offset,
        due: event.timeStart,
        name: event.name,
      });
  },
  /**
   * A file source in an answer: Rust shows the copy saved on this Mac. Resolves
   * false when nothing is saved, so the caller opens the activity in Moodle.
   */
  assistantOpenFile: (target: OfflineCourse, key: string): Promise<boolean> =>
    desktop
      ? invoke<boolean>("assistant_open_file", { target, key })
      : Promise.resolve(false),
  /** The course on screen in the organisation's Moodle view, if any. */
  assistantCurrentCourse: (siteId: string): Promise<number | null> =>
    desktop
      ? invoke<number | null>("assistant_current_course", { siteId })
      : Promise.resolve(
          demo
            ? (demoData.courses.find((c) => c.siteId === siteId)?.id ?? null)
            : null,
        ),
  offlineResources: (course: OfflineCourse) =>
    desktop
      ? invoke<{ items: OfflineResource[]; skipped: number }>(
          "offline_resources",
          { ...course },
        )
      : demo
        ? Promise.resolve({
            items: demoResources.map((r) => ({ ...r })),
            skipped: 1,
          })
        : desktopFeature(),
  offlineSaved: (course: OfflineCourse) =>
    desktop
      ? invoke<OfflineCopy[]>("offline_saved", { ...course })
      : Promise.resolve(
          demo
            ? demoOfflineCopies
                .filter(
                  (c) =>
                    c.siteId === course.siteId &&
                    c.courseId === course.courseId,
                )
                .map((c) => ({ ...c }))
            : [],
        ),
  offlineSave: async (
    course: OfflineCourse,
    keys: string[],
  ): Promise<OfflineOutcome[]> => {
    if (desktop)
      return invoke<OfflineOutcome[]>("offline_save", { ...course, keys });
    if (!demo) return desktopFeature();
    await pause(600);
    return keys.map((key) => {
      const resource = demoResources.find((r) => r.key === key);
      if (!resource)
        return {
          key,
          reused: false,
          error:
            "This resource is no longer available. Reload the course file list.",
        };
      const reused = demoOfflineCopies.some(
        (c) =>
          c.resourceKey === key &&
          c.siteId === course.siteId &&
          c.courseId === course.courseId,
      );
      if (!reused)
        demoOfflineCopies.push({
          id: crypto.randomUUID(),
          siteId: course.siteId,
          courseId: course.courseId,
          resourceKey: key,
          moduleId: resource.moduleId,
          filename: resource.filename,
          sectionName: resource.sectionName,
          bytes: resource.bytes ?? 1024,
          savedAt: Math.floor(Date.now() / 1000),
          available: true,
          needsConfirmation: false,
        });
      return { key, reused, error: null };
    });
  },
  offlineOpen: (
    course: OfflineCourse,
    copyId: string,
    reveal = false,
    confirmed = false,
  ) =>
    desktop
      ? invoke<void>("offline_open", {
          target: course,
          copyId,
          reveal,
          confirmed,
        })
      : desktopFeature(),
  popupContext: () => invoke<PopupContext>("popup_context"),
  popupNav: (action: NavAction | "downloads") =>
    invoke<void>("popup_nav", { action }),
  downloads: (filter: DownloadQuery): Promise<DownloadPage> => {
    if (desktop) return invoke<DownloadPage>("downloads_page", { ...filter });
    const needle = filter.query.trim().toLowerCase();
    const items = (demo ? demoData.downloads : []).filter(
      (item) =>
        item.filename.toLowerCase().includes(needle) &&
        (!filter.siteId || item.siteId === filter.siteId) &&
        (!filter.state || item.state === filter.state) &&
        (filter.courseId == null ||
          (filter.courseId === 0
            ? item.courseId == null
            : item.courseId === filter.courseId)),
    );
    return Promise.resolve({
      items: items.slice(filter.offset, filter.offset + DOWNLOAD_PAGE_SIZE),
      total: items.length,
    });
  },
  clearDownloads: (siteId: string | null) => {
    if (desktop) return invoke<void>("downloads_clear", { siteId });
    if (!demo) return desktopFeature();
    demoData.downloads = demoData.downloads.filter(
      (item) =>
        item.state === "downloading" || (siteId && item.siteId !== siteId),
    );
    return Promise.resolve();
  },
  previewDownload: (id: string) =>
    desktop ? invoke<void>("download_preview", { id }) : desktopFeature(),
  refreshCourseFolders: (id: string) =>
    desktop ? invoke<void>("course_folders_refresh", { id }) : desktopFeature(),
  copyDownloadPath: (id: string) =>
    desktop ? invoke<void>("download_copy_path", { id }) : desktopFeature(),
  inbox: () =>
    desktop
      ? invoke<InboxItem[]>("inbox_list")
      : Promise.resolve(
          demo ? demoData.inbox.map((item) => ({ ...item })) : [],
        ),
  /** Marks one notification read, or every notification of the site when `id` is null. */
  markRead: async (siteId: string, id: number | null) => {
    if (desktop) return invoke<void>("inbox_mark_read", { siteId, id });
    if (!demo) return desktopFeature();
    await pause(250);
    for (const item of demoData.inbox)
      if (item.siteId === siteId && (id === null || item.id === id))
        item.read = true;
    for (const status of demoData.unread)
      if (status.siteId === siteId)
        status.unread =
          (status.messages ?? 0) +
          demoData.inbox.filter((n) => n.siteId === siteId && !n.read).length;
  },
  openPage: (id: string, page: MoodlePage, value: number | null = null) =>
    demo
      ? Promise.resolve()
      : desktop
        ? invoke<void>("site_open_page", { id, page, value })
        : desktopFeature(),
  activities: () =>
    desktop
      ? invoke<Activity[]>("activities_list")
      : Promise.resolve<Activity[]>(
          demo
            ? demoData.courses.map((c, index) => ({
                siteId: c.siteId,
                courseId: c.id,
                id: 1000 + index,
                name: `${c.name} · Reading`,
                module: "resource",
                refreshedAt: demoData.refreshed.courses,
              }))
            : [],
        ),
  courses: () =>
    desktop
      ? invoke<Listing<Course>>("courses_list")
      : Promise.resolve(
          demo ? demoListing(demoData.courses, "courses") : empty<Course>(),
        ),
  refreshCourses: async (id: string) => {
    if (desktop) return invoke<void>("courses_refresh", { id });
    if (!demo) return desktopFeature();
    await pause(700);
    demoData.refreshed.courses = Math.floor(Date.now() / 1000);
  },
  pinCourse: (siteId: string, courseId: number, pinned: boolean) => {
    if (desktop)
      return invoke<void>("course_pin", { siteId, courseId, pinned });
    if (!demo) return desktopFeature();
    for (const course of demoData.courses)
      if (course.siteId === siteId && course.id === courseId)
        course.pinned = pinned;
    return Promise.resolve();
  },
  exportAgenda: (
    siteId: string | null,
    start: number | null,
    end: number | null,
  ) =>
    desktop
      ? invoke<{ filename: string; count: number; revealed: boolean }>(
          "agenda_export",
          { siteId, start, end },
        )
      : desktopFeature(),
  moveSite: async (id: string, direction: -1 | 1) => {
    if (desktop) return invoke<void>("site_move", { id, direction });
    if (!demo) return desktopOnly();
    const index = demoSites.findIndex((site) => site.id === id);
    const next = index + direction;
    if (index >= 0 && next >= 0 && next < demoSites.length)
      [demoSites[index], demoSites[next]] = [demoSites[next], demoSites[index]];
  },
  agenda: () =>
    desktop
      ? invoke<Listing<AgendaEvent>>("agenda_list")
      : Promise.resolve(
          demo ? demoListing(demoData.events, "agenda") : empty<AgendaEvent>(),
        ),
  refreshAgenda: async (id: string) => {
    if (desktop) return invoke<void>("agenda_refresh", { id });
    if (!demo) return desktopFeature();
    await pause(700);
    demoData.refreshed.agenda = Math.floor(Date.now() / 1000);
  },
  openDownload: (id: string, confirmed = false) =>
    desktop
      ? invoke<void>("download_open", { id, confirmed })
      : desktopFeature(),
  revealDownload: (id: string) =>
    desktop ? invoke<void>("download_reveal", { id }) : desktopFeature(),
  deepLink: () =>
    desktop
      ? invoke<DeepLinkOutcome | null>("deep_link_open")
      : Promise.resolve(null),
  unread: () =>
    desktop
      ? invoke<UnreadStatus[]>("unread_list")
      : Promise.resolve(
          demo ? demoData.unread.map((status) => ({ ...status })) : [],
        ),
  setShortLabel: (id: string, label: string) =>
    desktop
      ? invoke<string | null>("site_label_set", { id, label })
      : desktopOnly(),
  sites: () =>
    desktop
      ? invoke<Site[]>("sites_list")
      : Promise.resolve(demo ? [...demoSites] : []),
  addSite: (url: string) =>
    desktop ? invoke<void>("site_add", { url }) : desktopOnly(),
  switchSite: (id: string) =>
    demo ? Promise.resolve() : invoke<void>("site_switch", { id }),
  onboarding: () =>
    desktop ? invoke<void>("show_onboarding") : Promise.resolve(),
  shellView: (view: ShellView) =>
    desktop ? invoke<void>("shell_view", { view }) : Promise.resolve(),
  reauthenticate: (id: string) =>
    desktop ? invoke<void>("site_reauthenticate", { id }) : desktopOnly(),
  removeSite: (id: string) =>
    desktop ? invoke<void>("site_remove", { id }) : desktopOnly(),
  settings: (): Promise<SavedSettings> =>
    desktop
      ? invoke<Partial<SavedSettings>>("settings_get").then(settingsFrom)
      : Promise.resolve({ ...previewSettings }),
  /** One command saves any one setting; Rust returns them all as saved. */
  saveSettings: (change: Partial<Settings>): Promise<SavedSettings> => {
    if (desktop)
      return invoke<Partial<SavedSettings>>("settings_set", {
        ...change,
      }).then(settingsFrom);
    previewSettings = { ...previewSettings, ...change };
    return Promise.resolve({ ...previewSettings });
  },
  nav: (action: NavAction) =>
    demo ? Promise.resolve() : invoke<void>("nav", { action }),
};

export function message(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}
