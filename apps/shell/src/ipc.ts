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
export type EventKind = "due" | "open" | "user" | "site" | "course" | "other";
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
export interface Settings {
  appearance: Appearance;
  language: Language;
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

/** Fills values an older settings store does not return yet. */
export function settingsFrom(raw: Partial<Settings> | null | undefined) {
  return {
    appearance: appearances.includes(raw?.appearance as Appearance)
      ? (raw!.appearance as Appearance)
      : "system",
    language: languages.includes(raw?.language as Language)
      ? (raw!.language as Language)
      : "system",
  } satisfies Settings;
}

// The browser preview has no Rust side; settings live in memory only and the
// UI says so instead of pretending they were saved.
let previewSettings: Settings = settingsFrom(null);

export const ipc = {
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
  settings: (): Promise<Settings> =>
    desktop
      ? invoke<Partial<Settings>>("settings_get").then(settingsFrom)
      : Promise.resolve({ ...previewSettings }),
  /** One command saves either setting; Rust returns both as saved. */
  saveSettings: (change: Partial<Settings>): Promise<Settings> => {
    if (desktop)
      return invoke<Partial<Settings>>("settings_set", { ...change }).then(
        settingsFrom,
      );
    previewSettings = { ...previewSettings, ...change };
    return Promise.resolve({ ...previewSettings });
  },
  nav: (action: NavAction) =>
    demo ? Promise.resolve() : invoke<void>("nav", { action }),
};

export function message(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}
