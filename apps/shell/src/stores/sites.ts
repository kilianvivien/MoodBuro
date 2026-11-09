import { create } from "zustand";
import { ipc, type Site } from "../ipc";
export const useSites = create<{
  sites: Site[];
  active: string | null;
  refresh: () => Promise<void>;
  select: (id: string | null) => void;
}>((set) => ({
  sites: [],
  active: null,
  refresh: async () => {
    set({ sites: await ipc.sites() });
  },
  select: (active) => set({ active }),
}));
