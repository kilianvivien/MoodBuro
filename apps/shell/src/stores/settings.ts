import { create } from "zustand";
import {
  ipc,
  message,
  type Appearance,
  type FileHandling,
  type Language,
  type Settings,
} from "../ipc";

/** Light/dark follow the system unless the user pins one; CSS reads data-theme. */
export function applyAppearance(
  appearance: Appearance,
  root: { dataset: DOMStringMap } | undefined = globalThis.document
    ?.documentElement,
) {
  if (!root) return;
  if (appearance === "system") delete root.dataset.theme;
  else root.dataset.theme = appearance;
}

type Field = keyof Settings;

interface SettingsState extends Settings {
  loaded: boolean;
  /** Per setting, so saving one never blocks or rolls back the other. */
  saving: Record<Field, boolean>;
  /** Raw backend message; views translate it. */
  error: string | null;
  /** The setting whose save failed, so the message shows under its group. */
  errorField: Field | null;
  /** The saved ⌘⇧M shortcut could not be registered when the app started. */
  shortcutUnavailable: boolean;
  load: () => Promise<void>;
  setAppearance: (appearance: Appearance) => Promise<void>;
  setLanguage: (language: Language) => Promise<void>;
  setPdf: (handling: FileHandling) => Promise<void>;
  setImages: (handling: FileHandling) => Promise<void>;
  setGlobalShortcut: (enabled: boolean) => Promise<void>;
}

const idle: Record<Field, boolean> = {
  appearance: false,
  language: false,
  pdf: false,
  images: false,
  globalShortcut: false,
};

export const useSettings = create<SettingsState>((set, get) => {
  /** Optimistic so controls, theme and language respond immediately. */
  async function save<K extends Field>(field: K, value: Settings[K]) {
    const previous = get()[field];
    const apply = (next: Settings[K]) => {
      if (field === "appearance") applyAppearance(next as Appearance);
    };
    apply(value);
    set((state) => ({
      [field]: value,
      saving: { ...state.saving, [field]: true },
      error: null,
      errorField: null,
    }));
    try {
      const saved = await ipc.saveSettings({ [field]: value });
      // Only this field: another setting may be mid-save.
      apply(saved[field]);
      set((state) => ({
        [field]: saved[field],
        saving: { ...state.saving, [field]: false },
        // A successful toggle re-registered (or released) the shortcut.
        ...(field === "globalShortcut"
          ? { shortcutUnavailable: saved.shortcutUnavailable }
          : {}),
      }));
    } catch (cause) {
      apply(previous);
      set((state) => ({
        [field]: previous,
        saving: { ...state.saving, [field]: false },
        error: message(cause),
        errorField: field,
      }));
    }
  }

  return {
    appearance: "system",
    language: "system",
    pdf: "inline",
    images: "inline",
    globalShortcut: false,
    shortcutUnavailable: false,
    loaded: false,
    saving: idle,
    error: null,
    errorField: null,
    load: async () => {
      try {
        const saved = await ipc.settings();
        applyAppearance(saved.appearance);
        set({ ...saved, loaded: true, error: null });
      } catch (cause) {
        set({ loaded: true, error: message(cause) });
      }
    },
    setAppearance: (appearance) => save("appearance", appearance),
    setLanguage: (language) => save("language", language),
    setPdf: (handling) => save("pdf", handling),
    setImages: (handling) => save("images", handling),
    setGlobalShortcut: (enabled) => save("globalShortcut", enabled),
  };
});
