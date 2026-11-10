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
        site: "Campus",
        subject: "A specific notification",
      },
    });
    expect(html).toContain('role="alertdialog"');
    expect(html).toContain("A specific notification");
    expect(html).toContain("Campus");
    expect(html).toContain('aria-describedby="assistant-confirm-body"');
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
