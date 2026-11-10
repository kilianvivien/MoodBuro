import {
  ArrowLeft,
  ArrowRight,
  CircleAlert,
  CircleCheck,
  LoaderCircle,
  ExternalLink,
  Download,
  House,
  RotateCw,
  Search,
  BookmarkPlus,
  Sparkles,
  X,
} from "lucide-react";
import type {
  DownloadFeedback,
  NavAction,
  Site,
  SessionRecovery,
} from "../ipc";
import { displayHost } from "../lib/site";
import type { View } from "./Sidebar";
import { useI18n } from "../i18n/context";

const icon = { size: 16, strokeWidth: 1.75, "aria-hidden": true } as const;

export function Toolbar({
  view,
  site,
  error,
  recovery,
  canLeave,
  onNav,
  onDismissError,
  onLeave,
  onDownloads,
  onQuickOpen,
  onBookmark,
  downloadFeedback = null,
  onDismissDownload,
  onAssistant,
  assistantOpen = false,
  paused = false,
}: {
  /** Shows or hides the AI assistant panel beside the content. */
  onAssistant?: () => void;
  assistantOpen?: boolean;
  view: View;
  site: Site | undefined;
  /**
   * Raw backend message, translated here. Lives in the toolbar because a
   * remote site view covers the content area.
   */
  error: string | null;
  recovery?: SessionRecovery;
  /** A site exists to return to from onboarding/settings. */
  canLeave: boolean;
  onNav: (action: NavAction) => void;
  onDismissError: () => void;
  onLeave: () => void;
  onDownloads?: () => void;
  downloadFeedback?: DownloadFeedback | null;
  onDismissDownload?: () => void;
  onQuickOpen?: () => void;
  onBookmark?: () => void;
  /** Quick Open covers the Moodle view: page commands have nothing to act on. */
  paused?: boolean;
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
  const session = !alert && recovery && (
    <div
      className="toolbar-alert session-recovery"
      role="status"
      title={t.toolbar.sessionHelp}
    >
      {recovery.phase !== "failed" && (
        <LoaderCircle className="spin" {...icon} />
      )}
      <span>
        {recovery.phase === "waiting"
          ? t.toolbar.sessionWaiting(
              i18n.date(recovery.retryAt * 1000, {
                hour: "2-digit",
                minute: "2-digit",
              }),
            )
          : recovery.phase === "retrying"
            ? t.toolbar.sessionRetrying
            : t.toolbar.sessionFailed}
      </span>
    </div>
  );
  const feedback = !alert && !session && downloadFeedback && (
    <div
      className="download-feedback"
      data-state={downloadFeedback.state}
      role="status"
    >
      {downloadFeedback.state === "downloading" ? (
        <LoaderCircle className="spin" {...icon} />
      ) : downloadFeedback.state === "complete" ? (
        <CircleCheck {...icon} />
      ) : (
        <CircleAlert {...icon} />
      )}
      <button
        className="download-feedback-link"
        onClick={onDownloads}
        title={t.downloads.feedback[downloadFeedback.state](
          downloadFeedback.filename,
        )}
      >
        {t.downloads.feedback[downloadFeedback.state](
          downloadFeedback.filename,
        )}
      </button>
      <button
        className="tool small"
        aria-label={t.toolbar.dismissLabel}
        title={t.toolbar.dismiss}
        onClick={onDismissDownload}
      >
        <X size={13} aria-hidden="true" />
      </button>
    </div>
  );
  const quickOpen = onQuickOpen && (
    <button
      className="tool"
      title={t.toolbar.quickOpen}
      aria-label={t.toolbar.quickOpen}
      onClick={onQuickOpen}
    >
      <Search {...icon} />
    </button>
  );
  const assistant = onAssistant && (
    <button
      className="tool"
      title={t.assistant.toggle}
      aria-label={t.assistant.title}
      aria-pressed={assistantOpen}
      onClick={onAssistant}
    >
      <Sparkles {...icon} />
    </button>
  );
  const local =
    view === "site" || view === "onboarding"
      ? t.toolbar.addOrganisation
      : t.toolbar[view];
  return (
    // "deep": the whole bar drags the window; buttons still click.
    <header className="toolbar" data-tauri-drag-region="deep">
      {browsing ? (
        <>
          <div
            className="toolbar-group"
            role="group"
            aria-label={t.toolbar.navigation}
          >
            <button
              className="tool"
              title={t.toolbar.home}
              aria-label={t.toolbar.home}
              disabled={paused}
              onClick={() => onNav("home")}
            >
              <House {...icon} />
            </button>
            <button
              className="tool"
              title={t.toolbar.back}
              aria-label={t.toolbar.back}
              disabled={paused}
              onClick={() => onNav("back")}
            >
              <ArrowLeft {...icon} />
            </button>
            <button
              className="tool"
              title={t.toolbar.forward}
              aria-label={t.toolbar.forward}
              disabled={paused}
              onClick={() => onNav("forward")}
            >
              <ArrowRight {...icon} />
            </button>
            <button
              className="tool"
              title={t.toolbar.reload}
              aria-label={t.toolbar.reload}
              disabled={paused}
              onClick={() => onNav("reload")}
            >
              <RotateCw size={15} strokeWidth={1.75} aria-hidden="true" />
            </button>
          </div>
          <div className="toolbar-title" key={site.id}>
            <h1>{site.name}</h1>
            <span>{displayHost(site.wwwroot)}</span>
          </div>
          {alert}
          {session}
          {feedback}
          {quickOpen}
          {onBookmark && (
            <button
              className="tool"
              title={t.bookmarks.add}
              aria-label={t.bookmarks.add}
              disabled={paused}
              onClick={onBookmark}
            >
              <BookmarkPlus {...icon} />
            </button>
          )}
          {onDownloads && (
            <button
              className="tool"
              title={t.toolbar.downloads}
              aria-label={t.toolbar.downloads}
              onClick={onDownloads}
            >
              <Download {...icon} />
            </button>
          )}
          <button
            className="tool"
            title={t.toolbar.openInBrowser}
            aria-label={t.toolbar.openInBrowser}
            disabled={paused}
            onClick={() => onNav("open_in_browser")}
          >
            <ExternalLink size={15} strokeWidth={1.75} aria-hidden="true" />
          </button>
          {assistant}
        </>
      ) : (
        <>
          <div className="toolbar-title" key={view}>
            <h1>{local}</h1>
          </div>
          {alert}
          {session}
          {feedback}
          {view !== "onboarding" && quickOpen}
          {view !== "onboarding" && view !== "downloads" && onDownloads && (
            <button
              className="tool"
              title={t.toolbar.downloads}
              aria-label={t.toolbar.downloads}
              onClick={onDownloads}
            >
              <Download {...icon} />
            </button>
          )}
          {assistant}
          {canLeave && (
            <button className="button" onClick={onLeave}>
              {view === "onboarding" ? t.toolbar.cancel : t.toolbar.done}
            </button>
          )}
        </>
      )}
    </header>
  );
}
