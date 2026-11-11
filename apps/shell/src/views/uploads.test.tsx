import { describe, it, expect, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "../i18n/context";
import { UploadConfirmation, Uploads } from "./Uploads";
import { DownloadList, RemoveConfirmation } from "./Downloads";
import type { Download } from "../ipc";
const review = {
  id: "approval",
  filename: "essay.pdf",
  bytes: 1234,
  organisation: "Example Campus",
  course: "Economics",
  assignment: "Policy brief",
};
const item: Download = {
  id: "d",
  siteId: "s",
  filename: "notes.pdf",
  state: "downloading",
  startedAt: 1,
  needsConfirmation: false,
};
describe("B3 consent and transfer controls", () => {
  for (const locale of ["en", "fr"] as const) {
    it(`names the exact draft destination and disables repeat confirmation (${locale})`, () => {
      const render = (busy = false) =>
        renderToStaticMarkup(
          <I18nProvider locale={locale}>
            <UploadConfirmation
              review={review}
              busy={busy}
              onCancel={vi.fn()}
              onConfirm={vi.fn()}
              onStop={vi.fn()}
            />
          </I18nProvider>,
        );
      const html = render();
      for (const label of [
        "essay.pdf",
        "Example Campus",
        "Economics",
        "Policy brief",
      ])
        expect(html).toContain(label);
      expect(html).toContain('role="alertdialog"');
      expect(html).not.toContain('type="file"');
      expect(html).not.toContain("https:");
      expect(
        html.indexOf(locale === "en" ? ">Cancel<" : ">Annuler<"),
      ).toBeLessThan(
        html.indexOf(
          locale === "en" ? ">Upload Draft<" : ">Envoyer le brouillon<",
        ),
      );
      expect(render(true)).toContain('disabled=""');
      expect(render(true)).toContain(locale === "en" ? "Stop" : "Arrêter");
    });
  }
  it("does not offer cancel until the native transfer handle exists", () => {
    const render = (progress: boolean) =>
      renderToStaticMarkup(
        <I18nProvider locale="en">
          <DownloadList
            items={[item]}
            sites={[]}
            busy={false}
            confirming={null}
            onOpen={vi.fn()}
            onReveal={vi.fn()}
            onCancel={vi.fn()}
            onConfirm={vi.fn()}
            onStop={vi.fn()}
            progress={progress ? [{ id: "d", received: 50, total: 100 }] : []}
          />
        </I18nProvider>,
      );
    expect(render(false)).not.toContain("Cancel Transfer");
    expect(render(true)).toContain("Cancel Transfer");
    expect(render(true)).toContain('value="50"');
    expect(render(true)).toContain("50 of 100 bytes");
  });
  it("separates file removal from history clearing and keeps removed files closed", () => {
    const removed = { ...item, state: "removed" as const };
    const html = renderToStaticMarkup(
      <I18nProvider locale="en">
        <RemoveConfirmation
          item={removed}
          busy={false}
          onCancel={vi.fn()}
          onConfirm={vi.fn()}
        />
        <DownloadList
          items={[removed]}
          sites={[]}
          busy={false}
          confirming={null}
          onOpen={vi.fn()}
          onReveal={vi.fn()}
          onCancel={vi.fn()}
          onConfirm={vi.fn()}
        />
      </I18nProvider>,
    );
    expect(html).toContain("~/Moodle/.trash");
    expect(html).toContain("History stays listed");
    expect(html).toMatch(/aria-label="Open notes.pdf" disabled/);
    expect(html).not.toMatch(/aria-label="Show notes.pdf in Finder" disabled/);
  });
  it("does not render a confirmation before a file and target are selected", () => {
    const html = renderToStaticMarkup(
      <I18nProvider locale="en">
        <Uploads sites={[]} />
      </I18nProvider>,
    );
    expect(html).not.toContain('role="alertdialog"');
    expect(html).toContain("Review Upload");
    expect(html).not.toContain('type="file"');
  });
});
