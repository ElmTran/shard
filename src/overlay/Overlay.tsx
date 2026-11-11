import { type CSSProperties, type PointerEvent as ReactPointerEvent, useEffect, useLayoutEffect, useRef, useState } from "react";
import { TextLayer } from "../ocr/TextLayer";
import { selectedText } from "../ocr/text";
import {
  HANDLE_CURSOR,
  type Handle,
  type Pt,
  type Rect,
  clamp,
  clampPt,
  contains,
  hitHandle,
  moveRect,
  rectFrom,
  resizeRect,
} from "../lib/geometry";
import { FADE_MS, errorText, ipc, onCapture, onHidden, presentWindow, sleep } from "../lib/ipc";
import {
  COLOR,
  type MosaicSource,
  type Shape,
  TEXT_FONT,
  TEXT_LINE_HEIGHT,
  TEXT_SIZE,
  type Tool,
  isMeaningful,
  makeMosaicSource,
  renderShapes,
} from "./annotate";
import { type Anchor, type OcrState, Toolbar } from "./Toolbar";

interface Capture {
  session: number;
  bitmap: ImageBitmap;
  width: number;
  height: number;
  dpr: number;
}

type Drag =
  | { kind: "create"; start: Pt }
  | { kind: "move"; start: Pt; orig: Rect }
  | { kind: "resize"; handle: Handle; start: Pt; orig: Rect }
  | { kind: "draw" };

interface TextEdit {
  at: Pt;
  value: string;
}

/** 选区最小边长（物理像素），更小的当作误点 */
const MIN_SEL = 4;
/** 调整手柄的命中范围（CSS 像素） */
const HANDLE_TOL = 6;
/** 文字输入框的边框 + 内边距（CSS 像素），让框里的文字和最终画上去的位置重合 */
const TEXT_PAD = 3;
const CORNERS = ["nw", "ne", "se", "sw"] as const;
const EDGES = ["n", "e", "s", "w"] as const;

export function Overlay() {
  const [cap, setCap] = useState<Capture | null>(null);
  const [sel, setSel] = useState<Rect | null>(null);
  const [shapes, setShapes] = useState<Shape[]>([]);
  const [draft, setDraft] = useState<Shape | null>(null);
  const [tool, setTool] = useState<Tool | null>(null);
  const [text, setText] = useState<TextEdit | null>(null);
  const [ocr, setOcr] = useState<OcrState | null>(null);
  const [dragging, setDragging] = useState(false);
  /** 正在退出：遮罩、边框、工具栏淡出，之后才真正执行动作并隐藏窗口 */
  const [leaving, setLeaving] = useState(false);
  const [cursor, setCursor] = useState("crosshair");

  const baseRef = useRef<HTMLCanvasElement>(null);
  const annotRef = useRef<HTMLCanvasElement>(null);
  const textLayerRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const drag = useRef<Drag | null>(null);
  const mosaic = useRef<MosaicSource | null>(null);
  const presented = useRef(0);
  const ocrSeq = useRef(0);
  const leavingRef = useRef(false);
  const capRef = useRef<Capture | null>(null);
  capRef.current = cap;

  const dpr = cap?.dpr ?? 1;
  const bounds: Rect = { x: 0, y: 0, w: cap?.width ?? 0, h: cap?.height ?? 0 };

  const resetEditing = () => {
    setSel(null);
    setShapes([]);
    setDraft(null);
    setTool(null);
    setText(null);
    setOcr(null);
    setDragging(false);
    setLeaving(false);
    leavingRef.current = false;
    drag.current = null;
    ocrSeq.current++;
  };

  // 新截图：重置状态 → 画底图后显示窗口（useLayoutEffect）
  useEffect(
    () =>
      onCapture((bitmap, meta) => {
        resetEditing();
        mosaic.current = null;
        setCursor("crosshair");
        setCap((prev) => {
          prev?.bitmap.close();
          return { session: meta.session, bitmap, width: meta.width, height: meta.height, dpr: window.devicePixelRatio };
        });
      }),
    [],
  );

  // 窗口已被 Rust 隐藏：过一会再释放截图。立刻清掉的话页面会变成黑底，
  // 而 DWM 有时还没真正把窗口藏起来，就会闪一帧黑屏。只处理当前这次，防止迟到的通知清掉新截图
  useEffect(() => {
    const unlisten = onHidden((session) => {
      setTimeout(() => {
        const current = capRef.current;
        if (!current || current.session !== session) return;
        current.bitmap.close();
        resetEditing();
        mosaic.current = null;
        setCap(null);
      }, 300);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useLayoutEffect(() => {
    if (!cap || presented.current === cap.session) return;
    presented.current = cap.session;
    const canvas = baseRef.current!;
    canvas.width = cap.width;
    canvas.height = cap.height;
    canvas.getContext("2d")!.drawImage(cap.bitmap, 0, 0);
    presentWindow().catch(console.error);
  }, [cap]);

  const getMosaic = () => (mosaic.current ??= makeMosaicSource(cap!.bitmap, dpr));

  // 标注层：选区、图形变化时重画
  useLayoutEffect(() => {
    const canvas = annotRef.current;
    if (!canvas || !sel || !cap) return;
    canvas.width = Math.max(1, sel.w);
    canvas.height = Math.max(1, sel.h);
    const ctx = canvas.getContext("2d")!;
    renderShapes(ctx, sel, draft ? [...shapes, draft] : shapes, dpr, getMosaic);
  }, [sel, shapes, draft, cap]);

  useEffect(() => {
    textareaRef.current?.focus();
  }, [text?.at]);

  // ---- 动作 ----

  const pendingText = (): Shape[] =>
    text && text.value.trim() ? [{ kind: "text", at: text.at, text: text.value }] : [];

  const commitText = () => {
    const extra = pendingText();
    if (extra.length) setShapes([...shapes, ...extra]);
    setText(null);
  };

  /** 合成选区图片（物理像素）；withAnnotations=false 只取原图（OCR 用） */
  const compose = (withAnnotations = true): ImageData | null => {
    if (!cap || !sel || sel.w < 1 || sel.h < 1) return null;
    const canvas = document.createElement("canvas");
    canvas.width = sel.w;
    canvas.height = sel.h;
    const ctx = canvas.getContext("2d", { willReadFrequently: true })!;
    ctx.drawImage(cap.bitmap, sel.x, sel.y, sel.w, sel.h, 0, 0, sel.w, sel.h);
    if (withAnnotations) renderShapes(ctx, sel, [...shapes, ...pendingText()], dpr, getMosaic);
    return ctx.getImageData(0, 0, sel.w, sel.h);
  };

  /**
   * 淡出界面并执行 fn。窗口由 Rust 隐藏，隐藏后通过 shard://hidden 释放截图。
   * concurrent=true 时不等淡出就执行（贴图：边淡出边准备贴图窗口，减少等待）
   */
  const leave = async (fn: () => Promise<unknown>, concurrent = false) => {
    if (leavingRef.current) return;
    leavingRef.current = true;
    commitText();
    setLeaving(true);
    if (!concurrent) await sleep(FADE_MS);
    try {
      await fn();
    } catch (e) {
      console.error(errorText(e));
      ipc.closeOverlay();
    }
  };

  const close = () => leave(() => ipc.closeOverlay());

  const act = (fn: (img: ImageData, sel: Rect) => Promise<unknown>, concurrent = false) => {
    const img = compose();
    if (!img || !sel) return;
    const s = sel;
    leave(() => fn(img, s), concurrent);
  };
  const copy = () => act((img) => ipc.overlayCopy(img));
  const pin = () => act((img, s) => ipc.overlayPin(img, s.x, s.y), true);
  const save = () => act((img) => ipc.overlaySave(img));
  const copyText = (t: string) => leave(() => ipc.copyText(t));

  const cancelOcr = () => {
    ocrSeq.current++;
    setOcr(null);
  };

  const toggleOcr = async () => {
    if (ocr) return cancelOcr();
    commitText();
    setTool(null);
    const img = compose(false);
    if (!img) return;
    const token = ++ocrSeq.current;
    setOcr({ status: "loading" });
    try {
      const lines = await ipc.overlayOcr(img);
      if (token === ocrSeq.current) setOcr({ status: "done", lines });
    } catch (e) {
      if (token === ocrSeq.current) setOcr({ status: "error", message: errorText(e) });
    }
  };

  const chooseTool = (t: Tool | null) => {
    commitText();
    setTool(t);
    if (t) cancelOcr();
  };

  const undo = () => {
    if (text) return setText(null);
    setShapes(shapes.slice(0, -1));
  };

  // ---- 鼠标 ----

  const toImg = (e: { clientX: number; clientY: number }): Pt => ({
    x: clamp(e.clientX * dpr, 0, bounds.w),
    y: clamp(e.clientY * dpr, 0, bounds.h),
  });

  const cursorAt = (p: Pt): string => {
    if (!sel) return "crosshair";
    if (tool) return contains(sel, p) ? (tool === "text" ? "text" : "crosshair") : "default";
    const h = hitHandle(sel, p, HANDLE_TOL * dpr);
    if (h) return HANDLE_CURSOR[h];
    if (contains(sel, p)) return "move";
    return shapes.length ? "default" : "crosshair";
  };

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (!cap || e.button !== 0 || leavingRef.current) return;
    if ((e.target as Element).closest(".text-layer, .text-input")) return;
    const p = toImg(e);
    // 正在输入文字时，点别处只是结束输入
    if (text) return commitText();

    if (tool && sel) {
      if (!contains(sel, p)) return;
      if (tool === "text") return setText({ at: p, value: "" });
      e.currentTarget.setPointerCapture(e.pointerId);
      setDraft(tool === "pen" || tool === "mosaic" ? { kind: tool, points: [p] } : { kind: tool, a: p, b: p });
      drag.current = { kind: "draw" };
      return;
    }

    if (ocr) cancelOcr();
    if (sel) {
      const handle = hitHandle(sel, p, HANDLE_TOL * dpr);
      if (handle) drag.current = { kind: "resize", handle, start: p, orig: sel };
      else if (contains(sel, p)) drag.current = { kind: "move", start: p, orig: sel };
      // 已经有标注时不允许重新框选，免得误点丢掉标注（右键可以重选）
      else if (!shapes.length) {
        drag.current = { kind: "create", start: p };
        setSel(null);
      } else return;
    } else {
      drag.current = { kind: "create", start: p };
    }
    e.currentTarget.setPointerCapture(e.pointerId);
    setDragging(true);
  };

  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (!cap || leavingRef.current) return;
    const p = toImg(e);
    const d = drag.current;
    if (!d) return setCursor(cursorAt(p));
    switch (d.kind) {
      case "create":
        setSel(rectFrom(d.start, p, bounds));
        break;
      case "move":
        setSel(moveRect(d.orig, p.x - d.start.x, p.y - d.start.y, bounds));
        break;
      case "resize":
        setSel(resizeRect(d.orig, d.handle, p.x - d.start.x, p.y - d.start.y, bounds));
        break;
      case "draw": {
        if (!sel) break;
        // 合并事件里的中间点，快速划线时笔迹更顺
        const pts = (e.nativeEvent.getCoalescedEvents?.() ?? [e.nativeEvent]).map((ev) => clampPt(toImg(ev), sel));
        setDraft((prev) => {
          if (!prev) return prev;
          if (prev.kind === "pen" || prev.kind === "mosaic") return { ...prev, points: [...prev.points, ...pts] };
          if (prev.kind === "arrow" || prev.kind === "spotlight") return { ...prev, b: pts[pts.length - 1] };
          return prev;
        });
        break;
      }
    }
  };

  const onPointerUp = () => {
    const d = drag.current;
    drag.current = null;
    setDragging(false);
    if (!d) return;
    if (d.kind === "draw") {
      if (draft && isMeaningful(draft, dpr)) setShapes([...shapes, draft]);
      setDraft(null);
    } else if (d.kind === "create" && (!sel || sel.w < MIN_SEL || sel.h < MIN_SEL)) {
      setSel(null);
    }
  };

  const onMouseDown = (e: React.MouseEvent) => {
    // 文字工具：别让这次点击把焦点从输入框抢走
    if (tool === "text" || text) e.preventDefault();
  };

  const onDoubleClick = (e: React.MouseEvent) => {
    if (!sel || tool || ocr) return;
    if (contains(sel, toImg(e))) copy();
  };

  const onContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    if (drag.current || leavingRef.current) return;
    if (sel) {
      resetEditing();
      setCursor("crosshair");
    } else close();
  };

  // ---- 键盘（处理函数每次渲染更新，监听只挂一次）----

  const onKey = useRef<(e: KeyboardEvent) => void>(() => {});
  onKey.current = (e) => {
    if (!cap || leavingRef.current) return;
    const key = e.key.toLowerCase();
    if (key === "escape") {
      e.preventDefault();
      if (text) setText(null);
      else close();
      return;
    }
    if (e.target instanceof HTMLTextAreaElement) {
      if (key === "enter" && !e.shiftKey) {
        e.preventDefault();
        commitText();
      }
      return;
    }
    if (key === "enter" && sel) {
      e.preventDefault();
      copy();
    } else if (e.ctrlKey && key === "z") {
      e.preventDefault();
      undo();
    } else if (e.ctrlKey && key === "c") {
      e.preventDefault();
      const t = selectedText(textLayerRef.current);
      if (t) copyText(t);
      else if (sel) copy();
    }
  };
  useEffect(() => {
    const handler = (e: KeyboardEvent) => onKey.current(e);
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // ---- 渲染 ----

  if (!cap) return <div className="overlay" />;

  const vw = cap.width / dpr;
  const vh = cap.height / dpr;
  const css = (r: Rect): CSSProperties => ({ left: r.x / dpr, top: r.y / dpr, width: r.w / dpr, height: r.h / dpr });
  const selCss = sel ? css(sel) : undefined;
  const ready = sel && sel.w >= MIN_SEL && sel.h >= MIN_SEL;
  const anchor: Anchor | null = sel
    ? { left: sel.x / dpr, top: sel.y / dpr, right: (sel.x + sel.w) / dpr, bottom: (sel.y + sel.h) / dpr }
    : null;
  const showHandles = !tool && !ocr && !dragging;

  return (
    <div
      className="overlay"
      style={{ cursor }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onMouseDown={onMouseDown}
      onDoubleClick={onDoubleClick}
      onContextMenu={onContextMenu}
    >
      <canvas ref={baseRef} className="absolute top-0 left-0" style={{ width: vw, height: vh }} />
      {sel && <canvas ref={annotRef} className="pointer-events-none absolute" style={selCss} />}

      {/* 界面层：每次截图淡入，退出时淡出 */}
      <div key={cap.session} className={`ui ${leaving ? "ui-leaving" : ""}`}>
        {anchor ? (
          <>
            <div className="mask" style={{ left: 0, top: 0, width: vw, height: anchor.top }} />
            <div className="mask" style={{ left: 0, top: anchor.bottom, width: vw, height: Math.max(0, vh - anchor.bottom) }} />
            <div className="mask" style={{ left: 0, top: anchor.top, width: anchor.left, height: anchor.bottom - anchor.top }} />
            <div
              className="mask"
              style={{ left: anchor.right, top: anchor.top, width: Math.max(0, vw - anchor.right), height: anchor.bottom - anchor.top }}
            />
          </>
        ) : (
          <div className="mask" style={{ inset: 0 }} />
        )}
        {sel && (
          <div className="sel-frame" style={selCss}>
            {showHandles && CORNERS.map((c) => <div key={c} className={`corner corner-${c}`} />)}
            {showHandles && sel.w / dpr > 60 && sel.h / dpr > 60 && EDGES.map((e) => <div key={e} className={`edge edge-${e}`} />)}
          </div>
        )}
        {sel && anchor && (
          <div className="size-label" style={{ left: anchor.left, top: anchor.top >= 30 ? anchor.top - 30 : anchor.top + 6 }}>
            {sel.w} <span className="opacity-60">×</span> {sel.h}
          </div>
        )}
        {ocr?.status === "done" && sel && <TextLayer ref={textLayerRef} lines={ocr.lines} scale={1 / dpr} style={selCss} />}
        {text && (
          <textarea
            ref={textareaRef}
            className="text-input"
            value={text.value}
            rows={1}
            spellCheck={false}
            onChange={(e) => setText({ ...text, value: e.target.value })}
            style={{
              left: text.at.x / dpr - TEXT_PAD,
              top: text.at.y / dpr - TEXT_PAD,
              font: `500 ${TEXT_SIZE}px/${TEXT_LINE_HEIGHT} ${TEXT_FONT}`,
              color: COLOR,
              borderColor: COLOR,
            }}
          />
        )}
        {ready && anchor && !dragging && (
          <Toolbar
            anchor={anchor}
            tool={tool}
            onTool={chooseTool}
            canUndo={shapes.length > 0 || !!text}
            onUndo={undo}
            ocr={ocr}
            onOcr={toggleOcr}
            onCopyText={() => ocr?.status === "done" && copyText(ocr.lines.map((l) => l.text).join("\n"))}
            onPin={pin}
            onSave={save}
            onClose={close}
            onCopy={copy}
          />
        )}
      </div>
    </div>
  );
}
