/** 与 Rust 侧 ocr::is_cjk / join_words 保持一致：相邻处任一侧是中文就直接相连，否则加空格 */
const CJK = new RegExp(
  "[" +
    "\\u2E80-\\u2FDF" + // 部首
    "\\u3000-\\u303F" + // 中文标点
    "\\u3040-\\u30FF" + // 假名
    "\\u3100-\\u312F" + // 注音
    "\\u3400-\\u4DBF" + // 扩展 A
    "\\u4E00-\\u9FFF" + // 基本汉字
    "\\uAC00-\\uD7AF" + // 谚文
    "\\uF900-\\uFAFF" + // 兼容汉字
    "\\uFE30-\\uFE4F" + // 竖排标点
    "\\uFF00-\\uFFEF" + // 全角字符
    "\\u{20000}-\\u{3FFFF}" +
    "]",
  "u",
);

export function joinWords(words: string[]): string {
  let out = "";
  for (const w of words) {
    if (out && w) {
      const prev = [...out].at(-1)!;
      const next = [...w][0];
      if (!CJK.test(prev) && !CJK.test(next)) out += " ";
    }
    out += w;
  }
  return out;
}

/** 文字层里被选中的词，按行拼成文本（部分选中的词按整词算） */
export function selectedText(root: HTMLElement | null): string {
  const sel = window.getSelection();
  if (!root || !sel || sel.isCollapsed) return "";
  const lines = new Map<string, string[]>();
  root.querySelectorAll<HTMLElement>("[data-word]").forEach((span) => {
    if (!sel.containsNode(span, true)) return;
    const line = span.dataset.line!;
    if (!lines.has(line)) lines.set(line, []);
    lines.get(line)!.push(span.textContent ?? "");
  });
  return [...lines.values()].map(joinWords).join("\n");
}
