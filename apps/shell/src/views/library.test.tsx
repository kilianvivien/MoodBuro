import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import type { ReactNode } from "react";
import type {
  AgendaEvent,
  ReminderPick,
  Course,
  InboxItem,
  Site,
  UnreadStatus,
} from "../ipc";
import { I18nProvider } from "../i18n/context";
import type { Locale } from "../i18n/locale";
import { catalogues } from "../i18n/messages";
import { Sidebar } from "../components/Sidebar";
import { Toolbar } from "../components/Toolbar";
import { QuickOpenPanel, quickOpenResults } from "../components/QuickOpen";
import { Segmented } from "../components/ui";
import { Agenda, AgendaList, agendaRange } from "./Agenda";
import { CourseList } from "./Courses";
import { InboxList } from "./Inbox";

const inLocale = (locale: Locale, node: ReactNode) =>
  renderToStaticMarkup(<I18nProvider locale={locale}>{node}</I18nProvider>);

const sites: Site[] = [
  {
    id: "a",
    wwwroot: "https://moodle.example.edu/",
    name: "Example University",
    userId: 2,
    fullName: "Ada Learner",
  },
  {
    id: "b",
    wwwroot: "https://moodle.exemple.fr/campus/",
    name: "Campus Exemple",
    userId: 3,
    fullName: "Ada Learner",
  },
];
// Local noon, so day grouping does not depend on the test machine's time zone.
const now = new Date(2026, 10, 9, 12, 0, 0);
const at = (hours: number) => Math.floor(now.getTime() / 1000) + hours * 3600;

describe("Inbox", () => {
  const items: InboxItem[] = [
    {
      siteId: "a",
      id: 2,
      subject: "Essay graded",
      timeCreated: at(-1),
      read: false,
    },
    {
      siteId: "b",
      id: 9,
      subject: "Salle modifiée",
      timeCreated: at(-26),
      read: true,
    },
  ];
  const statuses: UnreadStatus[] = [
    { siteId: "a", unread: 3, messages: 2, connection: "online" },
    { siteId: "b", unread: 0, messages: 0, connection: "online" },
  ];
  const list = (
    props: Partial<Parameters<typeof InboxList>[0]> = {},
    locale: Locale = "en",
  ) =>
    inLocale(
      locale,
      <InboxList
        items={items}
        sites={sites}
        statuses={statuses}
        filter="all"
        busy={[]}
        now={now}
        onOpen={vi.fn()}
        onMarkRead={vi.fn()}
        {...props}
      />,
    );

  it("groups notifications by day and offers Mark as Read only when unread", () => {
    const html = list();
    expect(html).toContain("Today");
    expect(html).toContain("Yesterday");
    expect(html).toContain('aria-label="Mark as read: Essay graded"');
    expect(html).not.toContain('aria-label="Mark as read: Salle modifiée"');
    expect(html).toContain('aria-label="Open in Moodle: Salle modifiée"');
    expect(html.match(/data-unread/g)).toHaveLength(1);
  });

  it("explains unread conversations that the list does not contain", () => {
    const html = list();
    expect(html).toContain("2 unread conversations in Example University");
    expect(html).toContain("Open Messages");
    expect(html).not.toContain("Campus Exemple</span><span");
    expect(list({ statuses: [] })).not.toContain("Open Messages");
  });

  it("disables a notification while it is being marked read", () => {
    expect(list({ busy: ["a:2"] })).toContain('disabled=""');
    expect(list()).not.toContain('disabled=""');
  });

  it("distinguishes an empty inbox from being caught up, in French too", () => {
    expect(list({ items: [] })).toContain("No notifications yet");
    const caughtUp = list(
      { items: [items[1]], filter: "unread", statuses: [] },
      "fr",
    );
    expect(caughtUp).toContain("Vous êtes à jour");
    expect(caughtUp).not.toContain("Salle modifiée");
    expect(list({}, "fr")).toContain("Aujourd’hui");
  });
});

describe("Courses", () => {
  const courses: Course[] = [
    {
      siteId: "a",
      id: 1,
      name: "Public Economics",
      shortName: "ECO",
      pinned: true,
    },
    { siteId: "a", id: 2, name: "Statistics", shortName: "STA", pinned: false },
    {
      siteId: "b",
      id: 3,
      name: "Économie locale",
      shortName: "",
      pinned: false,
    },
  ];
  const list = (query = "", locale: Locale = "en", items = courses) =>
    inLocale(
      locale,
      <CourseList
        courses={items}
        sites={sites}
        query={query}
        onOpen={vi.fn()}
        onPin={vi.fn()}
      />,
    );

  it("shows pinned courses first, then each organisation", () => {
    const html = list();
    expect(html.indexOf("Pinned")).toBeLessThan(
      html.indexOf("Example University"),
    );
    expect(html).toContain("2 courses");
    expect(html).toContain("1 course<");
    expect(html).toContain('aria-label="Unpin Public Economics"');
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain('aria-label="Pin Statistics"');
  });

  it("searches names without accents, across organisations", () => {
    const html = list("econ");
    expect(html).toContain("Public Economics");
    expect(html).toContain("Économie locale");
    expect(html).not.toContain("Statistics");
    // One ranked list while searching: no Pinned section.
    expect(html).not.toContain("Pinned");
  });

  it("has distinct empty and no-match states", () => {
    expect(list("", "en", [])).toContain("No courses yet");
    expect(list("zzz")).toContain("No course matches “zzz”");
    expect(list("zzz", "fr")).toContain(
      "Aucun cours ne correspond à «\u00a0zzz\u00a0»",
    );
  });
});

describe("Agenda", () => {
  const events: AgendaEvent[] = [
    {
      siteId: "a",
      id: 1,
      name: "Policy brief due",
      timeStart: at(2),
      duration: 0,
      kind: "due",
      courseId: 1,
      courseName: "Public Economics",
    },
    {
      siteId: "b",
      id: 2,
      name: "Séminaire",
      timeStart: at(26),
      duration: 3600,
      kind: "course",
      courseId: null,
      courseName: null,
    },
  ];
  const list = (locale: Locale, items = events) =>
    inLocale(
      locale,
      <AgendaList events={items} sites={sites} now={now} onOpen={vi.fn()} />,
    );

  it("groups events by day with their course, organisation and kind", () => {
    const html = list("en");
    expect(html).toContain("Today");
    expect(html).toContain("Tomorrow");
    expect(html).toContain("Public Economics · Example University");
    expect(html).toContain('data-kind="due"');
    expect(html).toContain("until 15:00");
    expect(html).toContain(
      'aria-label="Open Policy brief due in the Moodle calendar"',
    );
  });

  it("offers a one-off reminder only for a deadline with a reminder time ahead", () => {
    const remind = (picks: ReminderPick[] = []) =>
      inLocale(
        "en",
        <AgendaList
          events={events}
          sites={sites}
          now={now}
          picks={picks}
          onRemind={vi.fn()}
          onOpen={vi.fn()}
        />,
      );
    // Due in 2 hours: 24 hours before has passed, 1 hour before has not.
    const html = remind();
    expect(html).toContain(
      'aria-label="Remind me 1 hour before: Policy brief due"',
    );
    expect(html).toContain('aria-pressed="false"');
    // The seminar is not a deadline: one control only.
    expect(html.match(/remind-toggle/g)).toHaveLength(1);
    const set = remind([
      {
        siteId: "a",
        eventId: 1,
        offset: 3600,
        due: events[0].timeStart,
        name: "Policy brief due",
      },
    ]);
    expect(set).toContain('aria-pressed="true"');
    expect(set).toContain("Reminder set 1 hour before: Policy brief due");
    // Without a handler (read-only uses) the control is absent.
    expect(list("en")).not.toContain("remind-toggle");
  });

  it("is translated and has an empty state", () => {
    const html = list("fr");
    expect(html).toContain("Demain");
    expect(html).toContain("À rendre");
    expect(html).toContain("jusqu’à 15:00");
    expect(list("fr", [])).toContain("Rien de prévu");
  });
});

describe("Quick Open", () => {
  const courses: Course[] = [
    {
      siteId: "a",
      id: 1,
      name: "Public Economics",
      shortName: "ECO",
      pinned: false,
    },
    {
      siteId: "b",
      id: 3,
      name: "Finances publiques",
      shortName: "FIP",
      pinned: true,
    },
    { siteId: "gone", id: 4, name: "Orphan", shortName: "", pinned: true },
  ];
  const results = (query: string) =>
    quickOpenResults(query, sites, courses, catalogues.en);

  it("offers pinned courses, organisations and views before typing", () => {
    const ids = results("").map((result) => result.id);
    expect(ids[0]).toBe("course:b:3");
    expect(ids).toContain("site:a");
    expect(ids).toContain("view:inbox");
    expect(ids).not.toContain("course:a:1");
    // A course whose organisation was removed is never offered.
    expect(ids).not.toContain("course:gone:4");
  });

  it("ranks courses, organisations and views by what was typed", () => {
    expect(results("pub eco")[0].id).toBe("course:a:1");
    expect(results("campus")[0].target).toEqual({ kind: "site", siteId: "b" });
    expect(results("FIP")[0].id).toBe("course:b:3");
    expect(results("sett")[0].target).toEqual({
      kind: "view",
      view: "settings",
    });
    expect(results("no such thing")).toEqual([]);
  });

  it("searches cached activity names across sites and drops orphaned metadata", () => {
    const activities = [
      {
        siteId: "a",
        courseId: 1,
        id: 42,
        name: "Économie essay",
        module: "assign",
        refreshedAt: 1,
      },
      {
        siteId: "gone",
        courseId: 4,
        id: 42,
        name: "Économie orphan",
        module: "quiz",
        refreshedAt: 1,
      },
      {
        siteId: "a",
        courseId: 99,
        id: 43,
        name: "Économie removed course",
        module: "quiz",
        refreshedAt: 1,
      },
    ];
    const found = quickOpenResults(
      "economie",
      sites,
      courses,
      catalogues.en,
      activities,
    );
    expect(found).toHaveLength(1);
    expect(found[0].target).toEqual({
      kind: "activity",
      siteId: "a",
      activityId: 42,
    });
    expect(
      quickOpenResults("", sites, courses, catalogues.en, activities).some(
        (r) => r.target.kind === "activity",
      ),
    ).toBe(false);
  });

  it("renders an accessible combobox with one selected option", () => {
    const html = inLocale(
      "fr",
      <QuickOpenPanel
        query=""
        results={quickOpenResults("", sites, courses, catalogues.fr)}
        active={1}
        onQuery={vi.fn()}
        onActive={vi.fn()}
        onChoose={vi.fn()}
      />,
    );
    expect(html).toContain('role="combobox"');
    expect(html).toContain('aria-label="Ouverture rapide"');
    expect(html).toContain('aria-activedescendant="quick-site:a"');
    expect(html.match(/aria-selected="true"/g)).toHaveLength(1);
    const none = inLocale(
      "en",
      <QuickOpenPanel
        query="zzz"
        results={[]}
        active={0}
        onQuery={vi.fn()}
        onActive={vi.fn()}
        onChoose={vi.fn()}
      />,
    );
    expect(none).toContain("No results");
    expect(none).toContain('aria-expanded="false"');
  });
});

describe("navigation chrome", () => {
  const sidebar = (
    props: Partial<Parameters<typeof Sidebar>[0]> = {},
    locale: Locale = "en",
  ) =>
    inLocale(
      locale,
      <Sidebar
        sites={sites}
        unread={[
          { siteId: "a", unread: 3, connection: "online" },
          { siteId: "b", unread: 2, connection: "online" },
        ]}
        active="a"
        view="inbox"
        onSite={vi.fn()}
        onView={vi.fn()}
        onAdd={vi.fn()}
        onSettings={vi.fn()}
        {...props}
      />,
    );

  it("adds Inbox, Courses and Agenda with the total unread count", () => {
    const html = sidebar();
    expect(html).toContain('aria-label="Notifications — 5 unread"');
    expect(html).toContain('aria-label="Courses"');
    expect(html).toContain('aria-label="Agenda"');
    // Inbox is current; the active organisation is not while a local view shows.
    expect(html.match(/aria-current="page"/g)).toHaveLength(1);
    expect(sidebar({}, "fr")).toContain(
      'aria-label="Notifications — 5 non lus"',
    );
  });

  it("hides app-wide views until an organisation is connected", () => {
    const html = sidebar({ sites: [], unread: [], view: "onboarding" });
    expect(html).not.toContain('aria-label="Notifications');
    expect(html).toContain('aria-label="Add organisation"');
  });

  it("titles each local view and offers Quick Open", () => {
    for (const [view, title] of [
      ["inbox", "Notifications"],
      ["courses", "Cours"],
      ["agenda", "Agenda"],
      ["downloads", "Téléchargements"],
    ] as const) {
      const html = inLocale(
        "fr",
        <Toolbar
          view={view}
          site={undefined}
          error={null}
          canLeave
          onNav={vi.fn()}
          onDismissError={vi.fn()}
          onLeave={vi.fn()}
          onQuickOpen={vi.fn()}
        />,
      );
      expect(html).toContain(`<h1>${title}</h1>`);
      expect(html).toContain('aria-label="Ouverture rapide (⌘K)"');
      expect(html).toContain("Terminé");
    }
  });

  it("keeps the segmented control a native radio group", () => {
    const html = inLocale(
      "en",
      <Segmented
        name="filter"
        legend="Show"
        value="unread"
        options={[
          { value: "all", label: "All" },
          { value: "unread", label: "Unread" },
        ]}
        onChange={vi.fn()}
      />,
    );
    expect(html.match(/type="radio"/g)).toHaveLength(2);
    expect(html).toContain('checked="" value="unread"');
  });
});

describe("calendar export date selection", () => {
  it("uses local midnight and an exclusive next-day end", () => {
    const range = agendaRange("2026-10-25", "2026-10-25");
    expect(range.valid).toBe(true);
    expect(range.start).toBe(new Date(2026, 9, 25).getTime() / 1000);
    expect(range.end).toBe(new Date(2026, 9, 26).getTime() / 1000);
  });
  it("supports open bounds and rejects reversed or nonexistent dates", () => {
    expect(agendaRange("", "")).toEqual({
      start: null,
      end: null,
      valid: true,
    });
    expect(agendaRange("2026-11-10", "2026-11-09").valid).toBe(false);
    expect(agendaRange("2026-02-30", "").valid).toBe(false);
    expect(agendaRange("", "not-a-date").valid).toBe(false);
  });
  it("renders accessible export controls and snapshot limitations in both languages", () => {
    for (const locale of ["en", "fr"] as const) {
      const html = inLocale(
        locale,
        <Agenda sites={sites} onOpenDay={vi.fn()} onSettings={vi.fn()} />,
      );
      expect(html).not.toContain('type="date"');
      expect(html).toContain('aria-expanded="false"');
      expect(html).toContain(catalogues[locale].agenda.dates);
      expect(html).toContain('<details class="help-disclosure">');
      expect(html).not.toContain('id="reminders-title"');
      expect(html).toContain(catalogues[locale].agenda.export);
      expect(html).toContain(catalogues[locale].agenda.exportHint);
      expect(html).toContain('disabled=""');
    }
  });
});

describe("Moodle session recovery notice", () => {
  it("explains the wait in both languages and shows retry and failure states", () => {
    for (const locale of ["en", "fr"] as const) {
      for (const phase of ["waiting", "retrying", "failed"] as const) {
        const html = inLocale(
          locale,
          <Toolbar
            view="site"
            site={sites[0]}
            error={null}
            canLeave
            recovery={{ siteId: "a", phase, retryAt: at(1) }}
            onNav={vi.fn()}
            onDismissError={vi.fn()}
            onLeave={vi.fn()}
          />,
        );
        expect(html).toContain('role="status"');
        expect(html).toContain(locale === "en" ? "six minutes" : "six minutes");
        expect(html).toContain(
          phase === "waiting"
            ? locale === "en"
              ? "Retrying at"
              : "Nouvel essai à"
            : phase === "retrying"
              ? locale === "en"
                ? "Restoring"
                : "Restauration"
              : locale === "en"
                ? "failed"
                : "Échec",
        );
      }
    }
  });
});

describe("Bookmarks sidebar", () => {
  it("shows five custom icons and keeps remaining bookmarks in the overflow command", () => {
    const bookmarks = Array.from({ length: 8 }, (_, i) => ({
      id: String(i),
      siteId: "a",
      userId: 2,
      name: `Saved page ${i}`,
      path: `course/view.php?id=${i + 1}`,
      icon: i === 0 ? "star" : "book",
    }));
    const html = inLocale(
      "en",
      <Sidebar
        sites={sites}
        active="a"
        view="site"
        bookmarks={bookmarks}
        onBookmark={vi.fn()}
        onBookmarkMenu={vi.fn()}
        onSite={vi.fn()}
        onAdd={vi.fn()}
        onSettings={vi.fn()}
      />,
    );
    expect(html).toContain('aria-label="Saved page 4"');
    expect(html).not.toContain('aria-label="Saved page 5"');
    expect(html).toContain('aria-haspopup="menu"');
    expect(html).toContain("lucide-star");
  });
});
