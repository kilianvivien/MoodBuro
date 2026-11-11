import { useEffect, useRef, useState } from "react";
import { ipc, message, type DesktopPreferences as Preferences } from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
export function DesktopPreferences() {
  const { t } = useI18n();
  const [value, setValue] = useState<Preferences | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [finder, setFinder] = useState(false);
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
    <section>
      <h3 className="section-title">{t.desktop.title}</h3>
      <div className="group">
        {value && (
          <>
            <div className="group-row">
              <label className="row-title" htmlFor="menu-bar-only">
                {t.desktop.menuBar}
              </label>
              <input
                id="menu-bar-only"
                type="checkbox"
                checked={value.menuBarOnly}
                disabled={busy}
                onChange={(e) => void change({ menuBarOnly: e.target.checked })}
              />
            </div>
            <div className="group-row">
              <label className="row-title" htmlFor="login-start">
                {t.desktop.login}
              </label>
              <input
                id="login-start"
                type="checkbox"
                checked={value.startAtLogin}
                disabled={busy}
                onChange={(e) =>
                  void change({ startAtLogin: e.target.checked })
                }
              />
            </div>
          </>
        )}
        <div className="group-row">
          <span className="row-title">{t.desktop.finder}</span>
          <button
            className="button"
            onClick={() => {
              void ipc
                .finderMoodleFolder()
                .then(() => setFinder(true))
                .catch((e) => setError(message(e)));
            }}
          >
            {t.desktop.showFolder}
          </button>
        </div>
      </div>
      {finder && (
        <p className="section-footnote" role="status">
          {t.desktop.finderHint}
        </p>
      )}
      {error && (
        <p className="section-message error" role="alert">
          <ErrorText error={error} context="settings" />
        </p>
      )}
    </section>
  );
}
