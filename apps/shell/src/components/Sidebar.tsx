import type { KeyboardEvent } from "react";
import { Plus, Settings } from "lucide-react";
import type { Site } from "../ipc";
import { displayHost, initials } from "../lib/site";
import { useI18n } from "../i18n/context";

export type View = "site" | "onboarding" | "settings";

/** Up/Down/Home/End move focus between sidebar items, like a native source list. */
function moveFocus(event: KeyboardEvent<HTMLElement>) {
  const keys = ["ArrowDown", "ArrowUp", "Home", "End"];
  if (!keys.includes(event.key)) return;
  const items = Array.from(
    event.currentTarget.querySelectorAll<HTMLButtonElement>(
      "[data-sidebar-item]",
    ),
  );
  const index = items.indexOf(document.activeElement as HTMLButtonElement);
  if (index < 0) return;
  event.preventDefault();
  const next =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? items.length - 1
        : (index + (event.key === "ArrowDown" ? 1 : -1) + items.length) %
          items.length;
  items[next].focus();
}

export function Sidebar({
  sites,
  active,
  view,
  onSite,
  onAdd,
  onSettings,
}: {
  sites: Site[];
  active: string | null;
  view: View;
  onSite: (id: string) => void;
  onAdd: () => void;
  onSettings: () => void;
}) {
  const { t } = useI18n();
  return (
    <nav className="sidebar" aria-label={t.sidebar.label} onKeyDown={moveFocus}>
      <ul className="site-list" role="list">
        {sites.map((site) => {
          const current = view === "site" && site.id === active;
          return (
            <li key={site.id}>
              <button
                data-sidebar-item
                className="sidebar-item site-avatar"
                title={`${site.name}\n${displayHost(site.wwwroot)}`}
                aria-label={site.name}
                aria-current={current ? "page" : undefined}
                onClick={() => onSite(site.id)}
              >
                <span aria-hidden="true">
                  {site.shortLabel || initials(site.name)}
                </span>
              </button>
            </li>
          );
        })}
        <li>
          <button
            data-sidebar-item
            className="sidebar-item add-site"
            title={t.sidebar.add}
            aria-label={t.sidebar.add}
            aria-current={view === "onboarding" ? "page" : undefined}
            onClick={onAdd}
          >
            <Plus size={18} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </li>
      </ul>
      <button
        data-sidebar-item
        className="sidebar-item settings-button"
        title={t.sidebar.settings}
        aria-label={t.sidebar.settings}
        aria-current={view === "settings" ? "page" : undefined}
        onClick={onSettings}
      >
        <Settings size={18} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </nav>
  );
}
