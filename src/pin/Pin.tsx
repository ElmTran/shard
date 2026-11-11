import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Menu } from "@tauri-apps/api/menu";
import { TextLayer } from "../ocr/TextLayer";
import { selectedText } from "../ocr/text";
import { FADE_MS, decodeImage, errorText, ipc, type OcrLine, presentWindow, sleep, toBitmap } from "../lib/ipc";

/**
 * 贴图窗口：1:1 显示图片；拖动移动；双击或 Esc 关闭；右键菜单。
 * 显示后在后台自动识别文字：文字层不可见，但可以直接框选复制（PixPin 的做法）
 */
export function Pin() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const textLayerRef = useRef<HTMLDivElement>(null);
  const [lines, setLines] = useState<OcrLine[]>([]);
  const [closing, setClosing] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number>(0);

  const notify = (msg: string) => {
    setToast(msg);
    clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1600);
  };

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      const raw = decodeImage(await ipc.getPinImage());
      const bitmap = await toBitmap(raw);
      if (cancelled) return;
      const canvas = canvasRef.current!;
      canvas.width = raw.width;
      canvas.height = raw.height;
      // 图片按 1:1 物理像素显示
      canvas.style.width = `${raw.width / window.devicePixelRatio}px`;
      canvas.style.height = `${raw.height / window.devicePixelRatio}px`;
      canvas.getContext("2d")!.drawImage(bitmap, 0, 0);
      bitmap.close();
      await presentWindow();
      // 识别失败不打扰，只是不能选文字
      ipc
        .pinOcr()
        .then((l) => !cancelled && setLines(l))
        .catch((e) => console.warn(errorText(e)));
    };
    const fail = (e: unknown) => {
      console.error(e);
      ipc.closePin();
    };
    // 这个窗口可能是预先建好的备用窗口：先挂上监听，再告诉 Rust 页面好了
    let loaded = false;
    const once = () => {
      if (loaded) return;
      loaded = true;
      load().catch(fail);
    };
    const unlisten = listen("shard://pin-load", once);
    unlisten
      .then(() => invoke<boolean>("pin_ready"))
      .then((ready) => ready && once())
      .catch(fail);
    return () => {
      cancelled = true;
      unlisten.then((f) => f());
    };
  }, []);

  const close = async () => {
    if (closing) return;
    setClosing(true);
    await sleep(FADE_MS);
    ipc.closePin();
  };

  const copyImage = () =>
    ipc
      .pinCopy()
      .then(() => notify("已复制图片"))
      .catch((e) => notify(errorText(e)));

  const copyText = (t: string) =>
    ipc
      .copyText(t)
      .then(() => notify("已复制文字"))
      .catch((e) => notify(errorText(e)));

  const save = () =>
    ipc
      .pinSave()
      .then((saved) => saved && notify("已保存"))
      .catch((e) => notify(errorText(e)));

  // 菜单和快捷键的处理函数每次渲染更新
  const handlers = useRef({ close, copyImage, copyText, save, lines });
  handlers.current = { close, copyImage, copyText, save, lines };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      if (key === "escape") handlers.current.close();
      else if (e.ctrlKey && key === "c") {
        e.preventDefault();
        const t = selectedText(textLayerRef.current);
        if (t) handlers.current.copyText(t);
        else handlers.current.copyImage();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const onMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0 || (e.target as Element).closest("[data-word]")) return;
    // 阴影边距里的点击不算
    if (!(e.target as Element).closest(".pin-image")) return;
    window.getSelection()?.removeAllRanges();
    if (e.detail >= 2) return close();
    getCurrentWindow().startDragging();
  };

  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    const h = handlers.current;
    const selected = selectedText(textLayerRef.current);
    const all = h.lines.map((l) => l.text).join("\n");
    const menu = await Menu.new({
      items: [
        ...(selected ? [{ id: "copy-sel", text: "复制选中文字", action: () => h.copyText(selected) }] : []),
        { id: "copy", text: "复制图片", action: () => h.copyImage() },
        ...(all ? [{ id: "copy-text", text: "复制全部文字", action: () => h.copyText(all) }] : []),
        { id: "save", text: "保存…", action: () => h.save() },
        { item: "Separator" as const },
        { id: "close", text: "关闭", action: () => h.close() },
      ],
    });
    await menu.popup();
  };

  return (
    <div className={`pin ${closing ? "pin-closing" : ""}`} onMouseDown={onMouseDown} onContextMenu={onContextMenu}>
      <div className="pin-image">
        <canvas ref={canvasRef} className="block" />
        {lines.length > 0 && (
          <TextLayer ref={textLayerRef} lines={lines} scale={1 / window.devicePixelRatio} quiet style={{ inset: 0 }} />
        )}
        {toast && <div className="pin-toast">{toast}</div>}
      </div>
    </div>
  );
}
