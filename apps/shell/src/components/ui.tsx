import {
  useEffect,
  useLayoutEffect,
  useRef,
  type KeyboardEvent,
  type ReactNode,
  type Ref,
} from "react";
import {
  ChevronsUpDown,
  CircleAlert,
  CircleCheck,
  File,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileSpreadsheet,
  FileText,
  FileVideo,
  Info,
  LoaderCircle,
  Presentation,
  RotateCw,
  Search,
  X,
} from "lucide-react";
import type { Site } from "../ipc";
import { useI18n } from "../i18n/context";
import { ago } from "../lib/time";

/**
 * 2–4 mutually exclusive options that apply immediately. Native radios give
 * arrow-key selection and one tab stop; the thumb slides between segments.
 */
export function Segmented<T extends string>({
  name,
  legend,
  options,
  value,
  disabled = false,
  onChange,
}: {
  name: string;
  legend: string;
  options: readonly { value: T; label: ReactNode; lang?: string }[];
  value: T;
  disabled?: boolean;
  onChange: (value: T) => void;
}) {
  const root = useRef<HTMLFieldSetElement>(null);
  // Measure the selected segment so the thumb can slide to it. Re-measured on
  // every render: labels change width with the language.
  useLayoutEffect(() => {
    const field = root.current;
    // Not `:has()`: WebKit before macOS 12.3 throws on it in querySelector.
    const selected = field
      ?.querySelector("input:checked")
      ?.closest<HTMLElement>(".segment");
    if (!field || !selected || selected.offsetWidth === 0) return;
    field.style.setProperty("--thumb-x", `${selected.offsetLeft}px`);
    field.style.setProperty("--thumb-width", `${selected.offsetWidth}px`);
    field.dataset.thumb = "";
  });
  return (
    <fieldset className="segmented" disabled={disabled} ref={root}>
      <legend className="visually-hidden">{legend}</legend>
      <span className="segmented-thumb" aria-hidden="true" />
      {options.map((option) => (
        <label key={option.value} className="segment">
          <input
            type="radio"
            name={name}
            value={option.value}
            checked={value === option.value}
            onChange={() => onChange(option.value)}
          />
          <span lang={option.lang}>{option.label}</span>
        </label>
      ))}
    </fieldset>
  );
}

/** Search field with a clear button; Escape clears before it leaves the view. */
export function SearchField({
  value,
  label,
  placeholder,
  inputRef,
  onChange,
  onKeyDown,
  ...input
}: {
  value: string;
  label: string;
  placeholder: string;
  inputRef?: Ref<HTMLInputElement>;
  onChange: (value: string) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLInputElement>) => void;
  role?: string;
  "aria-controls"?: string;
  "aria-expanded"?: boolean;
  "aria-activedescendant"?: string;
  "aria-autocomplete"?: "list";
}) {
  const { t } = useI18n();
  return (
    <div className="text-field search-field">
      <Search size={14} strokeWidth={2} aria-hidden="true" />
      <input
        ref={inputRef}
        type="text"
        inputMode="search"
        value={value}
        aria-label={label}
        placeholder={placeholder}
        autoComplete="off"
        autoCorrect="off"
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          onKeyDown?.(event);
          if (event.key === "Escape" && value && !event.defaultPrevented) {
            event.preventDefault();
            event.stopPropagation();
            onChange("");
          }
        }}
        {...input}
      />
      {value && (
        <button
          type="button"
          className="tool small"
          title={t.toolbar.dismiss}
          aria-label={t.toolbar.dismiss}
          onClick={() => onChange("")}
        >
          <X size={12} strokeWidth={2} aria-hidden="true" />
        </button>
      )}
    </div>
  );
}

/**
 * Pop-up button: a native select in a push-button bezel, so the menu is the
 * real macOS one. `label` names it for VoiceOver unless a visible label is
 * linked through `id`.
 */
export function PopupButton({
  id,
  label,
  value,
  disabled = false,
  className,
  onChange,
  children,
}: {
  id?: string;
  label?: string;
  value: string | number;
  disabled?: boolean;
  className?: string;
  onChange: (value: string) => void;
  children: ReactNode;
}) {
  return (
    <span
      className={className ? `popup-button ${className}` : "popup-button"}
      data-disabled={disabled || undefined}
    >
      <select
        id={id}
        aria-label={label}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      >
        {children}
      </select>
      <ChevronsUpDown size={12} strokeWidth={2} aria-hidden="true" />
    </span>
  );
}

/** Organisation filter; hidden while there is only one organisation. */
export function SiteSelect({
  sites,
  value,
  onChange,
}: {
  sites: readonly Site[];
  value: string | null;
  onChange: (value: string | null) => void;
}) {
  const { t } = useI18n();
  if (sites.length < 2) return null;
  return (
    <PopupButton
      label={t.common.siteFilter}
      value={value ?? ""}
      onChange={(next) => onChange(next || null)}
    >
      <option value="">{t.common.allSites}</option>
      {sites.map((site) => (
        <option key={site.id} value={site.id}>
          {site.name}
        </option>
      ))}
    </PopupButton>
  );
}

/** On/off setting that applies immediately: a native checkbox drawn as a switch. */
export function Switch({
  id,
  label,
  checked,
  disabled = false,
  describedBy,
  onChange,
}: {
  id?: string;
  /** Accessible name when no visible label is linked through `id`. */
  label?: string;
  checked: boolean;
  disabled?: boolean;
  describedBy?: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <input
      id={id}
      className="switch"
      type="checkbox"
      role="switch"
      aria-label={label}
      aria-describedby={describedBy}
      checked={checked}
      disabled={disabled}
      onChange={(event) => onChange(event.target.checked)}
    />
  );
}

/** Outcome or state of an action, under the group it concerns. */
export function Notice({
  tone = "info",
  children,
}: {
  tone?: "info" | "success" | "error";
  children: ReactNode;
}) {
  const Icon =
    tone === "error" ? CircleAlert : tone === "success" ? CircleCheck : Info;
  return (
    <p
      className={`section-message ${tone}`}
      role={tone === "error" ? "alert" : "status"}
    >
      <Icon size={14} strokeWidth={2} aria-hidden="true" />
      <span className="error-text">{children}</span>
    </p>
  );
}

/**
 * Inline confirmation. Focus starts on Cancel; Escape cancels only the
 * confirmation, never the view around it.
 */
export function Confirm({
  id,
  title,
  body,
  cancel,
  confirm,
  tone = "default",
  nested = false,
  busy = false,
  onCancel,
  onConfirm,
  children,
}: {
  /** Prefix for the title/body ids; unique among confirmations on screen. */
  id: string;
  title?: string;
  body: ReactNode;
  cancel: ReactNode;
  confirm: ReactNode;
  tone?: "default" | "primary" | "destructive";
  /** Under a list row, aligned with its text column. */
  nested?: boolean;
  busy?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
  /** Further body paragraphs. */
  children?: ReactNode;
}) {
  const cancelButton = useRef<HTMLButtonElement>(null);
  useEffect(() => cancelButton.current?.focus(), []);
  return (
    <div
      className={nested ? "confirm" : "confirm standalone"}
      role="alertdialog"
      aria-labelledby={title ? `${id}-title` : `${id}-body`}
      aria-describedby={`${id}-body`}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          if (!busy) onCancel();
        }
      }}
    >
      {title && (
        <p className="confirm-title" id={`${id}-title`}>
          {title}
        </p>
      )}
      <p className="confirm-body" id={`${id}-body`}>
        {body}
      </p>
      {children}
      <div className="confirm-actions">
        <button
          className="button"
          ref={cancelButton}
          disabled={busy}
          onClick={onCancel}
        >
          {cancel}
        </button>
        <button
          className={tone === "default" ? "button" : `button ${tone}`}
          disabled={busy}
          onClick={onConfirm}
        >
          {confirm}
        </button>
      </div>
    </div>
  );
}

const fileKinds: readonly [RegExp, typeof File][] = [
  [/\.(pdf|docx?|odt|rtf|txt|md|pages)$/i, FileText],
  [/\.(xlsx?|ods|csv|numbers)$/i, FileSpreadsheet],
  [/\.(pptx?|odp|key)$/i, Presentation],
  [/\.(zip|tar|gz|tgz|7z|rar)$/i, FileArchive],
  [/\.(png|jpe?g|gif|webp|svg|heic|tiff?)$/i, FileImage],
  [/\.(mp3|m4a|wav|aac|ogg|flac)$/i, FileAudio],
  [/\.(mp4|mov|m4v|webm|mkv|avi)$/i, FileVideo],
  [/\.(html?|xml|json|py|js|ts|c|h|java|r|ipynb|tex)$/i, FileCode],
];
/** Leading tile of a file row: a glyph for the kind of file, from its name. */
export function FileIcon({ name }: { name: string }) {
  const Icon = fileKinds.find(([pattern]) => pattern.test(name))?.[1] ?? File;
  return (
    <span className="row-icon file-icon" aria-hidden="true">
      <Icon size={16} strokeWidth={1.75} />
    </span>
  );
}

/** Title row of a local view: what it is, how much, and its commands. */
export function PageHeader({
  title,
  detail,
  children,
}: {
  title: string;
  detail?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <header className="page-header">
      <div className="page-heading">
        <h2>{title}</h2>
        {detail && <span className="page-detail">{detail}</span>}
      </div>
      {children && <div className="page-actions">{children}</div>}
    </header>
  );
}

export function EmptyState({
  icon,
  title,
  hint,
}: {
  icon: ReactNode;
  title: string;
  hint?: string;
}) {
  return (
    <div className="empty-state">
      <span className="empty-icon" aria-hidden="true">
        {icon}
      </span>
      <p className="empty-title">{title}</p>
      {hint && <p className="empty-hint">{hint}</p>}
    </div>
  );
}

/** Placeholder rows while a cached list loads; announced once as "Loading…". */
export function Skeleton({ rows = 4 }: { rows?: number }) {
  const { t } = useI18n();
  return (
    <div className="group skeleton" role="status" aria-label={t.common.loading}>
      {Array.from({ length: rows }, (_, index) => (
        <div className="list-row" key={index} aria-hidden="true">
          <span className="skeleton-tile" />
          <span className="skeleton-lines">
            <span />
            <span />
          </span>
        </div>
      ))}
    </div>
  );
}

/** Refresh command plus an honest "Updated … ago" for the oldest organisation. */
export function RefreshControl({
  sites,
  refreshed,
  busy,
  onRefresh,
}: {
  sites: readonly Site[];
  refreshed: readonly { siteId: string; refreshedAt: number }[];
  busy: boolean;
  onRefresh: () => void;
}) {
  const { t, locale } = useI18n();
  const times = sites.map(
    (site) => refreshed.find((r) => r.siteId === site.id)?.refreshedAt,
  );
  const oldest = times.includes(undefined)
    ? undefined
    : Math.min(...(times as number[]));
  const when = oldest === undefined ? null : ago(locale, oldest);
  const status = busy
    ? t.common.refreshing
    : oldest === undefined
      ? t.common.never
      : when
        ? t.common.updated(when)
        : t.common.updatedNow;
  return (
    <>
      <span className="refresh-status" role="status">
        {status}
      </span>
      <button
        className="button icon"
        title={t.common.refresh}
        aria-label={t.common.refresh}
        disabled={busy || sites.length === 0}
        onClick={onRefresh}
      >
        {busy ? (
          <LoaderCircle
            className="spin"
            size={14}
            strokeWidth={2}
            aria-hidden="true"
          />
        ) : (
          <RotateCw size={14} strokeWidth={2} aria-hidden="true" />
        )}
      </button>
    </>
  );
}
