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
    const sites = await ipc.sites();
    // A removed site must never stay selected.
    set(({ active }) => ({
      sites,
      active: sites.some((site) => site.id === active) ? active : null,
    }));
  },
  select: (active) => set({ active }),
}));
