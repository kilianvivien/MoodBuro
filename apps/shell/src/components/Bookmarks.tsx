import { useEffect, useRef, useState } from "react";
import {
  Bookmark as BookmarkGlyph,
  Star,
  BookOpen,
  CalendarDays,
  FileText,
  Folder,
  MessageSquare,
  GraduationCap,
  Pencil,
  Trash2,
  ChevronsUpDown,
} from "lucide-react";
import { ipc, message, type Bookmark, type Site } from "../ipc";
import { PageHeader } from "./ui";
import { ErrorText } from "../i18n/ErrorText";
import { useI18n } from "../i18n/context";
export const bookmarkIcons = {
  bookmark: BookmarkGlyph,
  star: Star,
  book: BookOpen,
  calendar: CalendarDays,
  file: FileText,
  folder: Folder,
  message: MessageSquare,
  graduation: GraduationCap,
};
export function BookmarkIcon({ name }: { name: string }) {
  const Icon =
    bookmarkIcons[name as keyof typeof bookmarkIcons] ?? BookmarkGlyph;
  return <Icon size={18} strokeWidth={1.75} aria-hidden="true" />;
}
export function BookmarksView({
  items,
  sites,
  initialDraft,
  onChanged,
  onOpen,
}: {
  items: Bookmark[];
  sites: Site[];
  initialDraft: Bookmark | null;
  onChanged: () => Promise<void>;
  onOpen: (bookmark: Bookmark) => void;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<Bookmark | null>(initialDraft);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);
  const [removing, setRemoving] = useState<string | null>(null);
  const nameField = useRef<HTMLInputElement>(null);
  const listRoot = useRef<HTMLUListElement>(null);
  const lastEdited = useRef<string | null>(initialDraft?.id ?? null);
  const draftId = draft?.id;
  useEffect(() => {
    if (draftId) {
      lastEdited.current = draftId;
      nameField.current?.focus();
      nameField.current?.select();
    } else {
      const buttons = Array.from(
        listRoot.current?.querySelectorAll<HTMLButtonElement>(
          "button[data-bookmark-id]",
        ) ?? [],
      );
      (
        buttons.find(
          (button) => button.dataset.bookmarkId === lastEdited.current,
        ) ?? buttons[0]
      )?.focus();
    }
  }, [draftId]);
  async function save() {
    if (!draft || busy) return;
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      await ipc.bookmarkSave(draft);
      await onChanged();
      setDraft(null);
      setSaved(true);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }
  async function remove(id: string) {
    if (busy) return;
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      await ipc.bookmarkDelete(id);
      await onChanged();
      setRemoving(null);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="page bookmarks-page">
      <PageHeader
        title={
          draft
            ? items.some((b) => b.id === draft.id)
              ? t.bookmarks.editTitle
              : t.bookmarks.newTitle
            : t.bookmarks.title
        }
        detail={draft ? undefined : String(items.length)}
      />
      {error && (
        <p role="alert" className="section-message error">
          <ErrorText error={error} context="navigation" />
        </p>
      )}
      {saved && (
        <p role="status" className="section-message">
          {t.bookmarks.saved}
        </p>
      )}
      {draft ? (
        <form
          className="group bookmark-editor"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.stopPropagation();
              setDraft(null);
            }
          }}
        >
          <div className="group-row bookmark-preview">
            <span className="bookmark-preview-icon">
              <BookmarkIcon name={draft.icon} />
            </span>
            <span className="row-text">
              <span className="row-title">
                {draft.name || t.bookmarks.name}
              </span>
              <span className="row-subtitle">
                {sites.find((s) => s.id === draft.siteId)?.name}
              </span>
            </span>
          </div>
          <label className="group-row">
            <span>{t.bookmarks.name}</span>
            <input
              ref={nameField}
              className="text-field"
              value={draft.name}
              maxLength={100}
              required
              disabled={busy}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </label>
          <label className="group-row">
            <span>{t.bookmarks.icon}</span>
            <span className="popup-button">
              <select
                value={draft.icon}
                disabled={busy}
                onChange={(e) => setDraft({ ...draft, icon: e.target.value })}
              >
                {Object.keys(bookmarkIcons).map((key) => (
                  <option key={key} value={key}>
                    {t.bookmarks.icons[key as keyof typeof bookmarkIcons]}
                  </option>
                ))}
              </select>
              <ChevronsUpDown size={12} strokeWidth={2} aria-hidden="true" />
            </span>
          </label>
          <div className="group-row bookmark-actions">
            <button
              type="button"
              className="button"
              disabled={busy}
              onClick={() => setDraft(null)}
            >
              {t.bookmarks.cancel}
            </button>
            <button
              className="button primary"
              disabled={busy || !draft.name.trim()}
            >
              {busy ? t.bookmarks.saving : t.bookmarks.save}
            </button>
          </div>
        </form>
      ) : (
        <ul
          ref={listRoot}
          className="group bookmark-list"
          aria-label={t.bookmarks.title}
        >
          {items.map((b) => (
            <li key={b.id} className="group-row">
              <button
                className="bookmark-link"
                data-bookmark-id={b.id}
                disabled={busy}
                title={b.name}
                onClick={() => onOpen(b)}
              >
                <BookmarkIcon name={b.icon} />
                <span className="row-text">
                  <span className="row-title">{b.name}</span>
                  <span className="row-subtitle">
                    {sites.find((s) => s.id === b.siteId)?.name}
                  </span>
                </span>
              </button>
              {removing === b.id ? (
                <span
                  className="row-actions bookmark-confirm"
                  role="group"
                  aria-label={t.bookmarks.remove}
                >
                  <span>{t.bookmarks.removeQuestion}</span>
                  <button
                    className="button"
                    autoFocus
                    disabled={busy}
                    onClick={() => setRemoving(null)}
                  >
                    {t.bookmarks.cancel}
                  </button>
                  <button
                    className="button destructive"
                    disabled={busy}
                    onClick={() => void remove(b.id)}
                  >
                    {t.bookmarks.remove}
                  </button>
                </span>
              ) : (
                <span className="row-actions">
                  <button
                    className="tool"
                    disabled={busy}
                    title={t.bookmarks.edit}
                    aria-label={`${t.bookmarks.edit} — ${b.name}`}
                    onClick={() => {
                      setDraft(b);
                      setSaved(false);
                    }}
                  >
                    <Pencil size={15} />
                  </button>
                  <button
                    className="tool"
                    disabled={busy}
                    title={t.bookmarks.remove}
                    aria-label={`${t.bookmarks.remove} — ${b.name}`}
                    onClick={() => setRemoving(b.id)}
                  >
                    <Trash2 size={15} />
                  </button>
                </span>
              )}
            </li>
          ))}
          {!items.length && (
            <li className="group-row section-message">{t.bookmarks.empty}</li>
          )}
        </ul>
      )}
      <p className="section-footnote">{t.bookmarks.local}</p>
    </div>
  );
}
