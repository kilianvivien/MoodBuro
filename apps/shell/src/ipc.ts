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
}

export type DeepLinkOutcome =
  { kind: "site"; siteId: string } | { kind: "onboarding"; url: string };

export interface UnreadStatus {
  siteId: string;
  unread: number;
  connection: "checking" | "online" | "offline" | "reauth" | "error";
}

export type Appearance = "system" | "light" | "dark";
/** UI language; "system" follows the Mac's preferred languages. */
export type Language = "system" | "fr" | "en";
export interface Settings {
  appearance: Appearance;
  language: Language;
}
export type ShellView = "onboarding" | "settings" | "downloads";
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

const desktopOnly = () =>
  Promise.reject(
    new Error("Open the desktop app with pnpm tauri dev to manage sign-in."),
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
  downloads: () =>
    desktop ? invoke<Download[]>("downloads_recent") : Promise.resolve([]),
  openDownload: (id: string, confirmed = false) =>
    desktop ? invoke<void>("download_open", { id, confirmed }) : desktopOnly(),
  revealDownload: (id: string) =>
    desktop ? invoke<void>("download_reveal", { id }) : desktopOnly(),
  deepLink: () =>
    desktop
      ? invoke<DeepLinkOutcome | null>("deep_link_open")
      : Promise.resolve(null),
  unread: () =>
    desktop ? invoke<UnreadStatus[]>("unread_list") : Promise.resolve([]),
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
