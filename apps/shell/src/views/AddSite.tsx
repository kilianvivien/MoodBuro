import appIcon from "../../../../assets/app-icon.png";
import { useState, type FormEvent } from "react";
import {
  Bell,
  CalendarDays,
  Check,
  CircleAlert,
  Globe,
  HardDriveDownload,
  LoaderCircle,
  LockKeyhole,
} from "lucide-react";
import { lastError, type Connection } from "../lib/connection";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";

type StepState = "waiting" | "active" | "complete";
/** One glyph per line of `t.onboarding.features`, in the same order. */
const featureIcons = [Bell, CalendarDays, HardDriveDownload];

function Step({ state, label }: { state: StepState; label: string }) {
  const { t } = useI18n();
  return (
    <li className={`progress-step ${state}`}>
      <span className="progress-mark" aria-hidden="true">
        {state === "complete" ? (
          <Check size={11} strokeWidth={3} />
        ) : state === "active" ? (
          <LoaderCircle className="spin" size={14} strokeWidth={2} />
        ) : null}
      </span>
      <span>
        {label}
        <span className="visually-hidden">
          {state === "complete"
            ? t.onboarding.stepDone
            : state === "active"
              ? t.onboarding.stepActive
              : ""}
        </span>
      </span>
    </li>
  );
}

export function AddSite({
  connection,
  firstRun,
  onSubmit,
  initialUrl = "",
}: {
  initialUrl?: string;
  connection: Connection;
  /** No organisation connected yet: use the welcoming copy. */
  firstRun: boolean;
  onSubmit: (url: string) => void;
}) {
  const { t } = useI18n();
  const [url, setUrl] = useState(initialUrl);
  const adding = connection.phase !== "idle" && connection.siteId === null;
  // A re-authentication from Settings also holds the single sign-in slot.
  const blocked = connection.phase !== "idle" && !adding;
  const error = lastError(connection, null);

  function submit(event: FormEvent) {
    event.preventDefault();
    const value = url.trim();
    if (value) onSubmit(value);
  }

  return (
    <div className="onboarding">
      <div className="app-mark" aria-hidden="true">
        <img src={appIcon} width="88" height="88" alt="" />
      </div>
      <h2 className="onboarding-title">
        {firstRun ? t.onboarding.welcome : t.onboarding.addTitle}
      </h2>
      <p className="onboarding-lead">{t.onboarding.lead}</p>

      <form className="connect-form" onSubmit={submit} noValidate>
        <label className="field-label" htmlFor="site-url">
          {t.onboarding.fieldLabel}
        </label>
        <div className="connect-row">
          <div
            className={`text-field ${error ? "invalid" : ""}`}
            data-disabled={adding || undefined}
          >
            <Globe size={16} strokeWidth={1.75} aria-hidden="true" />
            <input
              id="site-url"
              type="text"
              inputMode="url"
              autoComplete="url"
              autoCapitalize="off"
              autoCorrect="off"
              spellCheck={false}
              // Autofocus belongs to this one primary field on an otherwise empty view.
              autoFocus
              placeholder={t.onboarding.placeholder}
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              disabled={adding}
              aria-invalid={error ? true : undefined}
              aria-describedby={error ? "url-error" : "url-help"}
            />
          </div>
          {!adding && (
            <button
              className="button primary large"
              type="submit"
              disabled={!url.trim() || blocked}
            >
              {t.onboarding.submit}
            </button>
          )}
        </div>
        {error ? (
          <p id="url-error" className="field-message error" role="alert">
            <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
            <ErrorText error={error} context="add" />
          </p>
        ) : (
          <p id="url-help" className="field-message">
            {blocked ? t.onboarding.blocked : t.onboarding.help}
          </p>
        )}

        {adding && (
          <div className="progress" role="status" aria-live="polite">
            <ol>
              <Step
                label={t.onboarding.findSite}
                state={connection.phase === "checking" ? "active" : "complete"}
              />
              <Step
                label={t.onboarding.signIn}
                state={connection.phase === "signing-in" ? "active" : "waiting"}
              />
            </ol>
            {connection.phase === "signing-in" && (
              <p className="progress-hint">{t.onboarding.continueInWindow}</p>
            )}
          </div>
        )}
      </form>

      {firstRun && (
        <ul className="feature-list" role="list">
          {t.onboarding.features.map((feature, index) => {
            const Icon = featureIcons[index] ?? Bell;
            return (
              <li key={feature.title}>
                <span className="row-icon" aria-hidden="true">
                  <Icon size={16} strokeWidth={1.75} />
                </span>
                <span className="row-text">
                  <span className="row-title">{feature.title}</span>
                  <span className="feature-body">{feature.body}</span>
                </span>
              </li>
            );
          })}
        </ul>
      )}

      <p className="privacy-note">
        <LockKeyhole size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>{t.onboarding.privacy}</span>
      </p>
    </div>
  );
}
