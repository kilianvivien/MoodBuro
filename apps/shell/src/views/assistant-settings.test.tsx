import { describe, it, expect, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  AssistantSettingsView,
  type AssistantSettingsViewProps,
} from "./AssistantSettings";
import { ApiKeyHelp } from "../components/ApiKeyHelp";
import { I18nProvider } from "../i18n/context";
const props: AssistantSettingsViewProps = {
  provider: "albert",
  model: "old-unlisted-model",
  hasKey: true,
  consent: false,
  models: ["provider-model-a", "provider-model-b"],
  busy: false,
  checking: false,
  asking: null,
  outcome: null,
  keyText: "",
  onKeyText: vi.fn(),
  onProvider: vi.fn(),
  onSaveKey: vi.fn(),
  onRemoveKey: vi.fn(),
  onModel: vi.fn(),
  onCheck: vi.fn(),
  onConsent: vi.fn(),
  onClearAll: vi.fn(),
  onAsk: vi.fn(),
};
describe("assistant settings", () => {
  it("offers exactly the provider's model IDs and no free text model input", () => {
    const html = renderToStaticMarkup(
      <I18nProvider locale="en">
        <AssistantSettingsView {...props} />
      </I18nProvider>,
    );
    expect(html).toContain('value="provider-model-a"');
    expect(html).toContain('value="provider-model-b"');
    expect(html).not.toContain("old-unlisted-model");
    expect(html).not.toContain('aria-label="Model name"');
    expect(html).toContain("Get an API key…");
    expect(html).toContain("What gets shared");
    const waiting = renderToStaticMarkup(
      <I18nProvider locale="en">
        <AssistantSettingsView {...props} models={null} checking />
      </I18nProvider>,
    );
    expect(waiting).toContain('<select disabled=""');
    expect(waiting).not.toContain('type="text"');
  });
  for (const locale of ["en", "fr"] as const)
    for (const provider of ["albert", "mistral"] as const)
      it(`explains obtaining the selected provider's key (${provider}/${locale})`, () => {
        const html = renderToStaticMarkup(
          <I18nProvider locale={locale}>
            <ApiKeyHelp provider={provider} onClose={vi.fn()} />
          </I18nProvider>,
        );
        expect(html).toContain("<dialog");
        expect(html.match(/<li>/g)).toHaveLength(3);
        expect(html).toContain(
          provider === "albert" ? "ProConnect" : "API Keys",
        );
        expect(html).not.toContain(
          provider === "albert" ? "Mistral Studio" : "ProConnect",
        );
        expect(html).not.toContain('type="password"');
      });
});
