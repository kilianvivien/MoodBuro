import { useEffect, useRef, useState } from "react";
import {
  ChevronsUpDown,
  CircleAlert,
  CircleCheck,
  KeyRound,
  LoaderCircle,
} from "lucide-react";
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
  modelText: string;
  onKeyText: (text: string) => void;
  onModelText: (text: string) => void;
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
  useEffect(() => {
    if (props.asking) cancel.current?.focus();
  }, [props.asking]);
  // The saved model stays selectable even when the provider no longer lists it.
  const choices =
    props.models && !props.models.includes(props.model)
      ? [props.model, ...props.models]
      : props.models;
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
            <label className="row-title" htmlFor="assistant-key">
              {s.key}
            </label>
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
            {choices ? (
              <label className="popup-button">
                <span className="visually-hidden">{s.modelLabel}</span>
                <select
                  value={props.model}
                  disabled={props.busy}
                  onChange={(event) => props.onModel(event.target.value)}
                >
                  {choices.map((model) => (
                    <option key={model} value={model}>
                      {model}
                    </option>
                  ))}
                </select>
                <ChevronsUpDown size={12} strokeWidth={2} aria-hidden="true" />
              </label>
            ) : (
              <form
                className="row-actions"
                onSubmit={(event) => {
                  event.preventDefault();
                  props.onModel(props.modelText);
                }}
              >
                <span className="text-field compact assistant-model">
                  <input
                    value={props.modelText}
                    spellCheck={false}
                    autoComplete="off"
                    aria-label={s.modelLabel}
                    disabled={props.busy}
                    onChange={(event) => props.onModelText(event.target.value)}
                  />
                </span>
                <button
                  type="submit"
                  className="button small"
                  disabled={
                    props.busy ||
                    props.modelText.trim() === "" ||
                    props.modelText.trim() === props.model
                  }
                >
                  {s.saveModel}
                </button>
              </form>
            )}
            <button
              className="button small"
              disabled={props.busy || !props.hasKey}
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
            <span className="row-subtitle">{s.consentHint(name)}</span>
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
      <p className="section-footnote">
        {s.intro} {s.footnote}
      </p>
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
  const [modelDraft, setModelDraft] = useState<string | null>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  /** One action at a time; the outcome is shown under the group. */
  async function act(action: () => Promise<string | void>) {
    if (busy) return;
    setBusy(true);
    setAsking(null);
    setOutcome(null);
    try {
      const success = await action();
      if (alive.current && success) setOutcome({ success });
    } catch (cause) {
      if (alive.current) setOutcome({ error: message(cause) });
    } finally {
      if (alive.current) setBusy(false);
    }
  }
  const check = async () => {
    setChecking(true);
    try {
      const found = await ipc.assistantModels();
      if (!alive.current) return;
      setModels(found);
      setModelDraft(null);
      return t.assistant.settings.connected(found.length);
    } finally {
      if (alive.current) setChecking(false);
    }
  };
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
      modelText={modelDraft ?? assistant.model}
      onKeyText={setKeyText}
      onModelText={setModelDraft}
      onAsk={setAsking}
      onProvider={(provider) =>
        void act(async () => {
          // Models and the pasted key belong to the provider they were entered for.
          setModels(null);
          setModelDraft(null);
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
          return check();
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
          await assistant.configure({ model: model.trim() });
          setModelDraft(null);
        })
      }
      onCheck={() => void act(check)}
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
