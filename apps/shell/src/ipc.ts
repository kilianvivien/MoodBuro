import { invoke, isTauri } from "@tauri-apps/api/core";

export interface Site {
  id: string;
  wwwroot: string;
  name: string;
  userId: number;
  fullName: string;
  shortLabel?: string | null;
}

export type Appearance = "system" | "light" | "dark";
/** UI language; "system" follows the Mac's preferred languages. */
export type Language = "system" | "fr" | "en";
export interface Settings {
  appearance: Appearance;
  language: Language;
}
export type ShellView = "onboarding" | "settings";
export type NavAction = "back" | "forward" | "reload" | "open_in_browser";

export const desktop = isTauri();

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
  setShortLabel: (id: string, label: string) =>
    desktop
      ? invoke<string | null>("site_label_set", { id, label })
      : desktopOnly(),
  sites: () => (desktop ? invoke<Site[]>("sites_list") : Promise.resolve([])),
  addSite: (url: string) =>
    desktop ? invoke<void>("site_add", { url }) : desktopOnly(),
  switchSite: (id: string) => invoke<void>("site_switch", { id }),
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
  nav: (action: NavAction) => invoke<void>("nav", { action }),
};

export function message(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}
