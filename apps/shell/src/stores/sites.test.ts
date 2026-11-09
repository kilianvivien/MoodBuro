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
      id: "campus",
      wwwroot: "https://moodle.example.edu/",
      name: "Example Campus",
      userId: 1,
      fullName: "Learner",
    },
  ]);
  useSites.getState().select("campus");
  await useSites.getState().refresh();
  expect(useSites.getState().sites[0].name).toBe("Example Campus");
  expect(useSites.getState().active).toBe("campus");
});
it("keeps saved sites intact when storage fails", async () => {
  vi.mocked(ipc.sites).mockRejectedValue(new Error("Storage unavailable"));
  await expect(useSites.getState().refresh()).rejects.toThrow(
    "Storage unavailable",
  );
  expect(useSites.getState().sites).toEqual([]);
});
it("clears the selection when the active site was removed", async () => {
  vi.mocked(ipc.sites).mockResolvedValue([]);
  useSites.getState().select("campus");
  await useSites.getState().refresh();
  expect(useSites.getState().active).toBeNull();
});
