import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "../i18n/context";
import {
  AssistantView,
  type AssistantViewProps,
} from "../components/AssistantPanel";
import { contextFrom, withoutMarkers } from "../lib/assistant";
import { clampWidth } from "../stores/assistant";

const props: AssistantViewProps = {
  site: undefined,
  provider: "albert",
  hasKey: true,
  consent: false,
  courses: [],
  entries: [],
  draft: "",
  busy: false,
  step: null,
  error: null,
  canRetry: false,
  confirmation: null,
  clearing: false,
  context: "none",
  text: "",
  onText: vi.fn(),
  onContext: vi.fn(),
  onSend: vi.fn(),
  onStop: vi.fn(),
  onRetry: vi.fn(),
  onAnswer: vi.fn(),
  onAskClear: vi.fn(),
  onClear: vi.fn(),
  onSource: vi.fn(),
  onSettings: vi.fn(),
  onClose: vi.fn(),
};
const render = (
  change: Partial<AssistantViewProps> = {},
  locale: "en" | "fr" = "en",
) =>
  renderToStaticMarkup(
    <I18nProvider locale={locale}>
      <AssistantView {...props} {...change} />
    </I18nProvider>,
  );

describe("assistant panel states", () => {
  it("renders an answer's Markdown as elements, never as HTML", () => {
    const html = render({
      entries: [
        {
          role: "assistant",
          text: "Only **one** item:\n\n- the *guide* <b>x</b>\n  - `nested`\n\nUnclosed **marker",
          context: null,
          sources: [],
          notes: [],
        },
      ],
    });
    expect(html).toContain("<strong>one</strong>");
    expect(html).toContain("<em>guide</em> &lt;b&gt;x&lt;/b&gt;");
    expect(html).toContain('<li data-depth="1"><code>nested</code></li>');
    expect(html).toContain("Unclosed **marker");
  });
  it("keeps the numbers of a list split by blank lines or nested bullets", () => {
    const html = render({
      entries: [
        {
          role: "assistant",
          text: "1. First\n  - detail\n2. Second\n\n3. Third",
          context: null,
          sources: [],
          notes: [],
        },
      ],
    });
    expect(html).toContain('<li value="2">Second</li>');
    expect(html).toContain('<li value="3">Third</li>');
    expect(html).toContain('<li data-depth="1">detail</li>');
  });
  it("requires setup without a key and exposes no composer", () => {
    const html = render({ hasKey: false });
    expect(html).not.toContain("<textarea");
    expect(html).toContain("Settings");
    expect(html).not.toContain('role="log"');
  });
  it("allows general chat without a site or context consent in both languages", () => {
    for (const locale of ["en", "fr"] as const) {
      const html = render({}, locale);
      expect(html).toContain("<textarea");
      expect(html).toContain('role="log"');
      expect(html).toContain('value="none"');
      expect(html).not.toContain('value="site"');
      expect(html).toContain("Albert API");
    }
  });
  it("renders streamed answers as escaped text and hides citation markers", () => {
    const html = render({
      busy: true,
      draft: "<script>bad()</script> [course:42]",
    });
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("[course:42]");
    expect(html).toContain('aria-busy="true"');
  });
  it("previews the exact write target and site in a labelled confirmation", () => {
    const html = render({
      confirmation: {
        request: "r",
        id: "c",
        kind: "mark-read",
        site: "Campus",
        subject: "A specific notification",
        detail: null,
      },
    });
    expect(html).toContain("Mark this notification as read?");
    expect(html).toContain('role="alertdialog"');
    expect(html).toContain("A specific notification");
    expect(html).toContain("Campus");
    expect(html).toContain('aria-describedby="assistant-confirm-body"');
  });
  it("words each confirmation for what it will actually do", () => {
    const ask = (
      kind: "save-file" | "reminder",
      detail: string | null,
      locale: "en" | "fr" = "en",
    ) =>
      render(
        {
          confirmation: {
            request: "r",
            id: "c",
            kind,
            site: "Campus",
            subject: "notes.pdf",
            detail,
          },
        },
        locale,
      );
    const save = ask("save-file", "1400000");
    expect(save).toContain("Save this file to your Mac?");
    expect(save).toContain("notes.pdf");
    expect(save).toContain("(1.4 MB)");
    expect(save).toContain(">Don’t Save<");
    // An unknown size is left out rather than shown as zero.
    expect(ask("save-file", null)).not.toContain("(");
    expect(ask("save-file", "1400000", "fr")).toMatch(/\(1,4\sMo\)/u);
    expect(ask("reminder", "hour")).toContain("1 hour before this deadline?");
    expect(ask("reminder", "day")).toContain("24 hours before this deadline?");
    expect(ask("reminder", "day", "fr")).toContain("24 heures avant");
  });
  it("offers suggestions only in an empty conversation, and file chips say what they do", () => {
    const empty = render({ suggestions: ["What is due this week?"] });
    expect(empty).toContain('aria-label="Suggested questions"');
    expect(empty).toContain(">What is due this week?<");
    expect(render()).not.toContain("assistant-suggestion");
    const answered = render({
      suggestions: ["What is due this week?"],
      entries: [
        {
          role: "assistant",
          text: "Here.",
          context: null,
          sources: [
            {
              kind: "file",
              id: 1,
              page: "activity",
              value: 42,
              label: "notes.pdf",
              course: 7,
              key: "a".repeat(64),
            },
          ],
          notes: [
            { code: "saved", label: "notes.pdf", age: null, stale: false },
          ],
        },
      ],
    });
    expect(answered).not.toContain("assistant-suggestion");
    expect(answered).toContain("Show the copy saved on this Mac");
    expect(answered).toContain("Saved to your Moodle folder: notes.pdf");
    // The key identifies the file to Rust; it is never shown.
    expect(answered).not.toContain("a".repeat(64));
  });
  it("hides a file marker while the answer streams", () => {
    expect(withoutMarkers("See the notes [file:3]. And [fi")).toBe(
      "See the notes. And",
    );
  });
  it("keeps clear-history confirmation separate from the chat log", () => {
    expect(render({ clearing: true })).toContain(
      'aria-labelledby="assistant-clear-title"',
    );
  });
  it("offers retry only for an unanswered question", () => {
    expect(
      render({
        error: "The AI provider returned an unexpected response.",
        canRetry: true,
      }),
    ).toContain("Try Again");
    expect(
      render({ error: "The AI provider returned an unexpected response." }),
    ).not.toContain(">Try Again<");
  });
});

it("maps explicit context and hides complete and partial source markers", () => {
  expect(contextFrom("none")).toBeNull();
  expect(contextFrom("site")).toEqual({ kind: "site", courseId: null });
  expect(contextFrom("course:42")).toEqual({ kind: "course", courseId: 42 });
  expect(withoutMarkers("Answer [course:42]. [activity:1")).toBe("Answer.");
  expect(withoutMarkers("Keep [ordinary text]")).toBe("Keep [ordinary text]");
});

it("clamps separator geometry including non-finite values", () => {
  expect(clampWidth(10)).toBe(280);
  expect(clampWidth(900)).toBe(520);
  expect(clampWidth(NaN)).toBe(340);
  expect(clampWidth(Infinity)).toBe(340);
});
