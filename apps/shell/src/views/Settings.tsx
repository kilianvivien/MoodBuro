import { useEffect, useRef, useState, type ReactNode } from "react";
import { Segmented } from "../components/ui";
import {
  ArrowUp,
  ArrowDown,
  CircleAlert,
  CircleCheck,
  Languages,
  LoaderCircle,
  Monitor,
  Moon,
  Plus,
  Sun,
} from "lucide-react";
import {
  ipc,
  message,
  type Appearance,
  type FileHandling,
  type Language,
  type Site,
} from "../ipc";
import { useSites } from "../stores/sites";
import type { Connection } from "../lib/connection";
import { displayHost, initials } from "../lib/site";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import type { ErrorContext } from "../i18n/errors";

/** Outcome of the last account action; errors stay raw until rendered. */
export type Notice =
  | { tone: "success"; text: string }
  | { tone: "error"; error: string; context: ErrorContext };

const appearanceIcons: Record<Appearance, typeof Sun> = {
  system: Monitor,
  light: Sun,
  dark: Moon,
};
const languageOptions: readonly Language[] = ["system", "fr", "en"];
const handlingOptions: readonly FileHandling[] = ["inline", "save", "open"];
/** The file types with a handling preference, in display order. */
const fileKinds = ["pdf", "images"] as const;
export type FileKind = (typeof fileKinds)[number];

function RemoveConfirmation({
  site,
  removing,
  onCancel,
  onConfirm,
}: {
  site: Site;
  removing: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => cancel.current?.focus(), []);
  const titleId = `remove-${site.id}-title`;
  return (
    <div
      className="confirm"
      role="alertdialog"
      aria-labelledby={titleId}
      aria-describedby={`remove-${site.id}-body`}
      onKeyDown={(event) => {
        if (event.key === "Escape" && !removing) {
          // Close only the confirmation, not Settings itself.
          event.preventDefault();
          event.stopPropagation();
          onCancel();
        }
      }}
    >
      <p id={titleId} className="confirm-title">
        {t.settings.removeTitle(site.name)}
      </p>
      <p id={`remove-${site.id}-body`} className="confirm-body">
        {t.settings.removeBody}
      </p>
      <div className="confirm-actions">
        <button
          ref={cancel}
          className="button"
          onClick={onCancel}
          disabled={removing}
        >
          {t.settings.cancel}
        </button>
        <button
          className="button destructive"
          onClick={onConfirm}
          disabled={removing}
        >
          {removing && (
            <LoaderCircle
              className="spin"
              size={14}
              strokeWidth={2}
              aria-hidden="true"
            />
          )}
          {t.settings.confirmRemove}
        </button>
      </div>
    </div>
  );
}

function SiteRow({
  site,
  connection,
  onOpen,
  onReauthenticate,
  onRemove,
  order,
}: {
  order?: {
    first: boolean;
    last: boolean;
    busy: boolean;
    move: (direction: -1 | 1) => void;
  };
  site: Site;
  connection: Connection;
  onOpen: () => void;
  onReauthenticate: () => void;
  onRemove: () => Promise<boolean>;
}) {
  const { t } = useI18n();
  const refresh = useSites((state) => state.refresh);
  const [shortLabel, setShortLabel] = useState(site.shortLabel ?? "");
  const [labelBusy, setLabelBusy] = useState(false);
  const [labelError, setLabelError] = useState<string | null>(null);
  const [labelSaved, setLabelSaved] = useState(false);
  async function saveLabel(event: React.FormEvent) {
    event.preventDefault();
    setLabelBusy(true);
    setLabelError(null);
    setLabelSaved(false);
    try {
      const saved = await ipc.setShortLabel(site.id, shortLabel);
      setShortLabel(saved ?? "");
      await refresh();
      setLabelSaved(true);
    } catch (cause) {
      setLabelError(message(cause));
    } finally {
      setLabelBusy(false);
    }
  }
  const [confirming, setConfirming] = useState(false);
  const [removing, setRemoving] = useState(false);
  const removeButton = useRef<HTMLButtonElement>(null);
  const signingIn =
    connection.phase !== "idle" && connection.siteId === site.id;
  const busy = connection.phase !== "idle";

  async function confirm() {
    setRemoving(true);
    // On success this row unmounts; only restore state when removal failed.
    if (!(await onRemove())) {
      setRemoving(false);
      setConfirming(false);
      removeButton.current?.focus();
    }
  }

  return (
    <li className="site-row">
      <div className="site-row-main">
        <span className="row-avatar" aria-hidden="true">
          {site.shortLabel || initials(site.name)}
        </span>
        <div className="row-text">
          <span className="row-title">{site.name}</span>
          <span className="row-subtitle">
            {[site.fullName, displayHost(site.wwwroot)]
              .filter(Boolean)
              .join(" · ")}
          </span>
          {signingIn && (
            <span className="row-status" role="status">
              <LoaderCircle
                className="spin"
                size={12}
                strokeWidth={2}
                aria-hidden="true"
              />
              {connection.phase === "checking"
                ? t.settings.connecting
                : t.settings.continueInWindow}
            </span>
          )}
        </div>
        {!confirming && (
          <div className="row-actions">
            {order && (
              <>
                <button
                  className="button small icon"
                  title={t.settings.moveUp(site.name)}
                  aria-label={t.settings.moveUp(site.name)}
                  disabled={order.first || order.busy || busy}
                  onClick={() => order.move(-1)}
                >
                  <ArrowUp size={14} aria-hidden="true" />
                </button>
                <button
                  className="button small icon"
                  title={t.settings.moveDown(site.name)}
                  aria-label={t.settings.moveDown(site.name)}
                  disabled={order.last || order.busy || busy}
                  onClick={() => order.move(1)}
                >
                  <ArrowDown size={14} aria-hidden="true" />
                </button>
              </>
            )}
            <button className="button" onClick={onOpen}>
              {t.settings.open}
            </button>
            <button
              className="button"
              onClick={onReauthenticate}
              disabled={busy}
              aria-label={t.settings.signInAgainLabel(site.name)}
            >
              {t.settings.signInAgain}
            </button>
            <button
              ref={removeButton}
              className="button"
              onClick={() => setConfirming(true)}
              disabled={signingIn}
              aria-label={t.settings.removeLabel(site.name)}
            >
              {t.settings.remove}
            </button>
          </div>
        )}
      </div>
      {!confirming && (
        <form
          className="site-label-editor"
          onSubmit={(event) => void saveLabel(event)}
        >
          <div className="row-text">
            <label htmlFor={`label-${site.id}`}>{t.settings.shortLabel}</label>
            <span id={`label-hint-${site.id}`} className="label-hint">
              {t.settings.shortLabelHint}
            </span>
            {labelSaved && (
              <span className="label-feedback success" role="status">
                <CircleCheck size={12} strokeWidth={2} aria-hidden="true" />
                {t.settings.labelSaved}
              </span>
            )}
            {labelError && (
              <span className="label-feedback error" role="alert">
                <CircleAlert size={12} strokeWidth={2} aria-hidden="true" />
                <ErrorText error={labelError} context="settings" />
              </span>
            )}
          </div>
          <div className="site-label-controls">
            <div
              className="text-field compact"
              data-disabled={labelBusy || undefined}
            >
              <input
                id={`label-${site.id}`}
                value={shortLabel}
                maxLength={4}
                placeholder={initials(site.name)}
                onChange={(event) => {
                  setShortLabel(event.target.value);
                  setLabelSaved(false);
                }}
                disabled={labelBusy}
                aria-describedby={`label-hint-${site.id}`}
                autoComplete="off"
                spellCheck={false}
              />
            </div>
            <button
              className="button small"
              disabled={labelBusy || shortLabel === (site.shortLabel ?? "")}
            >
              {t.settings.saveLabel}
            </button>
          </div>
        </form>
      )}
      {confirming && (
        <RemoveConfirmation
          site={site}
          removing={removing}
          onCancel={() => {
            setConfirming(false);
            removeButton.current?.focus();
          }}
          onConfirm={() => void confirm()}
        />
      )}
    </li>
  );
}

export function Settings({
  desktop,
  sites,
  appearance,
  appearanceSaving,
  language,
  languageSaving,
  globalShortcut,
  globalShortcutSaving,
  shortcutUnavailable,
  files,
  filesSaving,
  settingsError,
  settingsErrorIn,
  connection,
  accountMessage,
  onAppearance,
  onLanguage,
  onGlobalShortcut,
  onFileHandling,
  onOpenSite,
  onReauthenticate,
  onRemove,
  onAdd,
  children,
}: {
  /** Further sections (the AI assistant), shown above the organisations. */
  children?: ReactNode;
  desktop: boolean;
  sites: Site[];
  appearance: Appearance;
  appearanceSaving: boolean;
  language: Language;
  languageSaving: boolean;
  globalShortcut: boolean;
  globalShortcutSaving: boolean;
  /** The saved shortcut could not be registered when the app started. */
  shortcutUnavailable: boolean;
  files: Record<FileKind, FileHandling>;
  filesSaving: Record<FileKind, boolean>;
  /** Raw backend message from loading or saving a setting. */
  settingsError: string | null;
  /** The group the failed setting belongs to. */
  settingsErrorIn: "general" | "files";
  connection: Connection;
  /** Outcome of the last account action, announced under the list. */
  accountMessage: Notice | null;
  onAppearance: (value: Appearance) => void;
  onLanguage: (value: Language) => void;
  onGlobalShortcut: (enabled: boolean) => void;
  onFileHandling: (kind: FileKind, value: FileHandling) => void;
  onOpenSite: (id: string) => void;
  onReauthenticate: (id: string) => void;
  onRemove: (id: string) => Promise<boolean>;
  onAdd: () => void;
}) {
  const [orderBusy, setOrderBusy] = useState(false);
  const [orderError, setOrderError] = useState<string | null>(null);
  const orderPending = useRef(false);
  async function moveSite(id: string, direction: -1 | 1) {
    if (orderPending.current) return;
    orderPending.current = true;
    setOrderBusy(true);
    setOrderError(null);
    try {
      await ipc.moveSite(id, direction);
      await useSites.getState().refresh();
    } catch (error) {
      setOrderError(message(error));
    } finally {
      orderPending.current = false;
      setOrderBusy(false);
    }
  }
  const { t } = useI18n();
  return (
    <div className="settings">
      {!desktop && (
        <p className="preview-banner" role="note">
          {t.settings.previewBanner}
        </p>
      )}

      <section aria-labelledby="general-heading">
        <h2 id="general-heading" className="section-title">
          {t.settings.general}
        </h2>
        <div className="group">
          <div className="group-row">
            <div className="row-text">
              <span className="row-title">{t.settings.theme}</span>
              <span className="row-subtitle">{t.settings.themeHint}</span>
            </div>
            <Segmented
              name="appearance"
              legend={t.settings.theme}
              value={appearance}
              disabled={appearanceSaving}
              onChange={onAppearance}
              options={(["system", "light", "dark"] as const).map((value) => {
                const Icon = appearanceIcons[value];
                return {
                  value,
                  label: (
                    <>
                      <Icon size={14} strokeWidth={1.75} aria-hidden="true" />
                      {t.settings.appearance[value]}
                    </>
                  ),
                };
              })}
            />
          </div>
          <div className="group-row">
            <div className="row-text">
              <span className="row-title">{t.settings.language}</span>
              <span className="row-subtitle">{t.settings.languageHint}</span>
            </div>
            <Segmented
              name="language"
              legend={t.settings.language}
              value={language}
              disabled={languageSaving}
              onChange={onLanguage}
              options={languageOptions.map((value) => ({
                value,
                // Language names read in their own language, as on macOS.
                lang: value === "system" ? undefined : value,
                label:
                  value === "system" ? (
                    <>
                      <Languages
                        size={14}
                        strokeWidth={1.75}
                        aria-hidden="true"
                      />
                      {t.settings.languages.system}
                    </>
                  ) : (
                    t.settings.languages[value]
                  ),
              }))}
            />
          </div>
          <div className="group-row">
            <div className="row-text">
              <span className="row-title">{t.settings.globalShortcut}</span>
              <span className="row-subtitle">
                {t.settings.globalShortcutHint}
              </span>
            </div>
            <Segmented
              name="global-shortcut"
              legend={t.settings.globalShortcut}
              value={globalShortcut ? "on" : "off"}
              disabled={globalShortcutSaving}
              onChange={(value) => onGlobalShortcut(value === "on")}
              options={(["off", "on"] as const).map((value) => ({
                value,
                label: t.settings.shortcutOptions[value],
              }))}
            />
          </div>
        </div>
        {settingsError && settingsErrorIn === "general" && (
          <p className="section-message error" role="alert">
            <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
            <ErrorText error={settingsError} context="settings" />
          </p>
        )}
        {globalShortcut && shortcutUnavailable && (
          <p className="section-message error" role="status">
            <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
            {t.settings.shortcutUnavailable}
          </p>
        )}
        {globalShortcut && (
          <p className="section-footnote">{t.settings.shortcutFootnote}</p>
        )}
      </section>

      <section aria-labelledby="files-heading">
        <h2 id="files-heading" className="section-title">
          {t.settings.files}
        </h2>
        <div className="group">
          {fileKinds.map((kind) => (
            <div className="group-row" key={kind}>
              <div className="row-text">
                <span className="row-title">{t.settings[kind]}</span>
                <span className="row-subtitle">
                  {t.settings[`${kind}Hint`]}
                </span>
              </div>
              <Segmented
                name={`handling-${kind}`}
                legend={t.settings[kind]}
                value={files[kind]}
                disabled={filesSaving[kind]}
                onChange={(value) => onFileHandling(kind, value)}
                options={handlingOptions.map((value) => ({
                  value,
                  label: t.settings.handling[value],
                }))}
              />
            </div>
          ))}
        </div>
        {settingsError && settingsErrorIn === "files" && (
          <p className="section-message error" role="alert">
            <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
            <ErrorText error={settingsError} context="settings" />
          </p>
        )}
        <details className="help-disclosure section-footnote">
          <summary>{t.common.details}</summary>
          <p>{t.settings.filesFootnote}</p>
        </details>
      </section>

      {children}

      <section aria-labelledby="accounts-heading">
        <div className="section-header">
          <h2 id="accounts-heading" className="section-title">
            {t.settings.organisations}
          </h2>
          <span className="section-count">
            {t.settings.siteCount(sites.length)}
          </span>
        </div>
        {orderError && (
          <p className="section-message error" role="alert">
            <ErrorText error={orderError} context="settings" />
          </p>
        )}
        <div className="group">
          {sites.length > 0 ? (
            <ul className="site-rows" role="list">
              {sites.map((site, index) => (
                <SiteRow
                  key={site.id}
                  site={site}
                  order={{
                    first: index === 0,
                    last: index === sites.length - 1,
                    busy: orderBusy,
                    move: (direction) => void moveSite(site.id, direction),
                  }}
                  connection={connection}
                  onOpen={() => onOpenSite(site.id)}
                  onReauthenticate={() => onReauthenticate(site.id)}
                  onRemove={() => onRemove(site.id)}
                />
              ))}
            </ul>
          ) : (
            <p className="group-empty">{t.settings.empty}</p>
          )}
          <div className="group-row group-footer">
            <button className="button" onClick={onAdd}>
              <Plus size={14} strokeWidth={2} aria-hidden="true" />
              {t.settings.add}
            </button>
          </div>
        </div>
        {accountMessage && (
          <p
            className={`section-message ${accountMessage.tone}`}
            role={accountMessage.tone === "error" ? "alert" : "status"}
          >
            {accountMessage.tone === "error" ? (
              <>
                <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
                <ErrorText
                  error={accountMessage.error}
                  context={accountMessage.context}
                />
              </>
            ) : (
              <>
                <CircleCheck size={14} strokeWidth={2} aria-hidden="true" />
                {accountMessage.text}
              </>
            )}
          </p>
        )}
        <details className="help-disclosure section-footnote">
          <summary>{t.common.details}</summary>
          <p>{t.settings.orderHint}</p>
          <p>{t.settings.footnote}</p>
        </details>
      </section>
    </div>
  );
}
