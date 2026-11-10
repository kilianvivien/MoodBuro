import { beforeEach, expect, it, vi } from "vitest";
import { applyAppearance, useSettings } from "./settings";
import { ipc, settingsFrom, type SavedSettings } from "../ipc";

const saved = (change: Partial<SavedSettings> = {}): SavedSettings => ({
  ...settingsFrom(null),
  ...change,
});

vi.mock("../ipc", async (actual) => ({
  ...(await actual<typeof import("../ipc")>()),
  ipc: { settings: vi.fn(), saveSettings: vi.fn() },
}));

beforeEach(() => {
  vi.mocked(ipc.saveSettings).mockReset();
  useSettings.setState({
    appearance: "system",
    language: "system",
    pdf: "inline",
    images: "inline",
    globalShortcut: false,
    shortcutUnavailable: false,
    saving: {
      appearance: false,
      language: false,
      pdf: false,
      images: false,
      globalShortcut: false,
    },
    error: null,
  });
});

it("maps the appearance to the root theme attribute", () => {
  const root = { dataset: {} as DOMStringMap };
  applyAppearance("dark", root);
  expect(root.dataset.theme).toBe("dark");
  applyAppearance("system", root);
  expect(root.dataset.theme).toBeUndefined();
});

it("loads both settings and resolves the load promise", async () => {
  vi.mocked(ipc.settings).mockResolvedValue(
    saved({ appearance: "dark", language: "fr" }),
  );
  await expect(useSettings.getState().load()).resolves.toBeUndefined();
  expect(useSettings.getState()).toMatchObject({
    appearance: "dark",
    language: "fr",
    loaded: true,
  });
});

it("fills a language an older settings store does not return", () => {
  expect(settingsFrom({ appearance: "light" })).toEqual({
    appearance: "light",
    language: "system",
    pdf: "inline",
    images: "inline",
    globalShortcut: false,
    shortcutUnavailable: false,
  });
  // Unknown values fall back to the defaults: nothing leaves the page or
  // registers a shortcut unless Rust says the user chose it.
  expect(
    settingsFrom({
      appearance: "sepia",
      language: "de",
      pdf: "print",
      images: "open",
      globalShortcut: "yes",
    } as never),
  ).toMatchObject({
    appearance: "system",
    language: "system",
    pdf: "inline",
    images: "open",
    globalShortcut: false,
  });
});

it("keeps the value Rust saved, sending only the changed setting", async () => {
  vi.mocked(ipc.saveSettings).mockResolvedValue(saved({ appearance: "light" }));
  await useSettings.getState().setAppearance("light");
  expect(ipc.saveSettings).toHaveBeenCalledWith({ appearance: "light" });
  expect(useSettings.getState()).toMatchObject({
    appearance: "light",
    saving: { appearance: false, language: false },
    error: null,
  });
});

it("saves the language through the same command", async () => {
  vi.mocked(ipc.saveSettings).mockResolvedValue(saved({ language: "fr" }));
  const saving = useSettings.getState().setLanguage("fr");
  // Optimistic: the UI switches before Rust answers.
  expect(useSettings.getState()).toMatchObject({
    language: "fr",
    saving: { language: true, appearance: false },
  });
  await saving;
  expect(ipc.saveSettings).toHaveBeenCalledWith({ language: "fr" });
  expect(useSettings.getState().language).toBe("fr");
});

it("rolls back and explains when saving fails", async () => {
  vi.mocked(ipc.saveSettings).mockRejectedValue(new Error("Disk full"));
  await useSettings.getState().setAppearance("dark");
  expect(useSettings.getState()).toMatchObject({
    appearance: "system",
    error: "Disk full",
  });
});

it("rolls back only the setting that failed", async () => {
  useSettings.setState({ appearance: "dark" });
  vi.mocked(ipc.saveSettings).mockRejectedValue("Could not save settings.");
  await useSettings.getState().setLanguage("en");
  expect(useSettings.getState()).toMatchObject({
    appearance: "dark",
    language: "system",
    error: "Could not save settings.",
  });
});

it("saves a file-handling choice without touching the other type", async () => {
  vi.mocked(ipc.saveSettings).mockResolvedValue(saved({ pdf: "open" }));
  await useSettings.getState().setPdf("open");
  expect(ipc.saveSettings).toHaveBeenCalledWith({ pdf: "open" });
  expect(useSettings.getState()).toMatchObject({
    pdf: "open",
    images: "inline",
  });
});

it("leaves the global shortcut off when it cannot be registered", async () => {
  vi.mocked(ipc.saveSettings).mockRejectedValue(
    new Error("The ⌘⇧M shortcut could not be registered."),
  );
  const saving = useSettings.getState().setGlobalShortcut(true);
  expect(useSettings.getState().saving.globalShortcut).toBe(true);
  await saving;
  expect(useSettings.getState()).toMatchObject({
    globalShortcut: false,
    saving: { globalShortcut: false },
    error: "The ⌘⇧M shortcut could not be registered.",
  });
});

it("clears the launch warning once the shortcut registers again", async () => {
  useSettings.setState({ globalShortcut: true, shortcutUnavailable: true });
  vi.mocked(ipc.saveSettings).mockResolvedValue(saved());
  await useSettings.getState().setGlobalShortcut(false);
  vi.mocked(ipc.saveSettings).mockResolvedValue(
    saved({ globalShortcut: true }),
  );
  await useSettings.getState().setGlobalShortcut(true);
  expect(useSettings.getState()).toMatchObject({
    globalShortcut: true,
    shortcutUnavailable: false,
  });
});
