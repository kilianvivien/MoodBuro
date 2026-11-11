import { useEffect, useRef, useState } from "react";
import { ipc, message, type DesktopPreferences as Preferences } from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import { Notice, Switch } from "./ui";
export function DesktopPreferences() {
  const { t } = useI18n();
  const [value, setValue] = useState<Preferences | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pending = useRef(false);
  useEffect(() => {
    let alive = true;
    void ipc
      .desktopPreferences()
      .then((v) => {
        if (alive) setValue(v);
      })
      .catch((e) => {
        if (alive) setError(message(e));
      });
    return () => {
      alive = false;
    };
  }, []);
  async function change(change: Partial<Preferences>) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    try {
      setValue(await ipc.desktopPreferencesSet(change));
    } catch (e) {
      setError(message(e));
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  return (
    <section aria-labelledby="desktop-heading">
      <h2 id="desktop-heading" className="section-title">
        {t.desktop.title}
      </h2>
      <div className="group">
        {value && (
          <>
            <div className="group-row">
              <label className="row-text" htmlFor="menu-bar-only">
                <span className="row-title">{t.desktop.menuBar}</span>
              </label>
              <Switch
                id="menu-bar-only"
                checked={value.menuBarOnly}
                disabled={busy}
                onChange={(menuBarOnly) => void change({ menuBarOnly })}
              />
            </div>
            <div className="group-row">
              <label className="row-text" htmlFor="login-start">
                <span className="row-title">{t.desktop.login}</span>
              </label>
              <Switch
                id="login-start"
                checked={value.startAtLogin}
                disabled={busy}
                onChange={(startAtLogin) => void change({ startAtLogin })}
              />
            </div>
          </>
        )}
        <div className="group-row">
          <div className="row-text">
            <span className="row-title">{t.desktop.finder}</span>
            <span className="row-subtitle">{t.desktop.finderHint}</span>
          </div>
          <button
            className="button"
            onClick={() => {
              void ipc.finderMoodleFolder().catch((e) => setError(message(e)));
            }}
          >
            {t.desktop.showFolder}
          </button>
        </div>
      </div>
      {error && (
        <Notice tone="error">
          <ErrorText error={error} context="settings" />
        </Notice>
      )}
    </section>
  );
}
