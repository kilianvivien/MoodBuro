import { create } from "zustand";
import {
  ipc,
  message,
  type Appearance,
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
  load: () => Promise<void>;
  setAppearance: (appearance: Appearance) => Promise<void>;
  setLanguage: (language: Language) => Promise<void>;
}

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
    }));
    try {
      const saved = await ipc.saveSettings({ [field]: value });
      // Only this field: another setting may be mid-save.
      apply(saved[field]);
      set((state) => ({
        [field]: saved[field],
        saving: { ...state.saving, [field]: false },
      }));
    } catch (cause) {
      apply(previous);
      set((state) => ({
        [field]: previous,
        saving: { ...state.saving, [field]: false },
        error: message(cause),
      }));
    }
  }

  return {
    appearance: "system",
    language: "system",
    loaded: false,
    saving: { appearance: false, language: false },
    error: null,
    load: async () => {
      try {
        const { appearance, language } = await ipc.settings();
        applyAppearance(appearance);
        set({ appearance, language, loaded: true, error: null });
      } catch (cause) {
        set({ loaded: true, error: message(cause) });
      }
    },
    setAppearance: (appearance) => save("appearance", appearance),
    setLanguage: (language) => save("language", language),
  };
});
