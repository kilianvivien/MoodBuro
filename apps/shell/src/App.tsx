import {
  useEffect,
  useEffectEvent,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { listen } from "@tauri-apps/api/event";
import {
  desktop,
  ipc,
  message,
  type DownloadFeedback,
  type MoodlePage,
  type NavAction,
  type Bookmark,
  type SessionRecovery,
  type UnreadStatus,
} from "./ipc";
import { useSites } from "./stores/sites";
import { useSettings } from "./stores/settings";
import { useAssistant } from "./stores/assistant";
import { AssistantPanel } from "./components/AssistantPanel";
import { Reminders } from "./views/Reminders";
import { AssistantSettings } from "./views/AssistantSettings";
import { connection as reduce, idle } from "./lib/connection";
import {
  isLocal,
  Sidebar,
  type Destination,
  type View,
} from "./components/Sidebar";
import { BookmarksView } from "./components/Bookmarks";
import { QuickOpen, type QuickTarget } from "./components/QuickOpen";
import { Inbox } from "./views/Inbox";
import { Courses } from "./views/Courses";
import { Agenda } from "./views/Agenda";
import type { TimetableRequest } from "./views/Timetable";
import { timetable, type TimetableStatus } from "./lib/timetable";
import { TimetableSettings } from "./views/TimetableSettings";
import { Toolbar } from "./components/Toolbar";
import { Uploads } from "./views/Uploads";
import { Downloads } from "./views/Downloads";
import { AddSite } from "./views/AddSite";
import {
  Settings,
  settingsSections,
  type Notice,
  type SettingsSection,
} from "./views/Settings";
import { I18nProvider } from "./i18n/context";
import { resolveLocale, systemLanguages } from "./i18n/locale";
import { catalogues } from "./i18n/messages";

/** Last removal: the site's name on success, else the raw backend error. */
type Removal = { name: string | undefined } | { error: string };

export default function App() {
  const { sites, active, refresh, select } = useSites();
  const settings = useSettings();
  const assistant = useAssistant();
  // Bumped whenever the panel should take keyboard focus.
  const [assistantFocus, setAssistantFocus] = useState(0);
  // The panel is playing its exit; Rust widens the Moodle view once it is done.
  const [assistantClosing, setAssistantClosing] = useState(false);
  const assistantLeaving = useRef(false);
  const [linkRequest, setLinkRequest] = useState(0);
  const [prefill, setPrefill] = useState({ url: "", revision: 0 });
  const consumingLink = useRef(false);
  const linkAgain = useRef(false);
  const [uploadRevision, setUploadRevision] = useState(0);
  const [calendarRequest, setCalendarRequest] = useState(0);
  // Organisations with timetables (the INSP Moodle), as Rust decides.
  const [timetableStatus, setTimetableStatus] =
    useState<TimetableStatus | null>(null);
  const timetableIds = timetableStatus?.sites ?? [];
  const [timetableRequest, setTimetableRequest] =
    useState<TimetableRequest | null>(null);
  const [settingsSection, setSettingsSection] =
    useState<SettingsSection>("general");
  const [view, setView] = useState<View>("onboarding");
  const [downloadFeedback, setDownloadFeedback] =
    useState<DownloadFeedback | null>(null);
  const [unread, setUnread] = useState<UnreadStatus[]>([]);
  const [recoveries, setRecoveries] = useState<SessionRecovery[]>([]);
  const [ready, setReady] = useState(false);
  // ⌘K palette, shown over whatever view is current.
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);
  const [bookmarkDraft, setBookmarkDraft] = useState<Bookmark | null>(null);
  const [bookmarkRevision, setBookmarkRevision] = useState(0);
  const [palette, setPalette] = useState(false);
  // Bumped after a refresh asked from a context menu, to remount the open list.
  const [listRevision, setListRevision] = useState(0);
  const [connection, dispatch] = useReducer(reduce, idle);
  // Raw backend messages: translated when rendered, so a language switch
  // also translates what is already on screen.
  const [error, setError] = useState<string | null>(null);
  // Outcome of the last removal, shown in Settings.
  const [removed, setRemoved] = useState<Removal | null>(null);
  const [preferred, setPreferred] = useState(systemLanguages);
  const locale = resolveLocale(settings.language, preferred);
  const t = catalogues[locale];
  const site = sites.find((s) => s.id === active);
  const timetableSites = sites.filter((s) => timetableIds.includes(s.id));
  useEffect(() => {
    void timetable
      .status()
      .then(setTimetableStatus)
      .catch(() => setTimetableStatus(null));
  }, [sites]);
  /** Read a timetable in Agenda › Week, from any entry point. */
  const openTimetable = useEffectEvent(
    (siteId: string, source: TimetableRequest["source"]) => {
      setTimetableRequest((previous) => ({
        revision: (previous?.revision ?? 0) + 1,
        siteId,
        source,
      }));
      void showLocal("agenda");
    },
  );

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  function failed(cause: unknown) {
    setError(message(cause));
  }

  async function openSite(id: string) {
    setError(null);
    setPalette(false);
    try {
      await ipc.switchSite(id);
      select(id);
      setView("site");
    } catch (cause) {
      failed(cause);
    }
  }

  async function showOnboarding() {
    setError(null);
    setPalette(false);
    dispatch({ type: "dismiss" });
    try {
      await ipc.onboarding();
      setView("onboarding");
    } catch (cause) {
      failed(cause);
    }
  }

  async function showSettings(section: SettingsSection = "general") {
    setSettingsSection(section);
    setError(null);
    setPalette(false);
    setRemoved(null);
    dispatch({ type: "dismiss" });
    try {
      // The remote view sits above the shell; hide it before showing Settings.
      await ipc.shellView("settings");
      setView("settings");
    } catch (cause) {
      failed(cause);
    }
  }

  /** Inbox, Courses, Agenda or Downloads: hide the Moodle view, then render. */
  async function showLocal(
    target: Destination | "downloads" | "bookmarks" | "uploads",
  ) {
    setError(null);
    setPalette(false);
    try {
      await ipc.shellView(target);
      setView(target);
    } catch (cause) {
      failed(cause);
    }
  }
  const showDownloads = () => {
    setDownloadFeedback(null);
    return showLocal("downloads");
  };

  /** Opens one of the fixed Moodle pages in the organisation's own view. */
  async function openPage(id: string, page: MoodlePage, value?: number) {
    setError(null);
    setPalette(false);
    try {
      await ipc.openPage(id, page, value ?? null);
      select(id);
      setView("site");
    } catch (cause) {
      failed(cause);
    }
  }

  async function refreshBookmarks() {
    setBookmarks(await ipc.bookmarks());
  }
  async function openBookmark(bookmark: Bookmark) {
    try {
      await ipc.bookmarkOpen(bookmark.id);
      select(bookmark.siteId);
      setView("site");
      setPalette(false);
    } catch (cause) {
      failed(cause);
    }
  }
  async function addBookmark() {
    try {
      const draft = await ipc.bookmarkCapture();
      const existing = bookmarks.find(
        (b) =>
          b.siteId === draft.siteId &&
          b.userId === draft.userId &&
          b.path === draft.path,
      );
      setBookmarkDraft(existing ?? draft);
      setBookmarkRevision((r) => r + 1);
      await showLocal("bookmarks");
    } catch (cause) {
      failed(cause);
    }
  }
  /** The full bookmark list, not the editor of a captured page. */
  function showBookmarks() {
    setBookmarkDraft(null);
    setBookmarkRevision((r) => r + 1);
    void showLocal("bookmarks");
  }
  async function openPalette() {
    if (sites.length > 0) setPalette(true);
  }
  function closePalette() {
    setPalette(false);
  }
  useEffect(() => {
    document.documentElement.toggleAttribute(
      "data-palette-site",
      palette && view === "site",
    );
    void ipc
      .shellView(palette ? "quick-open" : "quick-open-close")
      .catch(failed);
    return () => document.documentElement.removeAttribute("data-palette-site");
  }, [palette, view]);
  function choose(target: QuickTarget) {
    setPalette(false);
    if (target.kind === "site") void openSite(target.siteId);
    else if (target.kind === "course")
      void openPage(target.siteId, "course", target.courseId);
    else if (target.kind === "activity")
      void openPage(target.siteId, "activity", target.activityId);
    else if (target.view === "settings") void showSettings();
    else if (target.view === "bookmarks") showBookmarks();
    else void showLocal(target.view);
  }

  /**
   * Opening: Rust narrows the Moodle view first, so the panel never sits under
   * it, then the panel enters. Closing is the reverse: the panel leaves, then
   * `closeAssistant` gives the room back. Toggling mid-exit keeps it open.
   */
  async function toggleAssistant() {
    if (assistantLeaving.current && useAssistant.getState().panel.open) {
      assistantLeaving.current = false;
      setAssistantClosing(false);
      setAssistantFocus((revision) => revision + 1);
      return;
    }
    // Something else already closed it mid-exit: start from a clean state.
    assistantLeaving.current = false;
    setAssistantClosing(false);
    if (useAssistant.getState().panel.open) {
      assistantLeaving.current = true;
      setAssistantClosing(true);
      return;
    }
    await useAssistant.getState().setOpen(true);
    setAssistantFocus((revision) => revision + 1);
  }
  async function closeAssistant() {
    if (!assistantLeaving.current) return;
    assistantLeaving.current = false;
    await useAssistant.getState().setOpen(false);
    setAssistantClosing(false);
  }

  /** Done/Cancel/Escape: back to the last site, else the onboarding view. */
  function leave() {
    const target = site?.id ?? sites[0]?.id;
    if (target) void openSite(target);
    else void showOnboarding();
  }

  async function addSite(url: string) {
    dispatch({ type: "add" });
    try {
      await ipc.addSite(url);
      dispatch({ type: "window-opened" });
    } catch (cause) {
      dispatch({ type: "failed", error: message(cause) });
    }
  }

  async function reauthenticate(id: string) {
    setRemoved(null);
    dispatch({ type: "reauthenticate", siteId: id });
    try {
      await ipc.reauthenticate(id);
      dispatch({ type: "window-opened" });
    } catch (cause) {
      dispatch({ type: "failed", error: message(cause) });
    }
  }

  async function removeSite(id: string) {
    const name = sites.find((s) => s.id === id)?.name;
    setRemoved(null);
    dispatch({ type: "dismiss" });
    try {
      await ipc.removeSite(id);
    } catch (cause) {
      setRemoved({ error: message(cause) });
      return false;
    }
    setRemoved({ name });
    await refresh().catch(failed);
    return true;
  }

  async function consumeLink() {
    if (consumingLink.current) {
      linkAgain.current = true;
      return true;
    }
    consumingLink.current = true;
    try {
      const result = await ipc.deepLink();
      if (!result) return false;
      setError(null);
      dispatch({ type: "dismiss" });
      setPalette(false);
      if (result.kind === "site") {
        select(result.siteId);
        setView("site");
      } else {
        setPrefill(({ revision }) => ({
          url: result.url,
          revision: revision + 1,
        }));
        setView("onboarding");
      }
      return true;
    } catch (cause) {
      await ipc.shellView("settings").catch(() => {});
      setView("settings");
      failed(cause);
      return true;
    } finally {
      consumingLink.current = false;
      if (linkAgain.current) {
        linkAgain.current = false;
        setLinkRequest((revision) => revision + 1);
      }
    }
  }
  const onLinkPending = useEffectEvent(() => {
    void consumeLink();
  });
  useEffect(() => {
    if (ready) onLinkPending();
  }, [ready, linkRequest]);

  const onSitesChanged = useEffectEvent((id: string) => {
    void refreshBookmarks().catch(failed);
    void refresh().catch(failed);
    setLinkRequest((revision) => revision + 1);
    if (isLocal(view) && view !== "onboarding") {
      // Rust may have revealed the site; keep the local view on top.
      void ipc.shellView(view).catch(failed);
      return;
    }
    setPalette(false);
    select(id);
    setView("site");
  });
  const onSiteRemoved = useEffectEvent(() => {
    void refreshBookmarks().catch(failed);
    void refresh().catch(failed);
    // Rust hid every site view; Settings stays, anything else falls back.
    setPalette(false);
    if (!isLocal(view)) setView("onboarding");
  });
  /** App-menu commands (Settings…, Add Organisation…); Rust already hid sites. */
  const onNavigate = useEffectEvent((target: string) => {
    if (target === "calendar-subscribe") {
      setCalendarRequest((request) => request + 1);
      void showLocal("agenda");
    } else if (target === "settings") void showSettings();
    else if (target === "onboarding") void showOnboarding();
    else if (target === "bookmarks") showBookmarks();
    else if (
      target === "uploads" ||
      target === "downloads" ||
      target === "inbox" ||
      target === "courses" ||
      target === "agenda"
    )
      void showLocal(target);
    else if (target === "quick-open") {
      if (sites.length > 0) setPalette((open) => !open);
      else leave();
    }
  });
  /** A choice in a sidebar context menu: `action` or `action:id`, named by Rust. */
  const onSidebarAction = useEffectEvent((payload: string) => {
    const split = payload.indexOf(":");
    const action = split < 0 ? payload : payload.slice(0, split);
    const id = split < 0 ? "" : payload.slice(split + 1);
    const run = (work: Promise<unknown>) => void work.catch(failed);
    if (action === "view") {
      if (id === "onboarding") void showOnboarding();
      else if (id === "inbox" || id === "courses" || id === "agenda")
        void showLocal(id);
    } else if (action === "settings") {
      const section = settingsSections.find((name) => name === id);
      void showSettings(section ?? "general");
    } else if (action === "bookmarks") showBookmarks();
    else if (action === "calendar") onNavigate("calendar-subscribe");
    else if (action === "mark-read") {
      // Every organisation, or the one whose menu it was.
      const targets = sites.filter((s) => !id || s.id === id);
      run(
        Promise.all(targets.map((s) => ipc.markRead(s.id, null)))
          .then(() => ipc.unread())
          .then(setUnread),
      );
    } else if (action === "refresh" && (id === "courses" || id === "agenda")) {
      const refreshOne =
        id === "courses" ? ipc.refreshCourses : ipc.refreshAgenda;
      run(
        Promise.allSettled(sites.map((s) => refreshOne(s.id))).then(() => {
          // Remount the list so it reads what was just saved.
          setListRevision((revision) => revision + 1);
          return showLocal(id);
        }),
      );
    } else if (action === "site-open") void openSite(id);
    else if (action === "site-home")
      run(openSite(id).then(() => ipc.nav("home")));
    else if (action === "site-notifications")
      void openPage(id, "notifications");
    else if (action === "site-messages") void openPage(id, "messages");
    else if (action === "site-up" || action === "site-down")
      run(
        ipc.moveSite(id, action === "site-up" ? -1 : 1).then(() => refresh()),
      );
    else if (action === "site-reauth")
      run(showSettings("organisations").then(() => reauthenticate(id)));
    else if (action === "bookmark-open" || action === "bookmark-edit") {
      const bookmark = bookmarks.find((b) => b.id === id);
      if (!bookmark) return;
      if (action === "bookmark-open") void openBookmark(bookmark);
      else {
        setBookmarkDraft(bookmark);
        setBookmarkRevision((r) => r + 1);
        void showLocal("bookmarks");
      }
    }
  });
  const onBookmarkAdd = useEffectEvent(() => {
    void addBookmark();
  });
  const onAuthFinished = useEffectEvent((result: string | null) =>
    dispatch({ type: "finished", error: result }),
  );
  const onLaunch = useEffectEvent(async (cancelled: () => boolean) => {
    // Load the language before the first view renders, without delaying sites.
    const settingsLoaded = settings.load();
    void assistant.load();
    try {
      await refresh();
    } catch (cause) {
      failed(cause);
    }
    await settingsLoaded;
    if (cancelled()) return;
    // Reopen the first organisation instead of the empty onboarding view.
    const first = useSites.getState().sites[0];
    const linked = await consumeLink();
    if (!linked && first) await openSite(first.id);
    if (!cancelled()) setReady(true);
  });
  const onKey = useEffectEvent((event: KeyboardEvent) => {
    // The native menu owns ⌘K in the app; the browser preview needs its own.
    if (
      !desktop &&
      (event.metaKey || event.ctrlKey) &&
      event.key.toLowerCase() === "k"
    ) {
      event.preventDefault();
      if (palette) closePalette();
      else void openPalette();
      return;
    }
    if (event.key !== "Escape" || event.defaultPrevented) return;
    if (palette) closePalette();
    else if (isLocal(view) && (view !== "onboarding" || sites.length > 0))
      leave();
  });

  useEffect(() => {
    let cancelled = false;
    void onLaunch(() => cancelled);
    const keydown = (event: KeyboardEvent) => onKey(event);
    // Automatic language follows changes to the system's preferred languages.
    const languagechange = () => setPreferred(systemLanguages());
    window.addEventListener("keydown", keydown);
    window.addEventListener("languagechange", languagechange);
    const stop = () => {
      cancelled = true;
      window.removeEventListener("keydown", keydown);
      window.removeEventListener("languagechange", languagechange);
    };
    void ipc.unread().then(setUnread).catch(failed);
    void refreshBookmarks().catch(failed);
    if (!desktop) return stop;
    const listeners = [
      listen("deep-link://pending", () =>
        setLinkRequest((revision) => revision + 1),
      ),
      listen<SessionRecovery[]>("session://changed", (e) =>
        setRecoveries(e.payload),
      ),
      listen<UnreadStatus[]>("unread://changed", (e) => setUnread(e.payload)),
      listen<string>("navigation://site-opened", (e) => {
        setPalette(false);
        select(e.payload);
        setView("site");
        void refresh().catch(failed);
      }),
      listen<string>("sites://changed", (e) => onSitesChanged(e.payload)),
      listen<string>("sites://removed", () => onSiteRemoved()),
      listen<string>("shell://navigate", (e) => onNavigate(e.payload)),
      listen("uploads://selected", () =>
        setUploadRevision((revision) => revision + 1),
      ),
      listen<string>("shell://error", (e) => setError(e.payload)),
      listen<DownloadFeedback>("downloads://feedback", (e) =>
        setDownloadFeedback(e.payload),
      ),
      // View › AI Assistant (⇧⌘A).
      listen("assistant://toggle", () => void toggleAssistant()),
      // The native “Open as timetable” item in a Moodle link menu.
      listen<{ siteId: string; intentId: string }>("timetable://link", (e) =>
        openTimetable(e.payload.siteId, {
          kind: "linkIntent",
          intentId: e.payload.intentId,
        }),
      ),
      listen("bookmarks://add", () => void onBookmarkAdd()),
      listen<string>("sidebar://action", (e) => onSidebarAction(e.payload)),
      listen<string | null>("auth://finished", (e) =>
        onAuthFinished(e.payload),
      ),
    ];
    void Promise.all(listeners)
      .then(() => ipc.sessionStatus())
      .then(setRecoveries)
      .catch(failed);
    return () => {
      stop();
      for (const listener of listeners)
        void listener.then((unlisten) => unlisten());
    };
  }, []);

  const reauthOutcome =
    connection.phase === "idle" && connection.last?.siteId
      ? connection.last
      : null;
  const reauthName = sites.find((s) => s.id === reauthOutcome?.siteId)?.name;
  const accountMessage: Notice | null = removed
    ? "error" in removed
      ? { tone: "error", error: removed.error, context: "remove" }
      : {
          tone: "success",
          text: removed.name
            ? t.settings.removed(removed.name)
            : t.settings.removedUnnamed,
        }
    : reauthOutcome
      ? reauthOutcome.error
        ? { tone: "error", error: reauthOutcome.error, context: "signIn" }
        : {
            tone: "success",
            text: reauthName
              ? t.settings.reauthenticated(reauthName)
              : t.settings.reauthenticatedUnnamed,
          }
      : null;

  return (
    <I18nProvider locale={locale}>
      <div
        className="app"
        data-palette={palette || undefined}
        data-view={view}
        data-assistant={assistant.panel.open || undefined}
        style={
          assistant.panel.open
            ? ({
                "--assistant-width": `${assistant.panel.width}px`,
              } as CSSProperties)
            : undefined
        }
      >
        <Sidebar
          sites={sites}
          unread={unread}
          bookmarks={bookmarks.filter((b) =>
            sites.some((s) => s.id === b.siteId && s.userId === b.userId),
          )}
          onBookmark={(b) => void openBookmark(b)}
          onMenu={(target) => void ipc.sidebarMenu(target).catch(failed)}
          onBookmarkMenu={() => {
            if (desktop) void ipc.bookmarkMenu().catch(failed);
            else showBookmarks();
          }}
          active={active}
          view={view}
          onSite={(id) => void openSite(id)}
          onView={(target) => void showLocal(target)}
          onAdd={() => void showOnboarding()}
          onSettings={() => void showSettings()}
        />
        <Toolbar
          view={view}
          site={site}
          error={error ?? assistant.error}
          recovery={recoveries.find((r) => r.siteId === active)}
          canLeave={view !== "onboarding" || sites.length > 0}
          onNav={(action: NavAction) => void ipc.nav(action).catch(failed)}
          onDismissError={() => {
            setError(null);
            useAssistant.setState({ error: null });
          }}
          onAssistant={ready ? () => void toggleAssistant() : undefined}
          assistantOpen={assistant.panel.open && !assistantClosing}
          onLeave={leave}
          downloadFeedback={
            sites.some(
              (s) =>
                s.id === downloadFeedback?.siteId &&
                s.userId === downloadFeedback?.userId,
            )
              ? downloadFeedback
              : null
          }
          onDismissDownload={() => setDownloadFeedback(null)}
          onDownloads={() => void showDownloads()}
          paused={palette}
          onBookmark={
            view === "site" && desktop ? () => void addBookmark() : undefined
          }
          onQuickOpen={
            sites.length > 0
              ? () => (palette ? closePalette() : void openPalette())
              : undefined
          }
        />
        <main className="content" aria-busy={!ready} data-view={view}>
          {ready && palette && (
            <QuickOpen sites={sites} onChoose={choose} onClose={closePalette} />
          )}
          {ready && view === "bookmarks" && (
            <BookmarksView
              key={bookmarkRevision}
              items={bookmarks}
              sites={sites}
              initialDraft={bookmarkDraft}
              onChanged={refreshBookmarks}
              onOpen={(b) => void openBookmark(b)}
            />
          )}
          {ready && view === "inbox" && (
            <Inbox
              sites={sites}
              statuses={unread}
              onOpenPage={(id, page) => void openPage(id, page)}
              onChanged={() => void ipc.unread().then(setUnread).catch(failed)}
            />
          )}
          {ready && view === "courses" && (
            <Courses
              key={listRevision}
              sites={sites}
              onOpenCourse={(id, course) => void openPage(id, "course", course)}
            />
          )}
          {ready && view === "uploads" && (
            <Uploads
              sites={sites}
              revision={uploadRevision}
              onOpened={(id) => {
                select(id);
                setView("site");
              }}
            />
          )}
          {ready && view === "agenda" && (
            <Agenda
              key={listRevision}
              calendarRequest={calendarRequest}
              sites={sites}
              timetableSites={timetableSites}
              timetableRequest={timetableRequest}
              onTimetableRequestHandled={(request) =>
                setTimetableRequest((pending) =>
                  pending === request ? null : pending,
                )
              }
              onFindTimetable={(id) => void openSite(id)}
              onSettings={() => void showSettings("notifications")}
              onOpenPage={(id) => void openPage(id, "calendar-export")}
              onOpenDay={(id, time) => void openPage(id, "calendar-day", time)}
            />
          )}
          {ready && view === "onboarding" && (
            <AddSite
              key={prefill.revision}
              initialUrl={prefill.url}
              connection={connection}
              firstRun={sites.length === 0}
              onSubmit={(url) => void addSite(url)}
            />
          )}
          {ready && view === "downloads" && (
            <Downloads
              onUpload={() => void showLocal("uploads")}
              sites={sites}
              timetableSites={timetableIds}
              onTimetable={(item) =>
                openTimetable(item.siteId, {
                  kind: "download",
                  downloadId: item.id,
                })
              }
              onReturn={(item) => {
                void ipc
                  .courses()
                  .then(({ items }) => {
                    if (
                      items.some(
                        (c) =>
                          c.siteId === item.siteId && c.id === item.courseId,
                      )
                    )
                      void openPage(item.siteId, "course", item.courseId!);
                    else void openSite(item.siteId);
                  })
                  .catch(() => void openSite(item.siteId));
              }}
            />
          )}
          {ready && view === "settings" && (
            <Settings
              section={settingsSection}
              onSection={setSettingsSection}
              reminders={
                <Reminders sites={sites} onAdd={() => void showOnboarding()} />
              }
              assistant={<AssistantSettings />}
              timetable={
                timetableSites.length > 0 ? (
                  <TimetableSettings
                    sites={timetableSites}
                    status={timetableStatus}
                  />
                ) : undefined
              }
              desktop={desktop}
              sites={sites}
              appearance={settings.appearance}
              appearanceSaving={settings.saving.appearance}
              language={settings.language}
              languageSaving={settings.saving.language}
              globalShortcut={settings.globalShortcut}
              globalShortcutSaving={settings.saving.globalShortcut}
              shortcutUnavailable={settings.shortcutUnavailable}
              files={{ pdf: settings.pdf, images: settings.images }}
              filesSaving={{
                pdf: settings.saving.pdf,
                images: settings.saving.images,
              }}
              settingsError={settings.error}
              settingsErrorIn={
                settings.errorField === "pdf" ||
                settings.errorField === "images"
                  ? "files"
                  : "general"
              }
              connection={connection}
              accountMessage={accountMessage}
              onAppearance={(value) => void settings.setAppearance(value)}
              onLanguage={(value) => void settings.setLanguage(value)}
              onGlobalShortcut={(enabled) =>
                void settings.setGlobalShortcut(enabled)
              }
              onFileHandling={(kind, value) =>
                void (kind === "pdf"
                  ? settings.setPdf(value)
                  : settings.setImages(value))
              }
              onOpenSite={(id) => void openSite(id)}
              onReauthenticate={(id) => void reauthenticate(id)}
              onRemove={removeSite}
              onAdd={() => void showOnboarding()}
            />
          )}
        </main>
        {ready && assistant.panel.open && (
          <AssistantPanel
            site={site}
            focus={assistantFocus}
            closing={assistantClosing}
            onClosed={() => void closeAssistant()}
            onOpenPage={(source, siteId) =>
              void openPage(siteId, source.page, source.value ?? undefined)
            }
            onSettings={() => void showSettings("assistant")}
            onClose={() => void toggleAssistant()}
          />
        )}
      </div>
    </I18nProvider>
  );
}
