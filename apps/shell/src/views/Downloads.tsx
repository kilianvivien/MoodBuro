import { useEffect, useEffectEvent, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  CircleAlert,
  CircleCheck,
  Copy,
  Download as DownloadIcon,
  FileText,
  FolderOpen,
  LoaderCircle,
  SearchX,
  TriangleAlert,
} from "lucide-react";
import {
  desktop,
  DOWNLOAD_PAGE_SIZE,
  ipc,
  message,
  type Download,
  type DownloadState,
  type Site,
} from "../ipc";
import { useI18n } from "../i18n/context";
import { ErrorText } from "../i18n/ErrorText";
import {
  EmptyState,
  PageHeader,
  SearchField,
  Segmented,
  SiteSelect,
  Skeleton,
} from "../components/ui";

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

const stateIcons = {
  downloading: LoaderCircle,
  complete: FileText,
  failed: TriangleAlert,
} as const;

export function DownloadList({
  items,
  sites,
  busy,
  confirming,
  filtered = false,
  onOpen,
  onReveal,
  onCopy,
  onCancel,
  onConfirm,
}: {
  items: Download[];
  sites: Site[];
  busy: boolean;
  confirming: string | null;
  /** A search or filter is active, so "nothing here" means "no match". */
  filtered?: boolean;
  onOpen: (item: Download) => void;
  onReveal: (id: string) => void;
  onCopy?: (id: string) => void;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t, date } = useI18n();
  if (items.length === 0)
    return filtered ? (
      <EmptyState
        icon={<SearchX size={22} strokeWidth={1.75} />}
        title={t.downloads.noMatch}
        hint={t.downloads.noMatchHint}
      />
    ) : (
      <EmptyState
        icon={<DownloadIcon size={22} strokeWidth={1.75} />}
        title={t.downloads.empty}
        hint={t.downloads.emptyHint}
      />
    );
  return (
    <ul className="group" role="list">
      {items.map((item) => {
        const Icon = stateIcons[item.state];
        const unavailable = busy || item.state !== "complete";
        return (
          <li key={item.id}>
            <div className="list-row" data-state={item.state}>
              <span className="row-icon" aria-hidden="true">
                <Icon
                  className={item.state === "downloading" ? "spin" : undefined}
                  size={16}
                  strokeWidth={1.75}
                />
              </span>
              <div className="row-text">
                <strong className="row-title" title={item.filename}>
                  {item.filename}
                </strong>
                <span className="row-subtitle">
                  {[
                    sites.find((s) => s.id === item.siteId)?.name,
                    t.downloads.states[item.state],
                    date(item.startedAt * 1000, {
                      dateStyle: "medium",
                      timeStyle: "short",
                    }),
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              </div>
              <div className="row-actions">
                <button
                  className="button small"
                  aria-label={t.downloads.openLabel(item.filename)}
                  disabled={unavailable}
                  onClick={() => onOpen(item)}
                >
                  {t.downloads.open}
                </button>
                <button
                  className="button small icon"
                  title={t.downloads.reveal}
                  aria-label={t.downloads.revealLabel(item.filename)}
                  disabled={unavailable}
                  onClick={() => onReveal(item.id)}
                >
                  <FolderOpen size={13} strokeWidth={2} aria-hidden="true" />
                </button>
                {onCopy && (
                  <button
                    className="button small icon"
                    title={t.downloads.copyPath}
                    aria-label={t.downloads.copyPathLabel(item.filename)}
                    disabled={unavailable}
                    onClick={() => onCopy(item.id)}
                  >
                    <Copy size={13} strokeWidth={2} aria-hidden="true" />
                  </button>
                )}
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
          </li>
        );
      })}
    </ul>
  );
}

function ClearConfirmation({
  site,
  busy,
  onCancel,
  onConfirm,
}: {
  site: Site | undefined;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => cancel.current?.focus(), []);
  return (
    <div
      className="confirm standalone"
      role="alertdialog"
      aria-labelledby="clear-downloads-title"
      aria-describedby="clear-downloads-body"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          e.preventDefault();
          if (!busy) onCancel();
        }
      }}
    >
      <p className="confirm-title" id="clear-downloads-title">
        {site ? t.downloads.clearTitleFor(site.name) : t.downloads.clearTitle}
      </p>
      <p className="confirm-body" id="clear-downloads-body">
        {t.downloads.clearBody}
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
        <button
          className="button destructive"
          disabled={busy}
          onClick={onConfirm}
        >
          {t.downloads.confirmClear}
        </button>
      </div>
    </div>
  );
}

type StateFilter = "all" | DownloadState;

export function Downloads({ sites }: { sites: Site[] }) {
  const { t } = useI18n();
  const [items, setItems] = useState<Download[]>([]);
  const [total, setTotal] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const [query, setQuery] = useState("");
  const [siteId, setSiteId] = useState<string | null>(null);
  const [state, setState] = useState<StateFilter>("all");
  const [pages, setPages] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [clearing, setClearing] = useState(false);
  const loadSequence = useRef(0);
  const returnFocus = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!confirming && !clearing && !busy && returnFocus.current) {
      returnFocus.current.focus();
      returnFocus.current = null;
    }
  }, [confirming, clearing, busy]);
  const load = useEffectEvent(async (cancelled: () => boolean) => {
    const sequence = ++loadSequence.current;
    const current = () => !cancelled() && sequence === loadSequence.current;
    try {
      const next: Download[] = [];
      let count = 0;
      // Every page shown so far is reloaded, so a finished transfer updates in place.
      for (let page = 0; page < pages; page += 1) {
        const result = await ipc.downloads({
          query,
          siteId,
          state: state === "all" ? null : state,
          offset: page * DOWNLOAD_PAGE_SIZE,
        });
        next.push(...result.items);
        count = result.total;
        if (result.items.length < DOWNLOAD_PAGE_SIZE) break;
      }
      if (current()) {
        setItems(next);
        setTotal(count);
        setLoaded(true);
      }
    } catch (cause) {
      if (current()) {
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
  }, [query, siteId, state, pages]);
  async function action(run: () => Promise<void>, done: string | null = null) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await run();
      setConfirming(null);
      setClearing(false);
      setNotice(done);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }
  const filter =
    <T,>(set: (value: T) => void) =>
    (value: T) => {
      setPages(1);
      setNotice(null);
      set(value);
    };
  const filtered = query.trim() !== "" || siteId !== null || state !== "all";
  return (
    <div className="page">
      <PageHeader
        title={t.downloads.title}
        detail={
          loaded && total > items.length
            ? t.downloads.shown(items.length, total)
            : undefined
        }
      >
        <button
          className="button"
          disabled={busy || clearing || !loaded || (total === 0 && !filtered)}
          onClick={(event) => {
            returnFocus.current = event.currentTarget;
            setNotice(null);
            setClearing(true);
          }}
        >
          {t.downloads.clear}
        </button>
      </PageHeader>
      <div className="filter-bar">
        <SearchField
          value={query}
          label={t.downloads.searchLabel}
          placeholder={t.downloads.searchPlaceholder}
          onChange={filter(setQuery)}
        />
        <SiteSelect sites={sites} value={siteId} onChange={filter(setSiteId)} />
        <Segmented<StateFilter>
          name="download-state"
          legend={t.downloads.stateFilter}
          value={state}
          options={[
            { value: "all", label: t.downloads.allStates },
            { value: "downloading", label: t.downloads.filters.downloading },
            { value: "complete", label: t.downloads.filters.complete },
            { value: "failed", label: t.downloads.filters.failed },
          ]}
          onChange={filter(setState)}
        />
      </div>
      {clearing && (
        <ClearConfirmation
          site={sites.find((site) => site.id === siteId)}
          busy={busy}
          onCancel={() => setClearing(false)}
          onConfirm={() =>
            void action(() => ipc.clearDownloads(siteId), t.downloads.cleared)
          }
        />
      )}
      {error && (
        <p className="section-message error" role="alert">
          <CircleAlert size={14} strokeWidth={2} aria-hidden="true" />
          <ErrorText error={error} context="navigation" />
        </p>
      )}
      {notice && (
        <p className="section-message success" role="status">
          <CircleCheck size={14} strokeWidth={2} aria-hidden="true" />
          {notice}
        </p>
      )}
      {!loaded ? (
        <Skeleton />
      ) : (
        <DownloadList
          items={items}
          sites={sites}
          busy={busy}
          confirming={confirming}
          filtered={filtered}
          onOpen={(item) => {
            if (item.needsConfirmation) {
              returnFocus.current = document.activeElement as HTMLElement;
              setConfirming(item.id);
            } else void action(() => ipc.openDownload(item.id));
          }}
          onReveal={(id) => void action(() => ipc.revealDownload(id))}
          onCopy={(id) =>
            void action(() => ipc.copyDownloadPath(id), t.downloads.copied)
          }
          onCancel={() => setConfirming(null)}
          onConfirm={() => {
            if (confirming)
              void action(() => ipc.openDownload(confirming, true));
          }}
        />
      )}
      {loaded && total > items.length && (
        <div className="list-more">
          <button className="button" onClick={() => setPages((n) => n + 1)}>
            {t.downloads.showMore}
          </button>
        </div>
      )}
      <p className="section-footnote">{t.downloads.location}</p>
    </div>
  );
}
