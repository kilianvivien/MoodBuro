import { useEffect, useEffectEvent, useReducer, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  desktop,
  ipc,
  message,
  type NavAction,
  type UnreadStatus,
} from "./ipc";
import { useSites } from "./stores/sites";
import { useSettings } from "./stores/settings";
import { connection as reduce, idle } from "./lib/connection";
import { Sidebar, type View } from "./components/Sidebar";
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

  async function showDownloads() {
    setError(null);
    try {
      await ipc.shellView("downloads");
      setView("downloads");
    } catch (cause) {
      failed(cause);
    }
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
    if (view === "settings" || view === "downloads") {
      // Rust may have revealed the site; keep Settings on top.
      void ipc.shellView(view).catch(failed);
      return;
    }
    select(id);
    setView("site");
  });
  const onSiteRemoved = useEffectEvent(() => {
    void refresh().catch(failed);
    // Rust hid every site view; Settings stays, anything else falls back.
    if (view !== "settings" && view !== "downloads") setView("onboarding");
  });
  /** App-menu commands (Settings…, Add Organisation…); Rust already hid sites. */
  const onNavigate = useEffectEvent((target: string) => {
    if (target === "settings") void showSettings();
    else if (target === "onboarding") void showOnboarding();
    else if (target === "downloads") void showDownloads();
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
    if (event.key !== "Escape" || event.defaultPrevented) return;
    if (
      view === "settings" ||
      view === "downloads" ||
      (view === "onboarding" && sites.length > 0)
    )
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
    if (!desktop) return stop;
    void ipc.unread().then(setUnread).catch(failed);
    const listeners = [
      listen("deep-link://pending", () =>
        setLinkRequest((revision) => revision + 1),
      ),
      listen<UnreadStatus[]>("unread://changed", (e) => setUnread(e.payload)),
      listen<string>("navigation://site-opened", (e) => {
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
          onAdd={() => void showOnboarding()}
          onSettings={() => void showSettings()}
        />
        <Toolbar
          view={view}
          site={site}
          error={error}
          canLeave={
            view === "settings" || view === "downloads" || sites.length > 0
          }
          onNav={(action: NavAction) => void ipc.nav(action).catch(failed)}
          onDismissError={() => setError(null)}
          onLeave={leave}
          onDownloads={() => void showDownloads()}
        />
        <main className="content" aria-busy={!ready}>
          {ready && view === "onboarding" && (
            <AddSite
              key={prefill.revision}
              initialUrl={prefill.url}
              connection={connection}
              firstRun={sites.length === 0}
              onSubmit={(url) => void addSite(url)}
            />
          )}
          {ready && view === "downloads" && <Downloads sites={sites} />}
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
