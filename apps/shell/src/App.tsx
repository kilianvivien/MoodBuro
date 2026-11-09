import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ArrowLeft,
  ArrowRight,
  ExternalLink,
  Plus,
  RotateCw,
  BookOpen,
  PanelsTopLeft,
} from "lucide-react";
import { ipc, desktop } from "./ipc";
import { useSites } from "./stores/sites";
import { AddSite } from "./views/AddSite";
export default function App() {
  const { sites, active, refresh, select } = useSites();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const selected = sites.find((site) => site.id === active);
  function failed(cause: unknown) {
    setError(cause instanceof Error ? cause.message : String(cause));
  }
  useEffect(() => {
    void refresh().catch(failed);
    if (!desktop) return;
    const listeners = [
      listen<string>("sites://changed", (event) => {
        select(event.payload);
        void refresh().catch(failed);
      }),
      listen<string | null>("auth://finished", (event) => {
        setPending(false);
        setError(event.payload);
      }),
    ];
    return () => {
      for (const listener of listeners)
        void listener.then((unlisten) => unlisten());
    };
  }, [refresh, select]);
  async function addSite(url: string) {
    setPending(true);
    setError(null);
    try {
      await ipc.addSite(url);
    } catch (cause) {
      failed(cause);
      setPending(false);
    }
  }
  async function switchSite(id: string) {
    setError(null);
    try {
      await ipc.switchSite(id);
      select(id);
    } catch (cause) {
      failed(cause);
    }
  }
  async function showOnboarding() {
    try {
      await ipc.onboarding();
      select(null);
      setError(null);
    } catch (cause) {
      failed(cause);
    }
  }
  return (
    <div className="app">
      <aside className="sidebar" aria-label="Moodle sites">
        <button
          className="brand"
          aria-label="MoodBuro home"
          onClick={() => void showOnboarding()}
        >
          <BookOpen size={25} />
        </button>
        <div className="site-list">
          {sites.map((site) => (
            <button
              key={site.id}
              title={site.name}
              aria-label={`Open ${site.name}`}
              aria-pressed={site.id === active}
              className={`site-avatar ${site.id === active ? "selected" : ""}`}
              onClick={() => void switchSite(site.id)}
            >
              {site.name.slice(0, 2).toUpperCase()}
            </button>
          ))}
          <button
            className={`add-site ${!active ? "selected" : ""}`}
            aria-label="Add organisation"
            onClick={() => void showOnboarding()}
          >
            <Plus size={22} />
          </button>
        </div>
        <span className="sidebar-bottom" title="Alpha foundation">
          <PanelsTopLeft size={20} />
        </span>
      </aside>
      <header className="toolbar">
        <div className="navigation">
          <button
            aria-label="Go back"
            disabled={!active}
            onClick={() => void ipc.nav("back").catch(failed)}
          >
            <ArrowLeft size={16} />
          </button>
          <button
            aria-label="Go forward"
            disabled={!active}
            onClick={() => void ipc.nav("forward").catch(failed)}
          >
            <ArrowRight size={16} />
          </button>
          <button
            aria-label="Reload"
            disabled={!active}
            onClick={() => void ipc.nav("reload").catch(failed)}
          >
            <RotateCw size={14} />
          </button>
        </div>
        <span className="toolbar-title">
          {selected?.name ?? "Welcome to MoodBuro"}
        </span>
        <button
          className="external"
          aria-label="Open current page in browser"
          disabled={!active}
          onClick={() => void ipc.nav("open_in_browser").catch(failed)}
        >
          <ExternalLink size={15} />
        </button>
      </header>
      <main>
        {!active && (
          <AddSite
            pending={pending}
            error={error}
            onSubmit={(url) => void addSite(url)}
          />
        )}
        {active && error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        {!active && (
          <footer>
            <span>
              MoodBuro <b>·</b> A home for your Moodle
            </span>
            <span>Made for your desktop</span>
          </footer>
        )}
      </main>
    </div>
  );
}
