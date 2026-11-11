import { useEffect, useState } from "react";
import { version as appVersion } from "../../package.json";
import { updates, type UpdateStatus } from "../ipc";
import { useI18n } from "../i18n/context";
import { Switch } from "./ui";
export function Updates() {
  const { t } = useI18n();
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let alive = true;
    const refresh = () => {
      void updates
        .status()
        .then((s) => {
          if (alive) setStatus(s);
        })
        .catch(() => {
          if (alive) setFailed(true);
        });
    };
    refresh();
    const timer = window.setInterval(refresh, 1000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);
  async function action(fn: () => Promise<void>) {
    setBusy(true);
    setFailed(false);
    try {
      await fn();
      setStatus(await updates.status());
    } catch {
      setFailed(true);
    } finally {
      setBusy(false);
    }
  }
  const working =
    busy ||
    ["checking", "downloading", "installing"].includes(status?.phase ?? "");
  const phases = t.updates.phases;
  const phase = status?.phase ?? "idle";
  return (
    <section aria-labelledby="updates-heading">
      <h2 id="updates-heading" className="section-title">
        {t.updates.title}
      </h2>
      <div className="group">
        <div className="group-row">
          <span className="row-title">{t.updates.installedVersion}</span>
          <span className="row-subtitle">{appVersion}</span>
        </div>
        <div className="group-row">
          <label htmlFor="automatic-updates" className="row-text">
            <span className="row-title">{t.updates.automatic}</span>
            <span className="row-subtitle">{t.updates.cadence}</span>
          </label>
          <Switch
            id="automatic-updates"
            checked={status?.automatic ?? true}
            disabled={working || !status?.configured}
            onChange={(on) => void action(() => updates.preference(on))}
          />
        </div>
        <div className="group-row">
          <div className="row-text" role="status" aria-live="polite">
            {status?.version && (
              <span className="row-title">
                {t.updates.version(status.version)}
              </span>
            )}
            <span className="row-subtitle">
              {phases[phase as keyof typeof phases] ?? phases.idle}
            </span>
            {phase === "downloading" && (
              <progress
                aria-label={phases.downloading}
                value={status?.total ? status.received : undefined}
                max={status?.total ?? undefined}
              />
            )}
          </div>
          <div className="row-actions">
            {phase === "ready" ? (
              <button
                className="button"
                disabled={working}
                onClick={() => void action(updates.restart)}
              >
                {t.updates.restart}
              </button>
            ) : (
              <>
                <button
                  className="button"
                  disabled={working || !status?.configured}
                  onClick={() => void action(updates.check)}
                >
                  {t.updates.check}
                </button>
                {status?.version &&
                  ["available", "installFailed"].includes(phase) && (
                    <button
                      className="button"
                      disabled={working}
                      onClick={() => void action(updates.install)}
                    >
                      {t.updates.install}
                    </button>
                  )}
              </>
            )}
          </div>
        </div>
        {status?.notes && (
          <div className="group-row">
            <div className="row-text">
              <p className="update-notes">{status.notes}</p>
            </div>
          </div>
        )}
      </div>
      {failed && <p role="alert">{phases.error}</p>}
    </section>
  );
}
