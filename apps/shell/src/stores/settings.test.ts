import { beforeEach, expect, it, vi } from "vitest";
import { applyAppearance, useSettings } from "./settings";
import { ipc, settingsFrom } from "../ipc";

vi.mock("../ipc", async (actual) => ({
  ...(await actual<typeof import("../ipc")>()),
  ipc: { settings: vi.fn(), saveSettings: vi.fn() },
}));

beforeEach(() => {
  vi.mocked(ipc.saveSettings).mockReset();
  useSettings.setState({
    appearance: "system",
    language: "system",
    saving: { appearance: false, language: false },
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
  vi.mocked(ipc.settings).mockResolvedValue({
    appearance: "dark",
    language: "fr",
  });
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
  });
  expect(
    settingsFrom({ appearance: "sepia", language: "de" } as never),
  ).toEqual({ appearance: "system", language: "system" });
});

it("keeps the value Rust saved, sending only the changed setting", async () => {
  vi.mocked(ipc.saveSettings).mockResolvedValue({
    appearance: "light",
    language: "system",
  });
  await useSettings.getState().setAppearance("light");
  expect(ipc.saveSettings).toHaveBeenCalledWith({ appearance: "light" });
  expect(useSettings.getState()).toMatchObject({
    appearance: "light",
    saving: { appearance: false, language: false },
    error: null,
  });
});

it("saves the language through the same command", async () => {
  vi.mocked(ipc.saveSettings).mockResolvedValue({
    appearance: "system",
    language: "fr",
  });
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
