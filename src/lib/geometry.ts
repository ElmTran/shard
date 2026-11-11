/** 坐标都是截图的物理像素 */
export interface Pt {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Handle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

export const clamp = (v: number, min: number, max: number) => Math.min(Math.max(v, min), max);

/** 两点围成的矩形（取整），限制在 bounds 内 */
export function rectFrom(a: Pt, b: Pt, bounds: Rect): Rect {
  const l = clamp(Math.round(Math.min(a.x, b.x)), bounds.x, bounds.x + bounds.w);
  const t = clamp(Math.round(Math.min(a.y, b.y)), bounds.y, bounds.y + bounds.h);
  const r = clamp(Math.round(Math.max(a.x, b.x)), bounds.x, bounds.x + bounds.w);
  const btm = clamp(Math.round(Math.max(a.y, b.y)), bounds.y, bounds.y + bounds.h);
  return { x: l, y: t, w: r - l, h: btm - t };
}

export function contains(r: Rect, p: Pt): boolean {
  return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h;
}

export function clampPt(p: Pt, r: Rect): Pt {
  return { x: clamp(p.x, r.x, r.x + r.w), y: clamp(p.y, r.y, r.y + r.h) };
}

/** 鼠标落在哪个调整手柄上：四角优先，其次四条边（整条边都可以拖） */
export function hitHandle(r: Rect, p: Pt, tol: number): Handle | null {
  const nearL = Math.abs(p.x - r.x) <= tol;
  const nearR = Math.abs(p.x - (r.x + r.w)) <= tol;
  const nearT = Math.abs(p.y - r.y) <= tol;
  const nearB = Math.abs(p.y - (r.y + r.h)) <= tol;
  const inX = p.x >= r.x - tol && p.x <= r.x + r.w + tol;
  const inY = p.y >= r.y - tol && p.y <= r.y + r.h + tol;
  if (nearT && nearL) return "nw";
  if (nearT && nearR) return "ne";
  if (nearB && nearL) return "sw";
  if (nearB && nearR) return "se";
  if (nearT && inX) return "n";
  if (nearB && inX) return "s";
  if (nearL && inY) return "w";
  if (nearR && inY) return "e";
  return null;
}

export function resizeRect(orig: Rect, h: Handle, dx: number, dy: number, bounds: Rect): Rect {
  let l = orig.x;
  let t = orig.y;
  let r = orig.x + orig.w;
  let b = orig.y + orig.h;
  if (h.includes("w")) l += dx;
  if (h.includes("e")) r += dx;
  if (h.includes("n")) t += dy;
  if (h.includes("s")) b += dy;
  return rectFrom({ x: l, y: t }, { x: r, y: b }, bounds);
}

export function moveRect(orig: Rect, dx: number, dy: number, bounds: Rect): Rect {
  return {
    ...orig,
    x: Math.round(clamp(orig.x + dx, bounds.x, bounds.x + bounds.w - orig.w)),
    y: Math.round(clamp(orig.y + dy, bounds.y, bounds.y + bounds.h - orig.h)),
  };
}

export const HANDLE_CURSOR: Record<Handle, string> = {
  n: "ns-resize",
  s: "ns-resize",
  e: "ew-resize",
  w: "ew-resize",
  ne: "nesw-resize",
  sw: "nesw-resize",
  nw: "nwse-resize",
  se: "nwse-resize",
};
