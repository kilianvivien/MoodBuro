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
} from "lucide-react";
import { ipc, message, type Bookmark, type Site } from "../ipc";
import { Confirm, EmptyState, Notice, PageHeader, PopupButton } from "./ui";
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
        <Notice tone="error">
          <ErrorText error={error} context="navigation" />
        </Notice>
      )}
      {saved && <Notice tone="success">{t.bookmarks.saved}</Notice>}
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
          <div className="group-row">
            <span className="row-icon" aria-hidden="true">
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
          <div className="group-row">
            <label className="row-text" htmlFor="bookmark-name">
              <span className="row-title">{t.bookmarks.name}</span>
            </label>
            <span
              className="text-field compact bookmark-name"
              data-disabled={busy || undefined}
            >
              <input
                id="bookmark-name"
                ref={nameField}
                value={draft.name}
                maxLength={100}
                required
                disabled={busy}
                autoComplete="off"
                spellCheck={false}
                onChange={(e) => setDraft({ ...draft, name: e.target.value })}
              />
            </span>
          </div>
          <div className="group-row">
            <label className="row-text" htmlFor="bookmark-icon">
              <span className="row-title">{t.bookmarks.icon}</span>
            </label>
            <PopupButton
              id="bookmark-icon"
              value={draft.icon}
              disabled={busy}
              onChange={(icon) => setDraft({ ...draft, icon })}
            >
              {Object.keys(bookmarkIcons).map((key) => (
                <option key={key} value={key}>
                  {t.bookmarks.icons[key as keyof typeof bookmarkIcons]}
                </option>
              ))}
            </PopupButton>
          </div>
          <div className="group-row group-footer">
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
      ) : items.length === 0 ? (
        <EmptyState
          icon={<BookmarkGlyph size={22} strokeWidth={1.75} />}
          title={t.bookmarks.emptyTitle}
          hint={t.bookmarks.empty}
        />
      ) : (
        <ul
          ref={listRoot}
          className="group bookmark-list"
          role="list"
          aria-label={t.bookmarks.title}
        >
          {items.map((b) => (
            <li key={b.id}>
              <div className="list-row">
                <span className="row-icon" aria-hidden="true">
                  <BookmarkIcon name={b.icon} />
                </span>
                <span className="row-text">
                  <span className="row-title" title={b.name}>
                    {b.name}
                  </span>
                  <span className="row-subtitle">
                    {sites.find((s) => s.id === b.siteId)?.name}
                  </span>
                </span>
                <span className="row-actions">
                  <button
                    className="button small icon"
                    disabled={busy || removing !== null}
                    title={t.bookmarks.edit}
                    aria-label={`${t.bookmarks.edit} — ${b.name}`}
                    onClick={() => {
                      setDraft(b);
                      setSaved(false);
                    }}
                  >
                    <Pencil size={13} strokeWidth={2} aria-hidden="true" />
                  </button>
                  <button
                    className="button small icon"
                    disabled={busy || removing !== null}
                    title={t.bookmarks.remove}
                    aria-label={`${t.bookmarks.remove} — ${b.name}`}
                    onClick={() => {
                      lastEdited.current = b.id;
                      setRemoving(b.id);
                    }}
                  >
                    <Trash2 size={13} strokeWidth={2} aria-hidden="true" />
                  </button>
                  <button
                    className="button small"
                    data-bookmark-id={b.id}
                    disabled={busy}
                    aria-label={`${t.courses.open} — ${b.name}`}
                    onClick={() => onOpen(b)}
                  >
                    {t.courses.open}
                  </button>
                </span>
              </div>
              {removing === b.id && (
                <Confirm
                  id={`remove-bookmark-${b.id}`}
                  title={t.bookmarks.removeTitle(b.name)}
                  body={t.bookmarks.removeBody}
                  cancel={t.bookmarks.cancel}
                  confirm={t.bookmarks.confirmRemove}
                  tone="destructive"
                  nested
                  busy={busy}
                  onCancel={() => setRemoving(null)}
                  onConfirm={() => void remove(b.id)}
                />
              )}
            </li>
          ))}
        </ul>
      )}
      <p className="section-footnote">{t.bookmarks.local}</p>
    </div>
  );
}
