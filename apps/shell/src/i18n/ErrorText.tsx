import { useI18n } from "./context";
import type { ErrorContext } from "./errors";

/**
 * A backend error in the UI language. When it has no translation, the generic
 * text says what failed and the original wording follows as a labelled detail.
 */
export function ErrorText({
  error,
  context,
}: {
  error: string;
  context?: ErrorContext;
}) {
  const i18n = useI18n();
  const { text, detail } = i18n.error(error, context);
  return (
    <span className="error-text">
      {text}
      {detail && (
        <span className="error-detail">
          {i18n.t.errors.detail} <span lang="en">{detail}</span>
        </span>
      )}
    </span>
  );
}
