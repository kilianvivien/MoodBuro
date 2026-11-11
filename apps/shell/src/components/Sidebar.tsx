import type { KeyboardEvent, MouseEvent } from "react";
import {
  Bell,
  BookOpen,
  CalendarDays,
  Plus,
  Settings,
  Ellipsis,
} from "lucide-react";
import type { Site, UnreadStatus, Bookmark, SidebarTarget } from "../ipc";
import { BookmarkIcon } from "./Bookmarks";
import { displayHost, initials } from "../lib/site";
import { useI18n } from "../i18n/context";

export type View =
  | "site"
  | "onboarding"
  | "settings"
  | "downloads"
  | "inbox"
  | "courses"
  | "agenda"
  | "bookmarks"
  | "uploads";
/** Local shell views; anything else shows a Moodle view positioned by Rust. */
export const isLocal = (view: View) => view !== "site";
/** Views reached from the sidebar's top group. */
export type Destination = "inbox" | "courses" | "agenda";
const destinations = [
  { view: "inbox", icon: Bell, keys: "⌘1" },
  { view: "courses", icon: BookOpen, keys: "⌘2" },
  { view: "agenda", icon: CalendarDays, keys: "⌘3" },
] as const;

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
  unread = [],
  active,
  view,
  onSite,
  onView,
  onAdd,
  onSettings,
  bookmarks = [],
  onBookmark,
  onBookmarkMenu,
  onMenu,
}: {
  /** Right-click (or the menu key) on an item: show its native context menu. */
  onMenu?: (target: SidebarTarget) => void;
  sites: Site[];
  bookmarks?: Bookmark[];
  onBookmark?: (bookmark: Bookmark) => void;
  onBookmarkMenu?: () => void;
  unread?: UnreadStatus[];
  active: string | null;
  view: View;
  onSite: (id: string) => void;
  onView?: (view: Destination) => void;
  onAdd: () => void;
  onSettings: () => void;
}) {
  const { t } = useI18n();
  const total = unread.reduce((sum, status) => sum + status.unread, 0);
  const menu = (target: SidebarTarget) => (event: MouseEvent<HTMLElement>) => {
    if (!onMenu) return;
    event.preventDefault();
    onMenu(target);
  };
  return (
    <nav className="sidebar" aria-label={t.sidebar.label} onKeyDown={moveFocus}>
      {onView && sites.length > 0 && (
        <ul className="view-list" role="list" aria-label={t.sidebar.views}>
          {destinations.map(({ view: target, icon: Icon, keys }) => {
            const badge = target === "inbox" && total > 0;
            const label = badge
              ? `${t.sidebar[target]} — ${t.sidebar.unread(total)}`
              : t.sidebar[target];
            return (
              <li key={target}>
                <button
                  data-sidebar-item
                  className="sidebar-item"
                  title={`${label} (${keys})`}
                  aria-label={label}
                  aria-current={view === target ? "page" : undefined}
                  onClick={() => onView(target)}
                  onContextMenu={menu({ kind: target })}
                >
                  <Icon size={18} strokeWidth={1.75} aria-hidden="true" />
                  {badge && (
                    // Keyed by value so a changed count replays the pop.
                    <span
                      className="count-badge"
                      key={total}
                      aria-hidden="true"
                    >
                      {total > 99 ? "99+" : total}
                    </span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
      <ul
        className="site-list"
        role="list"
        aria-label={t.sidebar.organisations}
      >
        {sites.map((site) => {
          const status = unread.find((item) => item.siteId === site.id);
          const stateLabel = status
            ? t.sidebar.connection[status.connection]
            : "";
          const unreadLabel = status?.unread
            ? t.sidebar.unread(status.unread)
            : "";
          const label = [site.name, stateLabel, unreadLabel]
            .filter(Boolean)
            .join(" — ");
          const current = view === "site" && site.id === active;
          return (
            <li key={site.id}>
              <button
                data-sidebar-item
                className="sidebar-item site-avatar"
                title={`${label}\n${displayHost(site.wwwroot)}`}
                aria-label={label}
                aria-current={current ? "page" : undefined}
                onClick={() => onSite(site.id)}
                onContextMenu={menu({ kind: "site", id: site.id })}
              >
                <span aria-hidden="true">
                  {site.shortLabel || initials(site.name)}
                </span>
                {status &&
                  (status.unread > 0 ||
                    ["reauth", "offline", "error"].includes(
                      status.connection,
                    )) && (
                    <span
                      key={`${status.connection}:${status.unread}`}
                      className="site-status"
                      data-warning={status.connection !== "online" || undefined}
                      aria-hidden="true"
                    >
                      {["reauth", "error"].includes(status.connection)
                        ? "!"
                        : status.connection === "offline"
                          ? "−"
                          : status.unread > 99
                            ? "99+"
                            : status.unread}
                    </span>
                  )}
              </button>
            </li>
          );
        })}
        <li>
          <button
            data-sidebar-item
            className="sidebar-item add-site"
            title={`${t.sidebar.add} (⌘N)`}
            aria-label={t.sidebar.add}
            aria-current={view === "onboarding" ? "page" : undefined}
            onClick={onAdd}
            onContextMenu={menu({ kind: "add" })}
          >
            <Plus size={18} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </li>
      </ul>
      {bookmarks.length > 0 && (
        <ul className="bookmark-sidebar" aria-label={t.bookmarks.title}>
          {bookmarks.slice(0, 5).map((b) => (
            <li key={b.id}>
              <button
                data-sidebar-item
                className="sidebar-item"
                title={[b.name, sites.find((s) => s.id === b.siteId)?.name]
                  .filter(Boolean)
                  .join(" — ")}
                aria-label={b.name}
                onClick={() => onBookmark?.(b)}
                onContextMenu={menu({ kind: "bookmark", id: b.id })}
              >
                <BookmarkIcon name={b.icon} />
              </button>
            </li>
          ))}
          <li>
            <button
              data-sidebar-item
              className="sidebar-item"
              title={t.bookmarks.more}
              aria-label={t.bookmarks.more}
              aria-haspopup="menu"
              onClick={onBookmarkMenu}
            >
              <Ellipsis size={18} strokeWidth={1.75} aria-hidden="true" />
            </button>
          </li>
        </ul>
      )}
      <button
        data-sidebar-item
        className="sidebar-item settings-button"
        title={`${t.sidebar.settings} (⌘,)`}
        aria-label={t.sidebar.settings}
        aria-current={view === "settings" ? "page" : undefined}
        onClick={onSettings}
        onContextMenu={menu({ kind: "settings" })}
      >
        <Settings size={18} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </nav>
  );
}
