import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ResourceFiles, SavedFiles } from "./CourseFiles";
import { I18nProvider } from "../i18n/context";
import { type ReactNode } from "react";
import { type Locale } from "../i18n/locale";
import type { OfflineCopy, OfflineResource } from "../ipc";
const render = (node: ReactNode, locale: Locale = "en") =>
  renderToStaticMarkup(<I18nProvider locale={locale}>{node}</I18nProvider>);
const resource: OfflineResource = {
  key: "a".repeat(64),
  moduleId: 42,
  sectionId: 1,
  sectionName: "Week 1",
  activityName: "Lecture",
  filename: "notes.pdf",
  bytes: null,
};
const copy: OfflineCopy = {
  id: "copy",
  siteId: "s",
  courseId: 2,
  resourceKey: resource.key,
  moduleId: 42,
  filename: "notes.pdf",
  sectionName: "Week 1",
  bytes: 128,
  savedAt: 1792888200,
  available: true,
  needsConfirmation: false,
};
describe("offline course files", () => {
  it("requires explicit selection, states unknown sizes and disables oversized files", () => {
    const html = render(
      <ResourceFiles
        items={[
          resource,
          {
            ...resource,
            key: "large",
            filename: "video.mp4",
            bytes: 129 * 1024 * 1024,
          },
        ]}
        selected={[resource.key]}
        busy={false}
        onToggle={vi.fn()}
      />,
    );
    expect(html).toContain("Size unknown");
    expect(html).toContain('checked=""');
    expect(html).toContain('disabled=""');
    expect(html).toContain("Exceeds the 128 MB limit");
    expect(html).not.toContain("token");
  });
  it("offers offline Open and Finder separately from network refresh", () => {
    const html = render(
      <SavedFiles
        items={[copy]}
        busy={false}
        onOpen={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(html).toContain('aria-label="Open notes.pdf"');
    expect(html).toContain('aria-label="Show notes.pdf in Finder"');
    expect(html).toContain('aria-label="Refresh offline copy of notes.pdf"');
    expect(html).not.toContain('disabled=""');
  });
  it("keeps missing copies listed while allowing a fresh copy to be requested", () => {
    const html = render(
      <SavedFiles
        items={[{ ...copy, available: false }]}
        busy={false}
        onOpen={vi.fn()}
        onRefresh={vi.fn()}
      />,
    );
    expect(html.match(/disabled=""/g)).toHaveLength(2);
    expect(html).toContain("moved or is unavailable");
    const french = render(
      <SavedFiles
        items={[]}
        busy={false}
        onOpen={vi.fn()}
        onRefresh={vi.fn()}
      />,
      "fr",
    );
    expect(french).toContain("Aucune copie enregistrée");
  });
});
