import { useEffect, useEffectEvent, useReducer, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  desktop,
  ipc,
  message,
  type MoodlePage,
  type NavAction,
  type UnreadStatus,
} from "./ipc";
import { useSites } from "./stores/sites";
import { useSettings } from "./stores/settings";
import { connection as reduce, idle } from "./lib/connection";
import {
  isLocal,
  Sidebar,
  type Destination,
  type View,
} from "./components/Sidebar";
import { QuickOpen, type QuickTarget } from "./components/QuickOpen";
import { Inbox } from "./views/Inbox";
import { Courses } from "./views/Courses";
import { Agenda } from "./views/Agenda";
import { Toolbar } from "./components/Toolbar";
import { Downloads } from "./views/Downloads";
import { AddSite } from "./views/AddSite";
import { Settings, type Notice } from "./views/Settings";
import { I18nProvider } from "./i18n/context";
import { resolveLocale, systemLanguages } from "./i18n/locale";
import { catalogues } from "./i18n/messages";

/** Last removal: the site's name on success, else the raw backend error. */
type Removal = { name: string | undefined } | { error: string };

export default function App() {
  const { sites, active, refresh, select } = useSites();
  const settings = useSettings();
  const [linkRequest, setLinkRequest] = useState(0);
  const [prefill, setPrefill] = useState({ url: "", revision: 0 });
  const consumingLink = useRef(false);
  const linkAgain = useRef(false);
  const [view, setView] = useState<View>("onboarding");
  const [unread, setUnread] = useState<UnreadStatus[]>([]);
  const [ready, setReady] = useState(false);
  // ⌘L palette, shown over whatever view is current.
  const [palette, setPalette] = useState(false);
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

  async function showSettings() {
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
  async function showLocal(target: Destination | "downloads") {
    setError(null);
    setPalette(false);
    try {
      await ipc.shellView(target);
      setView(target);
    } catch (cause) {
      failed(cause);
    }
  }
  const showDownloads = () => showLocal("downloads");

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

  async function openPalette() {
    if (sites.length === 0) return;
    setError(null);
    try {
      // The Moodle view sits above the shell; hide it while the palette is open.
      await ipc.shellView("quick-open");
      setPalette(true);
    } catch (cause) {
      failed(cause);
    }
  }
  /** Dismissed without a choice: bring back the Moodle view that was hidden. */
  function closePalette() {
    setPalette(false);
    if (view === "site" && site) void ipc.switchSite(site.id).catch(failed);
  }
  function choose(target: QuickTarget) {
    if (target.kind === "site") void openSite(target.siteId);
    else if (target.kind === "course")
      void openPage(target.siteId, "course", target.courseId);
    else if (target.kind === "activity")
      void openPage(target.siteId, "activity", target.activityId);
    else if (target.view === "settings") void showSettings();
    else void showLocal(target.view);
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
    void refresh().catch(failed);
    // Rust hid every site view; Settings stays, anything else falls back.
    setPalette(false);
    if (!isLocal(view)) setView("onboarding");
  });
  /** App-menu commands (Settings…, Add Organisation…); Rust already hid sites. */
  const onNavigate = useEffectEvent((target: string) => {
    if (target === "settings") void showSettings();
    else if (target === "onboarding") void showOnboarding();
    else if (
      target === "downloads" ||
      target === "inbox" ||
      target === "courses" ||
      target === "agenda"
    )
      void showLocal(target);
    else if (target === "quick-open") {
      if (sites.length > 0) setPalette(true);
      else leave();
    }
  });
  const onAuthFinished = useEffectEvent((result: string | null) =>
    dispatch({ type: "finished", error: result }),
  );
  const onLaunch = useEffectEvent(async (cancelled: () => boolean) => {
    // Load the language before the first view renders, without delaying sites.
    const settingsLoaded = settings.load();
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
    // The native menu owns ⌘L in the app; the browser preview needs its own.
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
    if (!desktop) return stop;
    const listeners = [
      listen("deep-link://pending", () =>
        setLinkRequest((revision) => revision + 1),
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
      listen<string>("shell://error", (e) => setError(e.payload)),
      listen<string | null>("auth://finished", (e) =>
        onAuthFinished(e.payload),
      ),
    ];
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
      <div className="app">
        <Sidebar
          sites={sites}
          unread={unread}
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
          error={error}
          canLeave={view !== "onboarding" || sites.length > 0}
          onNav={(action: NavAction) => void ipc.nav(action).catch(failed)}
          onDismissError={() => setError(null)}
          onLeave={leave}
          onDownloads={() => void showDownloads()}
          paused={palette}
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
              sites={sites}
              onOpenCourse={(id, course) => void openPage(id, "course", course)}
            />
          )}
          {ready && view === "agenda" && (
            <Agenda
              sites={sites}
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
              sites={sites}
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
              desktop={desktop}
              sites={sites}
              appearance={settings.appearance}
              appearanceSaving={settings.saving.appearance}
              language={settings.language}
              languageSaving={settings.saving.language}
              settingsError={settings.error}
              connection={connection}
              accountMessage={accountMessage}
              onAppearance={(value) => void settings.setAppearance(value)}
              onLanguage={(value) => void settings.setLanguage(value)}
              onOpenSite={(id) => void openSite(id)}
              onReauthenticate={(id) => void reauthenticate(id)}
              onRemove={removeSite}
              onAdd={() => void showOnboarding()}
            />
          )}
        </main>
      </div>
    </I18nProvider>
  );
}
