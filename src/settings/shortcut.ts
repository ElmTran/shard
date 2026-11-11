/**
 * 把键盘事件转成 tauri-plugin-global-shortcut 认的快捷键字符串，如 "Ctrl+Shift+A"、"Alt+F1"。
 * 只按了修饰键时返回 null；没有修饰键的普通键也不行（会抢掉正常打字），F 键等功能键除外
 */
export function shortcutFromEvent(e: KeyboardEvent): { value: string | null; preview: string[]; error?: string } {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");

  const key = keyName(e.code);
  if (!key) return { value: null, preview: mods };
  const parts = [...mods, key];
  if (!mods.length && !STANDALONE.test(key)) {
    return { value: null, preview: parts, error: "需要搭配 Ctrl、Alt、Shift 或 Win 键" };
  }
  return { value: parts.join("+"), preview: parts };
}

/** 不需要修饰键也可以单独做快捷键的键 */
const STANDALONE = /^(F([1-9]|1[0-9]|2[0-4])|PrintScreen|Pause|ScrollLock|Insert)$/;

const MODIFIER_CODES = /^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/;

function keyName(code: string): string | null {
  if (!code || MODIFIER_CODES.test(code)) return null;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  // 其余键（F1、Space、PrintScreen、Backquote、ArrowUp、Numpad1 …）直接用 code，插件能识别
  return code;
}

/** 显示用：拆成一个个按键 */
export function shortcutKeys(value: string): string[] {
  return value.split("+").map((k) => (k === "Super" ? "Win" : k));
}
