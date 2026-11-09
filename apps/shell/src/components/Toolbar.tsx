import {
  ArrowLeft,
  ArrowRight,
  CircleAlert,
  ExternalLink,
  RotateCw,
  X,
} from "lucide-react";
import type { NavAction, Site } from "../ipc";
import { displayHost } from "../lib/site";
import type { View } from "./Sidebar";
import { useI18n } from "../i18n/context";

const icon = { size: 16, strokeWidth: 1.75, "aria-hidden": true } as const;

export function Toolbar({
  view,
  site,
  error,
  canLeave,
  onNav,
  onDismissError,
  onLeave,
}: {
  view: View;
  site: Site | undefined;
  /**
   * Raw backend message, translated here. Lives in the toolbar because a
   * remote site view covers the content area.
   */
  error: string | null;
  /** A site exists to return to from onboarding/settings. */
  canLeave: boolean;
  onNav: (action: NavAction) => void;
  onDismissError: () => void;
  onLeave: () => void;
}) {
  const i18n = useI18n();
  const { t } = i18n;
  const browsing = view === "site" && site;
  // No room for a second line: an untranslated original goes in the tooltip.
  const described = error ? i18n.error(error, "navigation") : null;
  const alert = described && (
    <div className="toolbar-alert" role="alert">
      <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
      <span
        title={
          described.detail
            ? `${described.text}\n${t.errors.detail} ${described.detail}`
            : described.text
        }
      >
        {described.text}
      </span>
      <button
        className="tool small"
        title={t.toolbar.dismiss}
        aria-label={t.toolbar.dismissLabel}
        onClick={onDismissError}
      >
        <X size={13} strokeWidth={2} aria-hidden="true" />
      </button>
    </div>
  );
  return (
    <header className="toolbar">
      {browsing ? (
        <>
          <div
            className="toolbar-group"
            role="group"
            aria-label={t.toolbar.navigation}
          >
            <button
              className="tool"
              title={t.toolbar.back}
              aria-label={t.toolbar.back}
              onClick={() => onNav("back")}
            >
              <ArrowLeft {...icon} />
            </button>
            <button
              className="tool"
              title={t.toolbar.forward}
              aria-label={t.toolbar.forward}
              onClick={() => onNav("forward")}
            >
              <ArrowRight {...icon} />
            </button>
            <button
              className="tool"
              title={t.toolbar.reload}
              aria-label={t.toolbar.reload}
              onClick={() => onNav("reload")}
            >
              <RotateCw size={15} strokeWidth={1.75} aria-hidden="true" />
            </button>
          </div>
          <div className="toolbar-title">
            <h1>{site.name}</h1>
            <span>{displayHost(site.wwwroot)}</span>
          </div>
          {alert}
          <button
            className="tool"
            title={t.toolbar.openInBrowser}
            aria-label={t.toolbar.openInBrowser}
            onClick={() => onNav("open_in_browser")}
          >
            <ExternalLink size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
        </>
      ) : (
        <>
          <div className="toolbar-title">
            <h1>
              {view === "settings"
                ? t.toolbar.settings
                : t.toolbar.addOrganisation}
            </h1>
          </div>
          {alert}
          {canLeave && (
            <button className="button" onClick={onLeave}>
              {view === "settings" ? t.toolbar.done : t.toolbar.cancel}
            </button>
          )}
        </>
      )}
    </header>
  );
}
