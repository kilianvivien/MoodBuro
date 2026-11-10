import { create } from "zustand";
import {
  ASSISTANT_WIDTH,
  ipc,
  message,
  type AssistantProvider,
  type AssistantStatus,
} from "../ipc";

/** Width as Rust clamps it, so the panel and the Moodle view always agree. */
export function clampWidth(width: number) {
  return Number.isFinite(width)
    ? Math.min(ASSISTANT_WIDTH.max, Math.max(ASSISTANT_WIDTH.min, width))
    : ASSISTANT_WIDTH.initial;
}

interface AssistantState extends AssistantStatus {
  loaded: boolean;
  /** Raw backend message from the last panel-geometry change; views translate it. */
  error: string | null;
  load: () => Promise<void>;
  /** Replaces the saved values with what Rust returned. */
  apply: (status: AssistantStatus) => void;
  configure: (change: {
    provider?: AssistantProvider;
    model?: string;
    consent?: boolean;
  }) => Promise<void>;
  /** Shows or hides the panel; Rust moves the Moodle view before this resolves. */
  setOpen: (open: boolean) => Promise<void>;
  /** `commit` false while the separator is dragged. */
  setWidth: (width: number, commit: boolean) => Promise<void>;
}

export const useAssistant = create<AssistantState>((set, get) => ({
  provider: "albert",
  model: "",
  hasKey: false,
  consent: false,
  panel: { open: false, width: ASSISTANT_WIDTH.initial },
  loaded: false,
  error: null,
  load: async () => {
    try {
      set({ ...(await ipc.assistant()), loaded: true, error: null });
    } catch (cause) {
      set({ loaded: true, error: message(cause) });
    }
  },
  apply: (status) => set({ ...status }),
  configure: async (change) =>
    set({ ...(await ipc.configureAssistant(change)) }),
  setOpen: async (open) => {
    try {
      // Rust first: the panel must not appear under a Moodle view still covering it.
      const panel = await ipc.assistantPanel(open, get().panel.width);
      set({ panel, error: null });
    } catch (cause) {
      set({ error: message(cause) });
    }
  },
  setWidth: async (width, commit) => {
    const { panel } = get();
    const next = clampWidth(width);
    // The shell follows at once; a failed call restores what Rust last accepted.
    set({ panel: { ...panel, width: next } });
    try {
      const saved = await ipc.assistantPanel(panel.open, next, commit);
      if (commit) set({ panel: saved, error: null });
    } catch (cause) {
      set({ panel, error: message(cause) });
    }
  },
}));
