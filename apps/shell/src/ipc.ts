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
  state: "downloading" | "complete" | "failed" | "cancelled" | "removed";
  startedAt: number;
  needsConfirmation: boolean;
  canPreview?: boolean;
  bytes?: number | null;
  metadataApplied?: boolean | null;
  courseId?: number | null;
  courseName?: string | null;
}

export interface DownloadProgress {
  id: string;
  received: number;
  total: number | null;
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
  refreshedAt: number;
  available: boolean;
  needsConfirmation: boolean;
}
export interface OfflineOutcome {
  key: string;
  reused: boolean;
  error: string | null;
}
export interface DesktopPreferences {
  menuBarOnly: boolean;
  startAtLogin: boolean;
}
export interface UploadFile {
  id: string;
  name: string;
  bytes: number;
}
export interface DraftAssignment {
  id: number;
  moduleId: number;
  name: string;
  supported: boolean;
}
export interface UploadReview {
  id: string;
  filename: string;
  bytes: number;
  organisation: string;
  course: string;
  assignment: string;
}
export interface DraftUpload {
  id: string;
  siteId: string;
  userId: number;
  courseId: number;
  assignmentId: number;
  moduleId: number;
  filename: string;
  bytes: number;
  state:
    | "started"
    | "uploaded"
    | "saving"
    | "saved"
    | "uncertain"
    | "failed"
    | "reviewed";
  createdAt: number;
  draftId: number | null;
}
export interface CalendarHandoff {
  siteId: string;
  userId: number;
  openedAt: number;
  provider: string;
}
export interface CourseSync {
  siteId: string;
  userId: number;
  courseId: number;
  generation: number;
  interval: number;
  state:
    | "paused"
    | "waiting"
    | "running"
    | "idle"
    | "error"
    | "cancelled"
    | "interrupted";
  nextRun: number;
  refreshedAt: number;
  failures: number;
  error: string | null;
  keys: string[];
  pending: number;
  missing: number;
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
  | "calendar-export"
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
  | "quick-open"
  | "quick-open-close"
  | "bookmarks"
  | "uploads";
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
const demoUploads: DraftUpload[] = [];
const demoApprovals = new Map<
  string,
  { target: OfflineCourse; assignment: number; review: UploadReview }
>();
const demoStopped = new Set<string>();
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
      ["d7", "demo-2", "Edt exemple de la semaine.pdf", "complete", 3 * hour],
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
const demoCourseSyncs: CourseSync[] = [];
const demoCalendarHandoffs: CalendarHandoff[] = [];
const demoDesktop: DesktopPreferences = {
  menuBarOnly: false,
  startAtLogin: false,
};
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
export interface Bookmark {
  id: string;
  siteId: string;
  userId: number;
  name: string;
  path: string;
  icon: string;
}
export interface SessionRecovery {
  siteId: string;
  phase: "waiting" | "retrying" | "failed";
  retryAt: number;
}

let demoBookmarks: Bookmark[] = demo
  ? demoData.courses.slice(0, 7).map((course, index) => ({
      id: `bookmark-${index}`,
      siteId: course.siteId,
      userId: demoSites.find((site) => site.id === course.siteId)!.userId,
      name: course.name,
      path: `course/view.php?id=${course.id}`,
      icon: [
        "book",
        "star",
        "file",
        "message",
        "calendar",
        "folder",
        "graduation",
      ][index],
    }))
  : [];

/** A sidebar item that has a context menu. */
export type SidebarTarget =
  | { kind: "inbox" | "courses" | "agenda" | "add" | "settings" }
  | { kind: "site" | "bookmark"; id: string };

export const ipc = {
  /** Shows the item's native context menu; a choice arrives as `sidebar://action`. */
  sidebarMenu: (target: SidebarTarget) =>
    desktop
      ? invoke<void>("sidebar_menu", {
          kind: target.kind,
          id: "id" in target ? target.id : null,
        })
      : Promise.resolve(),
  bookmarkMenu: () =>
    desktop ? invoke<void>("bookmark_menu") : Promise.resolve(),
  bookmarks: () =>
    desktop
      ? invoke<Bookmark[]>("bookmarks_list")
      : Promise.resolve(demoBookmarks.map((b) => ({ ...b }))),
  bookmarkCapture: (): Promise<Bookmark> =>
    desktop
      ? invoke<Bookmark>("bookmark_capture")
      : demo
        ? Promise.resolve({ ...demoBookmarks[0], id: crypto.randomUUID() })
        : desktopFeature(),
  bookmarkSave: (bookmark: Bookmark): Promise<void> => {
    if (desktop) return invoke<void>("bookmark_save", { bookmark });
    if (!demo) return desktopFeature();
    demoBookmarks = demoBookmarks.some((b) => b.id === bookmark.id)
      ? demoBookmarks.map((b) => (b.id === bookmark.id ? { ...bookmark } : b))
      : [...demoBookmarks, { ...bookmark }];
    return Promise.resolve();
  },
  bookmarkDelete: (id: string): Promise<void> => {
    if (desktop) return invoke<void>("bookmark_delete", { id });
    if (!demo) return desktopFeature();
    demoBookmarks = demoBookmarks.filter((b) => b.id !== id);
    return Promise.resolve();
  },
  bookmarkOpen: (id: string): Promise<void> =>
    desktop
      ? invoke<void>("bookmark_open", { id })
      : demo
        ? Promise.resolve()
        : desktopFeature(),
  sessionStatus: () =>
    desktop ? invoke<SessionRecovery[]>("session_status") : Promise.resolve([]),
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
  assistantKeyHelp: (provider: AssistantProvider, guide: boolean) => {
    if (desktop) return invoke<void>("assistant_key_help", { provider, guide });
    const url =
      provider === "albert"
        ? guide
          ? "https://guides.ia.numerique.gouv.fr/albert-api"
          : "https://albert.playground.etalab.gouv.fr/"
        : guide
          ? "https://docs.mistral.ai/getting-started/quickstarts/studio/activate-and-generate-api-key"
          : "https://console.mistral.ai/api-keys";
    window.open(url, "_blank", "noopener,noreferrer");
    return Promise.resolve();
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
      const existing = demoOfflineCopies.find(
        (c) =>
          c.resourceKey === key &&
          c.siteId === course.siteId &&
          c.courseId === course.courseId,
      );
      const refreshedAt = Math.floor(Date.now() / 1000);
      if (existing) existing.refreshedAt = refreshedAt;
      else
        demoOfflineCopies.push({
          id: crypto.randomUUID(),
          siteId: course.siteId,
          courseId: course.courseId,
          resourceKey: key,
          moduleId: resource.moduleId,
          filename: resource.filename,
          sectionName: resource.sectionName,
          bytes: resource.bytes ?? 1024,
          savedAt: refreshedAt,
          refreshedAt,
          available: true,
          needsConfirmation: false,
        });
      return { key, reused: !!existing, error: null };
    });
  },
  courseSyncList: () =>
    desktop
      ? invoke<CourseSync[]>("course_sync_list")
      : Promise.resolve(
          demoCourseSyncs.map((job) => ({ ...job, keys: [...job.keys] })),
        ),
  courseSyncConfigure: async (
    target: OfflineCourse,
    keys: string[],
    interval: number,
  ): Promise<void> => {
    if (desktop)
      return invoke("course_sync_configure", { target, keys, interval });
    if (!demo) return desktopFeature();
    const old = demoCourseSyncs.findIndex(
      (job) => job.siteId === target.siteId && job.courseId === target.courseId,
    );
    if (old >= 0) demoCourseSyncs.splice(old, 1);
    demoCourseSyncs.push({
      ...target,
      generation: 1,
      interval,
      state: interval ? "waiting" : "paused",
      nextRun: Math.floor(Date.now() / 1000),
      refreshedAt: 0,
      failures: 0,
      error: null,
      keys: [...keys],
      pending: keys.length,
      missing: 0,
    });
  },
  courseSyncControl: async (
    target: OfflineCourse,
    cancel: boolean,
  ): Promise<void> => {
    if (desktop) return invoke("course_sync_control", { target, cancel });
    if (!demo) return desktopFeature();
    const job = demoCourseSyncs.find(
      (job) => job.siteId === target.siteId && job.courseId === target.courseId,
    );
    if (job) job.state = cancel ? "cancelled" : "waiting";
  },
  offlineRemove: async (
    target: OfflineCourse,
    copyId: string,
  ): Promise<"archived" | "kept" | "missing"> => {
    if (desktop) return invoke("offline_remove", { target, copyId });
    if (!demo) return desktopFeature();
    const index = demoOfflineCopies.findIndex(
      (copy) =>
        copy.id === copyId &&
        copy.siteId === target.siteId &&
        copy.courseId === target.courseId,
    );
    if (index < 0)
      throw new Error(
        "This offline file is unavailable. Save a fresh copy from the course.",
      );
    demoOfflineCopies.splice(index, 1);
    return "archived";
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
  popupContext: (): Promise<PopupContext> =>
    desktop
      ? invoke<PopupContext>("popup_context")
      : Promise.resolve({
          ...previewSettings,
          siteName: demo ? demoSites[0].name : "",
          canGoBack: false,
          canGoForward: false,
        }),
  popupNav: (action: NavAction | "downloads") =>
    desktop ? invoke<void>("popup_nav", { action }) : desktopFeature(),
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
  cancelDownload: (id: string) =>
    desktop ? invoke<void>("download_cancel", { id }) : desktopFeature(),
  removeDownloadFile: (id: string, confirmed: boolean) => {
    if (desktop) return invoke<void>("download_remove_file", { id, confirmed });
    if (!demo) return desktopFeature();
    const row = demoData.downloads.find((r) => r.id === id);
    if (!confirmed || row?.state !== "complete")
      return Promise.reject(new Error("This download is unavailable."));
    row.state = "removed";
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
  desktopPreferences: () =>
    desktop
      ? invoke<DesktopPreferences>("desktop_preferences")
      : Promise.resolve({ ...demoDesktop }),
  desktopPreferencesSet: async (
    change: Partial<DesktopPreferences>,
  ): Promise<DesktopPreferences> => {
    if (desktop) return invoke("desktop_preferences_set", change);
    if (!demo) return desktopFeature();
    Object.assign(demoDesktop, change);
    return { ...demoDesktop };
  },
  finderMoodleFolder: () =>
    desktop
      ? invoke<void>("finder_moodle_folder")
      : demo
        ? Promise.resolve()
        : desktopFeature(),
  uploadPick: async (): Promise<UploadFile | null> => {
    if (desktop) return invoke("upload_pick");
    if (!demo) return desktopFeature();
    return { id: "demo-file", name: "Essay.docx", bytes: 24576 };
  },
  uploadFiles: () =>
    desktop ? invoke<UploadFile[]>("upload_files") : Promise.resolve([]),
  uploadAssignments: async (
    target: OfflineCourse,
  ): Promise<DraftAssignment[]> =>
    desktop
      ? invoke("upload_assignments", { target })
      : demo
        ? [
            { id: 42, moduleId: 84, name: "Policy brief", supported: true },
            { id: 43, moduleId: 85, name: "Group project", supported: false },
          ]
        : [],
  uploadPrepare: async (
    target: OfflineCourse,
    assignment: number,
    file: string,
  ): Promise<UploadReview> => {
    if (desktop) return invoke("upload_prepare", { target, assignment, file });
    if (!demo) return desktopFeature();
    const review = {
      id: crypto.randomUUID(),
      filename: "Essay.docx",
      bytes: 24576,
      organisation:
        demoSites.find((s) => s.id === target.siteId)?.name ?? "Example Campus",
      course:
        demoData.courses.find((c) => c.id === target.courseId)?.name ??
        "Course",
      assignment: "Policy brief",
    };
    demoApprovals.set(review.id, { target, assignment, review });
    return review;
  },
  uploadConfirm: async (
    approval: string,
    confirmed: boolean,
  ): Promise<void> => {
    if (desktop) return invoke("upload_confirm", { approval, confirmed });
    if (!demo) return desktopFeature();
    const a = demoApprovals.get(approval);
    demoApprovals.delete(approval);
    if (!confirmed || !a)
      throw new Error(
        "The upload approval expired. Review the destination again.",
      );
    const row: DraftUpload = {
      id: approval,
      ...a.target,
      assignmentId: a.assignment,
      moduleId: 84,
      filename: a.review.filename,
      bytes: a.review.bytes,
      state: "started",
      createdAt: Math.floor(Date.now() / 1000),
      draftId: null,
    };
    demoUploads.unshift(row);
    await pause(600);
    if (demoStopped.delete(approval)) {
      row.state = "uncertain";
      throw new Error("Upload stopped. Check Moodle before retrying.");
    }
    row.state = "saved";
  },
  uploadStop: (approval: string) => {
    if (desktop) return invoke<void>("upload_stop", { approval });
    demoStopped.add(approval);
    return Promise.resolve();
  },
  uploadHistory: () =>
    desktop
      ? invoke<DraftUpload[]>("upload_history")
      : Promise.resolve(demoUploads.map((r) => ({ ...r }))),
  uploadOpen: (target: OfflineCourse, assignment: number) =>
    desktop
      ? invoke<void>("upload_open", { target, assignment })
      : demo
        ? Promise.resolve()
        : desktopFeature(),
  uploadReviewed: (id: string, confirmed: boolean) =>
    desktop
      ? invoke<void>("upload_reviewed", { id, confirmed })
      : (() => {
          const row = demoUploads.find((r) => r.id === id);
          if (confirmed && row?.state === "uncertain") row.state = "reviewed";
          return Promise.resolve();
        })(),
  calendarHandoffs: () =>
    desktop
      ? invoke<CalendarHandoff[]>("calendar_handoffs")
      : Promise.resolve(demoCalendarHandoffs.map((h) => ({ ...h }))),
  subscribeCalendar: async (
    siteId: string,
    userId: number,
    confirmed: boolean,
    repeat: boolean,
  ): Promise<void> => {
    if (desktop)
      return invoke("calendar_subscribe", {
        siteId,
        userId,
        confirmed,
        repeat,
      });
    if (!demo) return desktopFeature();
    if (!confirmed)
      throw new Error(
        "Confirm sharing the private calendar link with Calendar first.",
      );
    const index = demoCalendarHandoffs.findIndex((h) => h.siteId === siteId);
    if (index >= 0 && !repeat)
      throw new Error(
        "Calendar setup was already opened. Confirm reopening it to avoid duplicates.",
      );
    if (index >= 0) demoCalendarHandoffs.splice(index, 1);
    demoCalendarHandoffs.push({
      siteId,
      userId,
      openedAt: Math.floor(Date.now() / 1000),
      provider: "moodle",
    });
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

export interface UpdateStatus {
  configured: boolean;
  automatic: boolean;
  phase: string;
  version: string | null;
  notes: string | null;
  received: number;
  total: number | null;
}
export const updates = {
  status: (): Promise<UpdateStatus> =>
    isTauri()
      ? invoke("updates_status")
      : Promise.resolve({
          configured: false,
          automatic: true,
          phase: "unconfigured",
          version: null,
          notes: null,
          received: 0,
          total: null,
        }),
  check: (): Promise<void> => invoke("updates_check"),
  install: (): Promise<void> => invoke("updates_install"),
  preference: (enabled: boolean): Promise<void> =>
    invoke("updates_preference", { enabled }),
  restart: (): Promise<void> => invoke("updates_restart"),
};
