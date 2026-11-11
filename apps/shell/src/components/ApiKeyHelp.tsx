import { useEffect, useRef, useState } from "react";
import { ipc, message, type AssistantProvider } from "../ipc";
import { providerNames } from "../lib/assistant";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";

export function ApiKeyHelp({
  provider,
  onClose,
}: {
  provider: AssistantProvider;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const s = t.assistant.settings;
  const dialog = useRef<HTMLDialogElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const modal = dialog.current;
    modal?.showModal();
    close.current?.focus();
    return () => modal?.close();
  }, []);
  function finish() {
    dialog.current?.close();
    onClose();
  }
  async function open(guide: boolean) {
    try {
      setError(null);
      await ipc.assistantKeyHelp(provider, guide);
    } catch (e) {
      setError(message(e));
    }
  }
  return (
    <dialog
      ref={dialog}
      className="key-help"
      aria-labelledby="key-help-title"
      aria-describedby="key-help-body"
      onCancel={(e) => {
        e.preventDefault();
        e.stopPropagation();
        finish();
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") e.stopPropagation();
      }}
    >
      <h2 id="key-help-title">{s.keyHelpTitle(providerNames[provider])}</h2>
      <p id="key-help-body">
        {provider === "albert" ? s.keyHelpAlbert : s.keyHelpMistral}
      </p>
      <ol>
        {(provider === "albert"
          ? s.keyHelpAlbertSteps
          : s.keyHelpMistralSteps
        ).map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
      {error && (
        <p className="section-message error" role="alert">
          <ErrorText error={error} context="settings" />
        </p>
      )}
      <div className="key-help-links">
        <button className="button" onClick={() => void open(false)}>
          {s.keyHelpPortal}
        </button>
        <button className="button" onClick={() => void open(true)}>
          {s.keyHelpGuide}
        </button>
      </div>
      <div className="confirm-actions">
        <button ref={close} className="button" onClick={finish}>
          {t.toolbar.done}
        </button>
      </div>
    </dialog>
  );
}
