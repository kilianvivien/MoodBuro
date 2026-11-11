import { useEffect, useRef, useState } from "react";
import {
  ChevronsUpDown,
  CircleHelp,
  CircleAlert,
  CircleCheck,
  KeyRound,
  LoaderCircle,
} from "lucide-react";
import { ApiKeyHelp } from "../components/ApiKeyHelp";
import { Segmented } from "../components/ui";
import { ipc, message, type AssistantProvider } from "../ipc";
import { useAssistant } from "../stores/assistant";
import { providerNames, providers } from "../lib/assistant";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";

type Asking = "key" | "history" | null;
/** Outcome of the last action in this section; errors stay raw until rendered. */
type Outcome = { error: string } | { success: string } | null;

export interface AssistantSettingsViewProps {
  provider: AssistantProvider;
  model: string;
  hasKey: boolean;
  consent: boolean;
  /** Models the provider listed after a successful connection check. */
  models: readonly string[] | null;
  busy: boolean;
  checking: boolean;
  asking: Asking;
  outcome: Outcome;
  keyText: string;
  onKeyText: (text: string) => void;
  onProvider: (provider: AssistantProvider) => void;
  onSaveKey: () => void;
  onRemoveKey: () => void;
  onModel: (model: string) => void;
  onCheck: () => void;
  onConsent: (consent: boolean) => void;
  onClearAll: () => void;
  onAsk: (asking: Asking) => void;
}

export function AssistantSettingsView(props: AssistantSettingsViewProps) {
  const { t } = useI18n();
  const s = t.assistant.settings;
  const name = providerNames[props.provider];
  const cancel = useRef<HTMLButtonElement>(null);
  const helpButton = useRef<HTMLButtonElement>(null);
  const [help, setHelp] = useState(false);
  useEffect(() => {
    if (props.asking) cancel.current?.focus();
  }, [props.asking]);
  const choices = props.models ?? [];
  const selected = choices.includes(props.model) ? props.model : "";
  const confirm = (
    title: string,
    body: string,
    action: string,
    onConfirm: () => void,
  ) => (
    <div
      className="confirm"
      role="alertdialog"
      aria-labelledby="assistant-ask-title"
      aria-describedby="assistant-ask-body"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          // Close only the confirmation, not Settings itself.
          event.preventDefault();
          event.stopPropagation();
          props.onAsk(null);
        }
      }}
    >
      <p id="assistant-ask-title" className="confirm-title">
        {title}
      </p>
      <p id="assistant-ask-body" className="confirm-body">
        {body}
      </p>
      <div className="confirm-actions">
        <button
          ref={cancel}
          className="button"
          onClick={() => props.onAsk(null)}
        >
          {t.assistant.cancel}
        </button>
        <button className="button destructive" onClick={onConfirm}>
          {action}
        </button>
      </div>
    </div>
  );
  return (
    <section aria-labelledby="assistant-heading">
      <h2 id="assistant-heading" className="section-title">
        {s.title}
      </h2>
      <div className="group">
        <div className="group-row">
          <div className="row-text">
            <span className="row-title">{s.provider}</span>
            <span className="row-subtitle">{s.providerHint}</span>
          </div>
          <Segmented
            name="assistant-provider"
            legend={s.provider}
            value={props.provider}
            disabled={props.busy}
            onChange={props.onProvider}
            options={providers.map((value) => ({
              value,
              label: providerNames[value],
            }))}
          />
        </div>
        <div className="group-row">
          <div className="row-text">
            <div className="row-title assistant-key-title">
              <label htmlFor="assistant-key">{s.key}</label>
              <button
                ref={helpButton}
                className="button small icon"
                title={s.keyHelp}
                aria-label={s.keyHelp}
                disabled={props.busy}
                onClick={() => setHelp(true)}
              >
                <CircleHelp size={13} strokeWidth={1.75} aria-hidden="true" />
              </button>
            </div>
            <span className="row-subtitle" id="assistant-key-state">
              {props.hasKey ? s.keySaved : s.keyMissing}
            </span>
          </div>
          <form
            className="row-actions"
            onSubmit={(event) => {
              event.preventDefault();
              props.onSaveKey();
            }}
          >
            <span className="text-field compact assistant-key">
              <KeyRound size={12} strokeWidth={2} aria-hidden="true" />
              {/* A password field: never shown, never offered back by autofill. */}
              <input
                id="assistant-key"
                type="password"
                autoComplete="off"
                spellCheck={false}
                value={props.keyText}
                placeholder={s.keyPlaceholder}
                aria-describedby="assistant-key-state"
                disabled={props.busy}
                onChange={(event) => props.onKeyText(event.target.value)}
              />
            </span>
            <button
              type="submit"
              className="button small"
              disabled={props.busy || props.keyText.trim() === ""}
            >
              {s.saveKey}
            </button>
            {props.hasKey && (
              <button
                type="button"
                className="button small"
                disabled={props.busy}
                onClick={() => props.onAsk("key")}
              >
                {s.removeKey}
              </button>
            )}
          </form>
        </div>
        {props.asking === "key" &&
          confirm(
            s.removeKeyTitle(name),
            s.removeKeyBody,
            s.removeKeyConfirm,
            props.onRemoveKey,
          )}
        <div className="group-row">
          <div className="row-text">
            <span className="row-title">{s.model}</span>
            <span className="row-subtitle">{s.modelHint}</span>
          </div>
          <div className="row-actions">
            <label className="popup-button assistant-model">
              <span className="visually-hidden">{s.modelLabel}</span>
              <select
                value={selected}
                disabled={props.busy || props.checking || choices.length === 0}
                onChange={(event) => props.onModel(event.target.value)}
              >
                {!selected && (
                  <option value="" disabled>
                    {props.checking ? s.checking : s.chooseModel}
                  </option>
                )}
                {choices.map((model) => (
                  <option key={model} value={model}>
                    {model}
                  </option>
                ))}
              </select>
              <ChevronsUpDown size={12} strokeWidth={2} aria-hidden="true" />
            </label>
            <button
              className="button small"
              disabled={props.busy || props.checking || !props.hasKey}
              onClick={props.onCheck}
            >
              {props.checking && (
                <LoaderCircle
                  className="spin"
                  size={12}
                  strokeWidth={2}
                  aria-hidden="true"
                />
              )}
              {props.checking ? s.checking : s.check}
            </button>
          </div>
        </div>
        <div className="group-row">
          <div className="row-text">
            <span className="row-title">{s.consent}</span>
            <span className="row-subtitle">{s.consentSummary}</span>
            <details className="help-disclosure">
              <summary>{s.consentDetails}</summary>
              <p>{s.consentHint(name)}</p>
            </details>
          </div>
          <Segmented
            name="assistant-consent"
            legend={s.consent}
            value={props.consent ? "on" : "off"}
            disabled={props.busy}
            onChange={(value) => props.onConsent(value === "on")}
            options={(["off", "on"] as const).map((value) => ({
              value,
              label: t.settings.shortcutOptions[value],
            }))}
          />
        </div>
        <div className="group-row">
          <div className="row-text">
            <span className="row-title">{s.history}</span>
            <span className="row-subtitle">{s.historyHint}</span>
          </div>
          <button
            className="button small"
            disabled={props.busy}
            onClick={() => props.onAsk("history")}
          >
            {s.clearAll}
          </button>
        </div>
        {props.asking === "history" &&
          confirm(
            s.clearAllTitle,
            s.clearAllBody,
            s.clearAllConfirm,
            props.onClearAll,
          )}
      </div>
      {props.outcome &&
        ("error" in props.outcome ? (
          <p className="section-message error" role="alert">
            <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
            <ErrorText error={props.outcome.error} context="settings" />
          </p>
        ) : (
          <p className="section-message success" role="status">
            <CircleCheck size={14} strokeWidth={2} aria-hidden="true" />
            {props.outcome.success}
          </p>
        ))}
      {help && (
        <ApiKeyHelp
          provider={props.provider}
          onClose={() => {
            setHelp(false);
            helpButton.current?.focus();
          }}
        />
      )}
      <details className="help-disclosure section-footnote">
        <summary>{t.common.details}</summary>
        <p>{s.intro}</p>
        <p>{s.footnote}</p>
      </details>
    </section>
  );
}

/** Provider, key, model, context sharing and history controls in Settings. */
export function AssistantSettings() {
  const { t } = useI18n();
  const assistant = useAssistant();
  const [models, setModels] = useState<string[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [checking, setChecking] = useState(false);
  const [asking, setAsking] = useState<Asking>(null);
  const [outcome, setOutcome] = useState<Outcome>(null);
  const [keyText, setKeyText] = useState("");
  const [modelRevision, setModelRevision] = useState(0);
  const pending = useRef(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  /** One action at a time; the outcome is shown under the group. */
  async function act(action: () => Promise<string | void>) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setAsking(null);
    setOutcome(null);
    try {
      const success = await action();
      if (alive.current && success) setOutcome({ success });
    } catch (cause) {
      if (alive.current) setOutcome({ error: message(cause) });
    } finally {
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  useEffect(() => {
    let current = true;
    setModels(null);
    setOutcome(null);
    if (!assistant.loaded || !assistant.hasKey) {
      setChecking(false);
      return;
    }
    setChecking(true);
    void ipc
      .assistantModels()
      .then((found) => {
        if (current) setModels(found);
      })
      .catch((cause) => {
        if (current) setOutcome({ error: message(cause) });
      })
      .finally(() => {
        if (current) setChecking(false);
      });
    return () => {
      current = false;
    };
  }, [assistant.provider, assistant.hasKey, assistant.loaded, modelRevision]);
  return (
    <AssistantSettingsView
      provider={assistant.provider}
      model={assistant.model}
      hasKey={assistant.hasKey}
      consent={assistant.consent}
      models={models}
      busy={busy}
      checking={checking}
      asking={asking}
      outcome={outcome}
      keyText={keyText}
      onKeyText={setKeyText}
      onAsk={setAsking}
      onProvider={(provider) =>
        void act(async () => {
          // Models and the pasted key belong to the provider they were entered for.
          setModels(null);
          setKeyText("");
          await assistant.configure({ provider });
        })
      }
      onSaveKey={() =>
        void act(async () => {
          assistant.apply(
            await ipc.setAssistantKey(assistant.provider, keyText),
          );
          setKeyText("");
          // A saved key is only useful if the provider accepts it: say so now.
          setModelRevision((n) => n + 1);
        })
      }
      onRemoveKey={() =>
        void act(async () => {
          assistant.apply(await ipc.removeAssistantKey(assistant.provider));
          setModels(null);
        })
      }
      onModel={(model) =>
        void act(async () => {
          if (models?.includes(model)) await assistant.configure({ model });
        })
      }
      onCheck={() => setModelRevision((n) => n + 1)}
      onConsent={(consent) => void act(() => assistant.configure({ consent }))}
      onClearAll={() =>
        void act(async () => {
          await ipc.clearAssistant(null, null, true);
          return t.assistant.settings.cleared;
        })
      }
    />
  );
}
