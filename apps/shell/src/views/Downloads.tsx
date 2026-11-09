import { useEffect, useEffectEvent, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { desktop, ipc, message, type Download, type Site } from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";

function Confirmation({
  item,
  busy,
  onCancel,
  onOpen,
}: {
  item: Download;
  busy: boolean;
  onCancel: () => void;
  onOpen: () => void;
}) {
  const { t } = useI18n();
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => cancel.current?.focus(), []);
  return (
    <div
      className="confirm"
      role="alertdialog"
      aria-labelledby="open-download-title"
      aria-describedby="open-download-body"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          e.preventDefault();
          if (!busy) onCancel();
        }
      }}
    >
      <p className="confirm-title" id="open-download-title">
        {t.downloads.confirmTitle(item.filename)}
      </p>
      <p className="confirm-body" id="open-download-body">
        {t.downloads.confirmBody}
      </p>
      <div className="confirm-actions">
        <button
          className="button"
          ref={cancel}
          disabled={busy}
          onClick={onCancel}
        >
          {t.downloads.cancel}
        </button>
        <button className="button" disabled={busy} onClick={onOpen}>
          {t.downloads.confirm}
        </button>
      </div>
    </div>
  );
}

export function DownloadList({
  items,
  sites,
  busy,
  confirming,
  onOpen,
  onReveal,
  onCancel,
  onConfirm,
}: {
  items: Download[];
  sites: Site[];
  busy: boolean;
  confirming: string | null;
  onOpen: (item: Download) => void;
  onReveal: (id: string) => void;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="group">
      {items.length === 0 ? (
        <p className="group-row">{t.downloads.empty}</p>
      ) : (
        items.map((item) => (
          <div key={item.id}>
            <div className="group-row">
              <div className="download-info">
                <strong title={item.filename}>{item.filename}</strong>
                <span>
                  {[
                    sites.find((s) => s.id === item.siteId)?.name,
                    t.downloads.states[item.state],
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              </div>
              <div className="download-actions">
                <button
                  className="button small"
                  aria-label={t.downloads.openLabel(item.filename)}
                  disabled={busy || item.state !== "complete"}
                  onClick={() => onOpen(item)}
                >
                  {t.downloads.open}
                </button>
                <button
                  className="button small"
                  aria-label={t.downloads.revealLabel(item.filename)}
                  disabled={busy || item.state !== "complete"}
                  onClick={() => onReveal(item.id)}
                >
                  {t.downloads.reveal}
                </button>
              </div>
            </div>
            {confirming === item.id && (
              <Confirmation
                item={item}
                busy={busy}
                onCancel={onCancel}
                onOpen={onConfirm}
              />
            )}
          </div>
        ))
      )}
    </div>
  );
}
export function Downloads({ sites }: { sites: Site[] }) {
  const { t } = useI18n();
  const [items, setItems] = useState<Download[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState<string | null>(null);
  const loadSequence = useRef(0);
  const returnFocus = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!confirming && !busy && returnFocus.current) {
      returnFocus.current.focus();
      returnFocus.current = null;
    }
  }, [confirming, busy]);
  const load = useEffectEvent(async (cancelled: () => boolean) => {
    const sequence = ++loadSequence.current;
    try {
      const items = await ipc.downloads();
      if (!cancelled() && sequence === loadSequence.current) {
        setItems(items);
        setLoaded(true);
      }
    } catch (cause) {
      if (!cancelled() && sequence === loadSequence.current) {
        setError(message(cause));
        setLoaded(true);
      }
    }
  });
  useEffect(() => {
    let cancelled = false;
    const refresh = () => {
      void load(() => cancelled);
    };
    // Subscribe before taking the initial snapshot, so an in-flight finish is not missed.
    const listener = desktop
      ? listen("downloads://changed", refresh)
      : Promise.resolve(() => {});
    void listener
      .then(() => {
        if (!cancelled) refresh();
      })
      .catch((cause) => {
        if (!cancelled) {
          setError(message(cause));
          setLoaded(true);
        }
      });
    return () => {
      cancelled = true;
      void listener.then((stop) => stop()).catch(() => {});
    };
  }, []);
  async function action(id: string, opening: boolean, confirmed = false) {
    setBusy(true);
    setError(null);
    try {
      if (opening) await ipc.openDownload(id, confirmed);
      else await ipc.revealDownload(id);
      setConfirming(null);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="settings">
      <section>
        <h2 className="section-title">{t.downloads.title}</h2>
        {!loaded ? (
          <p role="status">{t.downloads.loading}</p>
        ) : (
          <DownloadList
            items={items}
            sites={sites}
            busy={busy}
            confirming={confirming}
            onOpen={(item) => {
              if (item.needsConfirmation) {
                returnFocus.current = document.activeElement as HTMLElement;
                setConfirming(item.id);
              } else void action(item.id, true);
            }}
            onReveal={(id) => void action(id, false)}
            onCancel={() => setConfirming(null)}
            onConfirm={() => {
              if (confirming) void action(confirming, true, true);
            }}
          />
        )}
        {error && (
          <p className="section-message error" role="alert">
            <ErrorText error={error} context="navigation" />
          </p>
        )}
        <p className="section-footnote">{t.downloads.location}</p>
      </section>
    </div>
  );
}
