import { beforeEach, expect, it, vi } from "vitest";
import { useSites } from "./sites";
import { ipc } from "../ipc";
vi.mock("../ipc", () => ({ ipc: { sites: vi.fn() } }));
beforeEach(() => {
  useSites.setState({ sites: [], active: null });
});
it("refreshes metadata from Rust and preserves the selected site", async () => {
  vi.mocked(ipc.sites).mockResolvedValue([
    {
      id: "insp",
      wwwroot: "https://moodle.insp.gouv.fr/",
      name: "INSP",
      userId: 1,
      fullName: "Learner",
    },
  ]);
  useSites.getState().select("insp");
  await useSites.getState().refresh();
  expect(useSites.getState().sites[0].name).toBe("INSP");
  expect(useSites.getState().active).toBe("insp");
});
it("keeps saved sites intact when storage fails", async () => {
  vi.mocked(ipc.sites).mockRejectedValue(new Error("Storage unavailable"));
  await expect(useSites.getState().refresh()).rejects.toThrow(
    "Storage unavailable",
  );
  expect(useSites.getState().sites).toEqual([]);
});
