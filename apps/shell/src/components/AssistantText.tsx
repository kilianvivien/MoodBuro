import type { ReactNode } from "react";

/**
 * An answer's text with the little Markdown models write: paragraphs, bullet and
 * numbered lists, headings, bold, italic and inline code. Everything becomes
 * React text nodes, so nothing the model writes is ever parsed as HTML, and
 * there are no links: Moodle items open through the verified sources below the
 * answer. A marker that is not closed yet (mid-stream) stays as typed.
 */

type Para = { kind: "p"; lines: string[] };
type List = {
  kind: "ul" | "ol";
  /** `number` is the one the model wrote, so a list split by a blank line or a
      nested bullet keeps counting instead of starting again at 1. */
  items: { text: string; depth: number; number?: number }[];
};
type Block = Para | List | { kind: "h"; text: string };

const BULLET = /^(\s*)[-*•]\s+(.*)$/u;
const NUMBER = /^(\s*)(\d{1,3})[.)]\s+(.*)$/u;
const HEADING = /^\s{0,3}#{1,6}\s+(.*?)\s*#*$/u;
const INLINE = /\*\*(.+?)\*\*|`([^`]+)`|\*([^*\s](?:[^*]*[^*\s])?)\*/gu;

function blocks(text: string): Block[] {
  const out: Block[] = [];
  let para: Para | null = null;
  let list: List | null = null;
  for (const line of text.split("\n")) {
    if (line.trim() === "") {
      para = list = null;
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading) {
      out.push({ kind: "h", text: heading[1] });
      para = list = null;
      continue;
    }
    const bullet = BULLET.exec(line);
    const numbered = bullet ? null : NUMBER.exec(line);
    const item = bullet
      ? { indent: bullet[1], text: bullet[2], number: undefined }
      : numbered && {
          indent: numbered[1],
          text: numbered[3],
          number: Number(numbered[2]),
        };
    if (item) {
      const kind = bullet ? "ul" : "ol";
      const nested = item.indent.length >= 2;
      // A nested list of the other kind stays inside the one it belongs to.
      if (!list || (!nested && list.kind !== kind)) {
        list = { kind, items: [] };
        out.push(list);
      }
      list.items.push({
        text: item.text,
        depth: nested ? 1 : 0,
        number: list.kind === "ol" && !nested ? item.number : undefined,
      });
      para = null;
      continue;
    }
    if (list && /^\s{2,}/u.test(line)) {
      // An indented line continues the item above it.
      list.items[list.items.length - 1].text += ` ${line.trim()}`;
      continue;
    }
    if (!para) {
      para = { kind: "p", lines: [] };
      out.push(para);
    }
    list = null;
    para.lines.push(line);
  }
  return out;
}

function inline(text: string): ReactNode[] {
  const out: ReactNode[] = [];
  let at = 0;
  for (const match of text.matchAll(INLINE)) {
    if (match.index > at) out.push(text.slice(at, match.index));
    const key = match.index;
    if (match[1] !== undefined)
      out.push(<strong key={key}>{inline(match[1])}</strong>);
    else if (match[2] !== undefined)
      out.push(<code key={key}>{match[2]}</code>);
    else out.push(<em key={key}>{match[3]}</em>);
    at = match.index + match[0].length;
  }
  if (at < text.length) out.push(text.slice(at));
  return out;
}

export function AssistantText({ text }: { text: string }) {
  return (
    <div className="assistant-text">
      {blocks(text).map((block, index) => {
        if (block.kind === "h")
          return (
            <p key={index} className="assistant-subhead">
              {inline(block.text)}
            </p>
          );
        if (block.kind === "p")
          return <p key={index}>{inline(block.lines.join("\n"))}</p>;
        const List = block.kind;
        return (
          <List key={index}>
            {block.items.map((item, at) => (
              <li
                key={at}
                value={item.number}
                data-depth={item.depth || undefined}
              >
                {inline(item.text)}
              </li>
            ))}
          </List>
        );
      })}
    </div>
  );
}
