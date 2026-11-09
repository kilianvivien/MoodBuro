import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ArrowLeft, ArrowRight, House, Download, RotateCw } from "lucide-react";
import { ipc, message, type PopupContext, type NavAction } from "../ipc";
import { I18nProvider } from "../i18n/context";
import { catalogues } from "../i18n/messages";
import { resolveLocale, systemLanguages } from "../i18n/locale";
import { describeError } from "../i18n/errors";

export function PopupBar() {
  const [context, setContext] = useState<PopupContext | null>(null);
  const [error, setError] = useState<string | null>(null);
  const locale = resolveLocale(
    context?.language ?? "system",
    systemLanguages(),
  );
  const t = catalogues[locale];
  useEffect(() => {
    document.documentElement.lang = locale;
    if (context?.appearance && context.appearance !== "system")
      document.documentElement.dataset.theme = context.appearance;
    else delete document.documentElement.dataset.theme;
  }, [context?.appearance, locale]);
  useEffect(() => {
    let cancelled = false;
    let sequence = 0;
    const refresh = async () => {
      const request = ++sequence;
      try {
        const value = await ipc.popupContext();
        if (!cancelled && request === sequence) setContext(value);
      } catch (cause) {
        if (!cancelled) setError(message(cause));
      }
    };
    const listener = listen("navigation://changed", () => {
      void refresh();
    });
    void listener
      .then(() => refresh())
      .catch((cause) => {
        if (!cancelled) setError(message(cause));
      });
    return () => {
      cancelled = true;
      void listener.then((stop) => stop()).catch(() => {});
    };
  }, []);
  async function navigate(action: NavAction | "downloads") {
    setError(null);
    try {
      await ipc.popupNav(action);
    } catch (cause) {
      setError(message(cause));
    }
  }
  const icon = { size: 16, strokeWidth: 1.75, "aria-hidden": true } as const;
  const back = context?.canGoBack ? t.toolbar.back : t.toolbar.backToMoodle;
  return (
    <I18nProvider locale={locale}>
      <header className="popup-bar" aria-label={t.toolbar.navigation}>
        <button
          className="tool"
          title={back}
          aria-label={back}
          onClick={() => void navigate("back")}
        >
          <ArrowLeft {...icon} />
        </button>
        <button
          className="tool"
          title={t.toolbar.forward}
          aria-label={t.toolbar.forward}
          disabled={!context?.canGoForward}
          onClick={() => void navigate("forward")}
        >
          <ArrowRight {...icon} />
        </button>
        <button
          className="tool"
          title={t.toolbar.home}
          aria-label={t.toolbar.home}
          onClick={() => void navigate("home")}
        >
          <House {...icon} />
        </button>
        <button
          className="tool"
          title={t.toolbar.reload}
          aria-label={t.toolbar.reload}
          onClick={() => void navigate("reload")}
        >
          <RotateCw {...icon} />
        </button>
        <h1 title={context?.siteName}>
          {context?.siteName ?? t.toolbar.popup}
        </h1>
        {error && (
          <span className="popup-error" role="alert" title={error}>
            {describeError(locale, error, "navigation").text}
          </span>
        )}
        <button
          className="tool"
          title={t.toolbar.downloads}
          aria-label={t.toolbar.downloads}
          onClick={() => void navigate("downloads")}
        >
          <Download {...icon} />
        </button>
      </header>
    </I18nProvider>
  );
}
