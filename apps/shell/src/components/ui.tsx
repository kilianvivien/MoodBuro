import {
  useLayoutEffect,
  useRef,
  type KeyboardEvent,
  type ReactNode,
  type Ref,
} from "react";
import {
  ChevronsUpDown,
  LoaderCircle,
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

/** Pop-up button built on a native select, so the menu is the real macOS one. */
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
    <label className="popup-button">
      <span className="visually-hidden">{t.common.siteFilter}</span>
      <select
        value={value ?? ""}
        onChange={(event) => onChange(event.target.value || null)}
      >
        <option value="">{t.common.allSites}</option>
        {sites.map((site) => (
          <option key={site.id} value={site.id}>
            {site.name}
          </option>
        ))}
      </select>
      <ChevronsUpDown size={12} strokeWidth={2} aria-hidden="true" />
    </label>
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

/** Small monogram tile naming the organisation a row belongs to. */
export function SiteTag({ site }: { site: Site | undefined }) {
  if (!site) return null;
  return <span className="site-tag">{site.name}</span>;
}
