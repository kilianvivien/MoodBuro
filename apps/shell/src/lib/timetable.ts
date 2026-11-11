import { invoke, isTauri } from "@tauri-apps/api/core";
import { previewFlags } from "./window";

/** Timetable weeks read by the native engine (INSP Moodle only). */
export type Cohort = "ALL" | "FI" | "CIL" | "OFF";
export interface TimetableOptions {
  cohort: Cohort;
  prefix: string;
  alarm: number | null;
  splitByModule: boolean;
}
export interface TimetableChoices {
  groups: Record<string, string>;
  halfPromotion: Record<string, "A" | "B">;
  rooms: Record<string, string>;
  times: Record<string, { start: string; end: string }>;
  sport: Record<string, "keep" | "exclude">;
}
export interface Session {
  uid: string;
  title: string;
  module: string | null;
  location: string | null;
  locationNote: string | null;
  speakers: string[];
  cohortLabel: string | null;
  preparation: string | null;
  notes: string[];
  remote: boolean;
  /** `YYYY-MM-DD`, school (Europe/Paris) date. */
  date: string;
  lastDate: string;
  /** `HH:MM`, Europe/Paris wall time. */
  start: string | null;
  end: string | null;
  startAt: number | null;
  endAt: number | null;
  allDay: boolean;
  tentative: boolean;
  links: string[];
  cancelled: boolean;
}
export interface QuestionSession {
  uid: string;
  sessionUid?: string;
  title: string | null;
  date: string | null;
  start: string | null;
  end: string | null;
  location: string | null;
  cohortLabel: string | null;
  module: string | null;
  options?: {
    label: string | null;
    location: string | null;
    start: string | null;
    end: string | null;
  }[];
}
export interface Questions {
  groups: QuestionSession[];
  participation: QuestionSession[];
  halfPromotion: { weekStart: string | null; sessions: QuestionSession[] }[];
  times: QuestionSession[];
  sport: { weekStart: string | null; sessions: QuestionSession[] }[];
  lv1: QuestionSession[];
  lv1Rooms: string[];
}
export interface WeekDiff {
  added: string[];
  moved: string[];
  changed: string[];
  removed: string[];
  unchanged: number;
}
export type WeekMode = "new" | "update" | "unchanged" | "edit" | "saved";
export interface WeekView {
  siteId: string;
  weekKey: string;
  weekStart: string;
  weekEnd: string;
  promotion: string | null;
  filename: string;
  mode: WeekMode;
  draftId: string | null;
  draftRevision: number | null;
  savedRevision: number | null;
  sessions: Session[];
  questions: Questions | null;
  pending: number;
  warnings: {
    code: string | null;
    message: string | null;
    uid: string | null;
  }[];
  diff: WeekDiff | null;
  options: TimetableOptions;
  choices: TimetableChoices;
  choicesReset: boolean;
}
export interface WeekSummary {
  weekKey: string;
  weekStart: string;
  weekEnd: string;
  promotion: string | null;
  cohort: string;
  filename: string;
  revision: number;
  updatedAt: number;
  sessions: number;
  tentative: number;
}
export type TimetableSource =
  | { kind: "courseResource"; courseId: number; resourceKey: string }
  | { kind: "offlineCopy"; courseId: number; copyId: string }
  | { kind: "download"; downloadId: string }
  | { kind: "linkIntent"; intentId: string };
export interface ExportedWeek {
  filename: string;
  sessions: number;
  cancelled: number;
  revealed: boolean;
  opened: boolean;
  historySaved: boolean;
}
export interface CalendarSync {
  enabled: boolean;
  access: "granted" | "denied" | "notDetermined";
}
export interface TimetableStatus {
  sites: string[];
  engine: string;
  engineAvailable: boolean;
  rules: string;
  adapterContract: number;
}

/** A file name that looks like a weekly timetable (a hint; the reader decides). */
export function looksLikeTimetable(name: string): boolean {
  const folded = name.toLowerCase();
  return (
    folded.endsWith(".pdf") &&
    (/(^|[^a-z])edt([^a-z]|$)/.test(folded) ||
      folded.includes("emploi du temps") ||
      /\bdu\s+\d{1,2}[ .\-/]/.test(folded))
  );
}

export const emptyChoices = (): TimetableChoices => ({
  groups: {},
  halfPromotion: {},
  rooms: {},
  times: {},
  sport: {},
});
export const defaultOptions = (): TimetableOptions => ({
  cohort: "ALL",
  prefix: "",
  alarm: null,
  splitByModule: false,
});

const desktop = isTauri();
const demo = !desktop && import.meta.env.DEV && previewFlags().demo;

// ---- Browser preview (`?demo`): one neutral sample week, in memory. ----
const iso = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
function monday(offsetWeeks = 0) {
  const now = new Date();
  const day = (now.getDay() + 6) % 7;
  return new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate() - day + offsetWeeks * 7,
  );
}
function demoSession(
  uid: string,
  dayIndex: number,
  start: string | null,
  end: string | null,
  title: string,
  extra: Partial<Session> = {},
): Session {
  const day = monday();
  day.setDate(day.getDate() + dayIndex);
  const at = (time: string | null) => {
    if (!time) return null;
    const [h, m] = time.split(":").map(Number);
    return Math.floor(
      new Date(
        day.getFullYear(),
        day.getMonth(),
        day.getDate(),
        h,
        m,
      ).getTime() / 1000,
    );
  };
  return {
    uid,
    title,
    module: "Parcours exemple",
    location: "Salle B",
    locationNote: null,
    speakers: ["Mme Alice EXEMPLE"],
    cohortLabel: "Promotion",
    preparation: null,
    notes: [],
    remote: false,
    date: iso(day),
    lastDate: iso(day),
    start,
    end,
    startAt: at(start),
    endAt: at(end),
    allDay: !start,
    tentative: false,
    links: [],
    cancelled: false,
    ...extra,
  };
}
const demoAll = (): Session[] => [
  demoSession("demo-1", 0, "09:00", "10:30", "Conférence d’ouverture", {
    location: "Amphi 1",
  }),
  demoSession("demo-2", 0, "10:00", "12:00", "Atelier en parallèle", {
    location: "C101",
  }),
  demoSession("demo-3", 1, "14:00", "17:00", "Études de cas", {
    location: null,
    locationNote: "Selon le groupe",
    cohortLabel: "Groupe A / Groupe B",
  }),
  demoSession("demo-4", 2, null, null, "Journée en autonomie"),
  demoSession("demo-5", 3, "09:30", "12:30", "Séminaire exemple", {
    speakers: ["M. Bob EXEMPLE"],
  }),
  demoSession("demo-6", 4, "13:30", "15:00", "Atelier facultatif", {
    cohortLabel: "Élèves inscrits",
    tentative: true,
    links: ["Lien d’inscription"],
  }),
  // Dense days like real weeks: three lanes, short and long sessions.
  demoSession(
    "demo-7",
    3,
    "10:00",
    "11:15",
    "La gestion des situations de crise en exemple",
    {
      location: "Auditorium exemple",
    },
  ),
  demoSession(
    "demo-8",
    3,
    "11:30",
    "12:15",
    "La gestion des situations de crise (suite)",
    {
      locationNote: "Selon le groupe",
      location: null,
    },
  ),
  demoSession("demo-9", 3, "10:30", "11:00", "Point d’étape", {
    location: "C104",
  }),
  demoSession("demo-10", 0, "13:00", "13:45", "Café exemple", {
    location: "Salle de réception",
    tentative: true,
  }),
  demoSession("demo-11", 0, "18:00", "18:30", "Association des élèves", {
    location: "Hall",
  }),
  demoSession(
    "demo-12",
    1,
    "19:00",
    "20:30",
    "Module de renforcement musculaire",
    {
      location: "Gymnase exemple",
      tentative: true,
    },
  ),
  demoSession("demo-13", 2, null, null, "Férié"),
  demoSession(
    "demo-14",
    4,
    "09:00",
    "12:30",
    "Parcours communication : atelier pratique en sous-groupes",
    {
      location: null,
      locationNote: "Selon le groupe",
    },
  ),
];
const demoQuestions = (): Questions => ({
  groups: [
    {
      uid: "demo-3",
      title: "Études de cas",
      date: demoAll()[2].date,
      start: "14:00",
      end: "17:00",
      location: null,
      cohortLabel: "Groupe A / Groupe B",
      module: "Parcours exemple",
      options: [
        { label: "Groupe A", location: "C201", start: "14:00", end: "17:00" },
        { label: "Groupe B", location: "C202", start: "14:00", end: "17:00" },
      ],
    },
  ],
  participation: [
    {
      uid: "demo-6",
      title: "Atelier facultatif",
      date: demoAll()[5].date,
      start: "13:30",
      end: "15:00",
      location: "Salle B",
      cohortLabel: "Élèves inscrits",
      module: "Parcours exemple",
    },
  ],
  halfPromotion: [],
  times: [],
  sport: [],
  lv1: [],
  lv1Rooms: [],
});
function demoApply(choices: TimetableChoices): Session[] {
  return demoAll()
    .filter((s) => choices.groups[s.uid] !== "__exclude__")
    .map((s) => {
      const group = demoQuestions().groups[0].options?.find(
        (o) => o.label === choices.groups[s.uid],
      );
      if (s.uid === "demo-3" && group)
        return {
          ...s,
          location: group.location,
          locationNote: null,
          cohortLabel: group.label,
        };
      if (s.uid === "demo-6" && choices.groups[s.uid] === "")
        return { ...s, tentative: false };
      return s;
    });
}
function demoPending(choices: TimetableChoices) {
  return ["demo-3", "demo-6"].filter((uid) => !(uid in choices.groups)).length;
}
const demoKey = () => `v1|${iso(monday())}`;
let demoSaved: WeekView | null = null;
let demoDraft: WeekView | null = null;
let demoPreferences = defaultOptions();
let demoCalendar = false;
function demoView(
  mode: WeekMode,
  choices: TimetableChoices,
  options: TimetableOptions,
): WeekView {
  const start = monday();
  const end = monday();
  end.setDate(end.getDate() + 4);
  return {
    siteId: "demo-2",
    weekKey: demoKey(),
    weekStart: iso(start),
    weekEnd: iso(end),
    promotion: "2026-2028",
    filename: "Edt exemple.pdf",
    mode,
    draftId: mode === "saved" ? null : "demo-draft",
    draftRevision:
      mode === "saved" ? null : (demoDraft?.draftRevision ?? 0) + 1,
    savedRevision: demoSaved?.savedRevision ?? null,
    sessions: demoApply(choices),
    questions: mode === "saved" ? null : demoQuestions(),
    pending: demoPending(choices),
    warnings: [],
    diff: null,
    options,
    choices,
    choicesReset: false,
  };
}
const delay = <T>(value: T, ms = 350) =>
  new Promise<T>((resolve) => setTimeout(() => resolve(value), ms));
const unavailable = () => Promise.reject(new Error("ENGINE_UNAVAILABLE"));

export const timetable = {
  status: (): Promise<TimetableStatus> =>
    desktop
      ? invoke("timetable_status")
      : Promise.resolve({
          sites: demo ? ["demo-2"] : [],
          engine: "0.2.0",
          engineAvailable: false,
          rules: "demo",
          adapterContract: 1,
        }),
  prepare: (
    siteId: string,
    userId: number,
    source: TimetableSource,
    weekStart: string | null = null,
  ): Promise<WeekView> => {
    if (desktop)
      return invoke("timetable_prepare", {
        request: { siteId, userId, source, weekStart },
      });
    if (!demo) return unavailable();
    demoDraft = demoView(
      demoSaved ? "update" : "new",
      demoSaved?.choices ?? emptyChoices(),
      demoPreferences,
    );
    return delay(demoDraft, 900);
  },
  review: (
    draftId: string,
    expectedDraftRevision: number,
    choices: TimetableChoices,
    options: TimetableOptions,
  ): Promise<WeekView> => {
    if (desktop)
      return invoke("timetable_review", {
        draftId,
        expectedDraftRevision,
        choices,
        options,
      });
    if (
      !demo ||
      !demoDraft ||
      demoDraft.draftRevision !== expectedDraftRevision
    )
      return Promise.reject(new Error("STALE_DRAFT"));
    demoDraft = demoView(demoDraft.mode, choices, options);
    return delay(demoDraft, 200);
  },
  commit: (
    draftId: string,
    expectedDraftRevision: number,
    requestId: string,
  ): Promise<WeekView> => {
    if (desktop)
      return invoke("timetable_commit", {
        draftId,
        expectedDraftRevision,
        requestId,
      });
    if (!demo || !demoDraft) return Promise.reject(new Error("STALE_DRAFT"));
    demoSaved = {
      ...demoDraft,
      mode: "saved",
      draftId: null,
      draftRevision: null,
      questions: null,
      savedRevision: (demoSaved?.savedRevision ?? 0) + 1,
      pending: demoDraft.sessions.filter((s) => s.tentative).length,
    };
    demoPreferences = { ...demoDraft.options, splitByModule: false };
    demoDraft = null;
    return delay(demoSaved, 200);
  },
  discard: (draftId: string): Promise<void> => {
    if (desktop) return invoke("timetable_discard", { draftId });
    demoDraft = null;
    return Promise.resolve();
  },
  weeks: (siteId: string, userId: number): Promise<WeekSummary[]> => {
    if (desktop) return invoke("timetable_weeks", { siteId, userId });
    return Promise.resolve(
      demo && demoSaved
        ? [
            {
              weekKey: demoSaved.weekKey,
              weekStart: demoSaved.weekStart,
              weekEnd: demoSaved.weekEnd,
              promotion: demoSaved.promotion,
              cohort: demoSaved.options.cohort,
              filename: demoSaved.filename,
              revision: demoSaved.savedRevision ?? 1,
              updatedAt: Math.floor(Date.now() / 1000),
              sessions: demoSaved.sessions.length,
              tentative: demoSaved.pending,
            },
          ]
        : [],
    );
  },
  week: (
    siteId: string,
    userId: number,
    weekKey: string,
  ): Promise<WeekView> => {
    if (desktop) return invoke("timetable_week", { siteId, userId, weekKey });
    return demoSaved && demoSaved.weekKey === weekKey
      ? Promise.resolve(demoSaved)
      : Promise.reject(new Error("WEEK_MISSING"));
  },
  edit: (
    siteId: string,
    userId: number,
    weekKey: string,
    expectedWeekRevision: number,
  ): Promise<WeekView> => {
    if (desktop)
      return invoke("timetable_edit", {
        siteId,
        userId,
        weekKey,
        expectedWeekRevision,
      });
    if (!demoSaved) return Promise.reject(new Error("WEEK_MISSING"));
    demoDraft = demoView("edit", demoSaved.choices, demoSaved.options);
    return delay(demoDraft, 200);
  },
  remove: (
    siteId: string,
    userId: number,
    weekKey: string,
    expectedWeekRevision: number,
  ): Promise<void> => {
    if (desktop)
      return invoke("timetable_remove", {
        siteId,
        userId,
        weekKey,
        expectedWeekRevision,
      });
    demoSaved = null;
    return Promise.resolve();
  },
  preferences: (siteId: string, userId: number): Promise<TimetableOptions> =>
    desktop
      ? invoke("timetable_preferences", { siteId, userId })
      : Promise.resolve({ ...demoPreferences }),
  setPreferences: (
    siteId: string,
    userId: number,
    options: TimetableOptions,
  ): Promise<void> => {
    if (desktop)
      return invoke("timetable_preferences_set", { siteId, userId, options });
    demoPreferences = { ...options };
    return Promise.resolve();
  },
  /** Saves the week as .ics in Downloads; `calendar` then opens it in Calendar. */
  export: (
    siteId: string,
    userId: number,
    weekKey: string,
    expectedWeekRevision: number,
    calendar: boolean,
  ): Promise<ExportedWeek> => {
    if (desktop)
      return invoke("timetable_export", {
        siteId,
        userId,
        weekKey,
        expectedWeekRevision,
        calendar,
      });
    if (!demo || !demoSaved) return unavailable();
    return delay(
      {
        filename: `Emploi du temps ${demoSaved.weekStart}.ics`,
        sessions: demoSaved.sessions.length,
        cancelled: 0,
        revealed: !calendar,
        opened: calendar,
        historySaved: true,
      },
      400,
    );
  },
  calendar: (siteId: string, userId: number): Promise<CalendarSync> =>
    desktop
      ? invoke("timetable_calendar", { siteId, userId })
      : Promise.resolve({
          enabled: demoCalendar,
          access: demoCalendar ? "granted" : "notDetermined",
        }),
  /** Asks for Calendar access once, then keeps « Emploi du temps » up to date. */
  calendarEnable: (
    siteId: string,
    userId: number,
  ): Promise<{ saved: number; removed: number }> => {
    if (desktop) return invoke("timetable_calendar_enable", { siteId, userId });
    if (!demo) return unavailable();
    demoCalendar = true;
    return delay({ saved: demoSaved?.sessions.length ?? 0, removed: 0 }, 500);
  },
  calendarDisable: (siteId: string, userId: number): Promise<void> => {
    if (desktop)
      return invoke("timetable_calendar_disable", { siteId, userId });
    demoCalendar = false;
    return Promise.resolve();
  },
  /** Opens a session link: in MoodBuro's Moodle view for the school's own pages. */
  openLink: (
    siteId: string,
    userId: number,
    weekKey: string,
    draftId: string | null,
    uid: string,
    index: number,
  ): Promise<"moodle" | "browser"> =>
    desktop
      ? invoke("timetable_open_link", {
          siteId,
          userId,
          weekKey,
          draftId,
          uid,
          index,
        })
      : unavailable(),
};
