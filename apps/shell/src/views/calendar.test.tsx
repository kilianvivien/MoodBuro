import { describe, it, expect, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  CalendarConsent,
  CalendarSubscription,
} from "../components/CalendarSubscription";
import { I18nProvider } from "../i18n/context";
import type { Site } from "../ipc";
const site: Site = {
  id: "s",
  userId: 42,
  wwwroot: "https://example.org/",
  name: "Example Campus",
  fullName: "Learner",
};
describe("calendar subscription consent", () => {
  for (const locale of ["en", "fr"] as const) {
    it(`explains private access briefly and warns on repeated setup (${locale})`, () => {
      const render = (repeat: boolean, busy = false) =>
        renderToStaticMarkup(
          <I18nProvider locale={locale}>
            <CalendarConsent
              site={site}
              repeat={repeat}
              busy={busy}
              onCancel={vi.fn()}
              onConfirm={vi.fn()}
            />
          </I18nProvider>,
        );
      const first = render(false);
      expect(first).toContain('role="alertdialog"');
      expect(first).toContain("Example Campus");
      expect(first).toContain(
        locale === "en" ? "private feed link" : "lien privé",
      );
      expect(first).not.toContain("authtoken");
      expect(first).not.toContain("https://");
      expect(first).not.toContain(
        locale === "en" ? "duplicate subscription" : "abonnement en double",
      );
      expect(render(true)).toContain(
        locale === "en" ? "duplicate subscription" : "abonnement en double",
      );
      expect(render(true, true).match(/disabled=""/g)).toHaveLength(2);
    });
  }
  it("keeps setup collapsed and identifies the rolling read-only feed", () => {
    const html = renderToStaticMarkup(
      <I18nProvider locale="en">
        <CalendarSubscription sites={[site]} />
      </I18nProvider>,
    );
    expect(html).toContain("next 60 days");
    expect(html).toContain("Read-only");
    expect(html).not.toContain('open=""');
    expect(html).not.toContain('role="alertdialog"');
    expect(html).toContain("Moodle Export…");
    expect(html).not.toContain("authtoken");
  });
});
