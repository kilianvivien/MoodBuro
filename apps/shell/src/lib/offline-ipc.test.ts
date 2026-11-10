import { beforeEach, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: vi.fn(),
}));
import { invoke } from "@tauri-apps/api/core";
import { ipc } from "../ipc";
const target = { siteId: "site", courseId: 7, userId: 42 };
beforeEach(() => vi.mocked(invoke).mockReset());
it("sends only account/course identity and opaque resource keys when saving", async () => {
  vi.mocked(invoke).mockResolvedValue([]);
  await ipc.offlineSave(target, ["a".repeat(64)]);
  expect(invoke).toHaveBeenCalledWith("offline_save", {
    ...target,
    keys: ["a".repeat(64)],
  });
});
it("binds offline Open/Finder to the backend target DTO and explicit risky-file confirmation", async () => {
  vi.mocked(invoke).mockResolvedValue(undefined);
  await ipc.offlineOpen(target, "copy", false, true);
  expect(invoke).toHaveBeenCalledWith("offline_open", {
    target,
    copyId: "copy",
    reveal: false,
    confirmed: true,
  });
  await ipc.offlineOpen(target, "copy", true);
  expect(invoke).toHaveBeenLastCalledWith("offline_open", {
    target,
    copyId: "copy",
    reveal: true,
    confirmed: false,
  });
});

it("scopes reminder changes to the saved account and both explicit offset choices", async () => {
  vi.mocked(invoke).mockResolvedValue(undefined);
  await ipc.setReminders("a", 7, true, false);
  expect(invoke).toHaveBeenCalledWith("reminders_set", {
    siteId: "a",
    userId: 7,
    hour: true,
    day: false,
  });
});
