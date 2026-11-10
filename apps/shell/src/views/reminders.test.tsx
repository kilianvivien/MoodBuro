import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "../i18n/context";
import { ReminderRows } from "./Reminders";
import type { ReminderSettings, Site } from "../ipc";
const sites: Site[] = [
  {
    id: "a",
    userId: 7,
    name: "Example Campus",
    wwwroot: "https://example.org/",
    fullName: "Learner",
  },
];
const setting: ReminderSettings = {
  siteId: "a",
  userId: 7,
  hour: false,
  day: false,
  enabledAt: 0,
  refreshedAt: null,
  refreshError: null,
  deliveryFailed: false,
};
const render = (
  settings: ReminderSettings[],
  busy = false,
  locale: "en" | "fr" = "en",
) =>
  renderToStaticMarkup(
    <I18nProvider locale={locale}>
      <ReminderRows
        sites={sites}
        settings={settings}
        busy={busy}
        now={10000}
        onChange={vi.fn()}
      />
    </I18nProvider>,
  );
describe("local deadline reminder preferences", () => {
  it("starts unchecked and labels both offsets for the specific organisation", () => {
    const html = render([setting]);
    expect(html).not.toContain('checked=""');
    expect(html).toContain('aria-label="Example Campus: 24 hours before"');
    expect(html).toContain('aria-label="Example Campus: 1 hour before"');
    expect(html).toContain("No successful agenda refresh yet");
  });
  it("waits for current-account settings and disables controls while saving", () => {
    expect(render([]).match(/disabled=""/g)).toHaveLength(2);
    expect(render([{ ...setting, userId: 8, hour: true }])).not.toContain(
      'checked=""',
    );
    expect(render([setting], true).match(/disabled=""/g)).toHaveLength(2);
  });
  it("reports stale data, refresh errors and native attempt failures honestly", () => {
    const html = render([
      {
        ...setting,
        hour: true,
        refreshedAt: 1,
        refreshError: "Could not read the agenda.",
        deliveryFailed: true,
      },
    ]);
    expect(html).toContain("Agenda data is stale");
    expect(html).toContain("Could not read the agenda.");
    expect(html).toContain("A notification attempt failed");
    expect(html.match(/checked=""/g)).toHaveLength(1);
  });
  it("renders the offset labels and empty refresh state in French", () => {
    const html = render([setting], false, "fr");
    expect(html).toContain("24 heures avant");
    expect(html).toContain("1 heure avant");
    expect(html).toContain("Aucune actualisation");
  });
});
