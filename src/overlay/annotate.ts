import type { Pt, Rect } from "../lib/geometry";

export type Tool = "spotlight" | "arrow" | "pen" | "text" | "mosaic";

/** 标注图形。坐标是截图的物理像素（绝对坐标，选区移动时图形留在原处） */
export type Shape =
  | { kind: "spotlight"; a: Pt; b: Pt }
  | { kind: "arrow"; a: Pt; b: Pt }
  | { kind: "pen"; points: Pt[] }
  | { kind: "mosaic"; points: Pt[] }
  | { kind: "text"; at: Pt; text: string };

export const COLOR = "#f5222d";
/** 以下尺寸是 CSS 像素，绘制时乘 dpr */
export const LINE_WIDTH = 3;
export const TEXT_SIZE = 18;
export const TEXT_LINE_HEIGHT = 1.3;
export const TEXT_FONT = '"Segoe UI", "Microsoft YaHei UI", sans-serif';
const MOSAIC_BRUSH = 12; // 半径
const MOSAIC_BLOCK = 8;
const SPOTLIGHT_DIM = "rgb(0 0 0 / 0.55)";

/** 马赛克取样图：整张截图缩小再放大 */
export interface MosaicSource {
  canvas: HTMLCanvasElement;
  block: number;
}

export function makeMosaicSource(bitmap: ImageBitmap, dpr: number): MosaicSource {
  const block = Math.max(2, Math.round(MOSAIC_BLOCK * dpr));
  const canvas = document.createElement("canvas");
  canvas.width = Math.ceil(bitmap.width / block);
  canvas.height = Math.ceil(bitmap.height / block);
  const ctx = canvas.getContext("2d")!;
  ctx.imageSmoothingQuality = "high";
  ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  return { canvas, block };
}

let dimCanvas: HTMLCanvasElement | null = null;

function normRect(a: Pt, b: Pt): Rect {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(a.x - b.x), h: Math.abs(a.y - b.y) };
}

/** 笔刷覆盖区域：沿路径密集排列的圆（并集），用作裁剪路径 */
function brushPath(points: Pt[], r: number): Path2D {
  const path = new Path2D();
  const dot = (p: Pt) => {
    path.moveTo(p.x + r, p.y);
    path.arc(p.x, p.y, r, 0, Math.PI * 2);
  };
  let prev: Pt | null = null;
  for (const p of points) {
    if (prev) {
      const d = Math.hypot(p.x - prev.x, p.y - prev.y);
      const steps = Math.max(1, Math.ceil(d / (r / 3)));
      for (let i = 1; i <= steps; i++) {
        dot({ x: prev.x + ((p.x - prev.x) * i) / steps, y: prev.y + ((p.y - prev.y) * i) / steps });
      }
    } else {
      dot(p);
    }
    prev = p;
  }
  return path;
}

function drawMosaic(ctx: CanvasRenderingContext2D, points: Pt[], src: MosaicSource, dpr: number) {
  ctx.save();
  ctx.clip(brushPath(points, MOSAIC_BRUSH * dpr));
  ctx.imageSmoothingEnabled = false;
  const { canvas, block } = src;
  ctx.drawImage(canvas, 0, 0, canvas.width, canvas.height, 0, 0, canvas.width * block, canvas.height * block);
  ctx.restore();
}

/** 聚光灯：选区内所有聚光灯矩形以外的部分压暗 */
function drawSpotlights(ctx: CanvasRenderingContext2D, sel: Rect, rects: Rect[]) {
  dimCanvas ??= document.createElement("canvas");
  dimCanvas.width = sel.w;
  dimCanvas.height = sel.h;
  const d = dimCanvas.getContext("2d")!;
  d.fillStyle = SPOTLIGHT_DIM;
  d.fillRect(0, 0, sel.w, sel.h);
  d.globalCompositeOperation = "destination-out";
  // 挖洞要用不透明色，否则只去掉一部分压暗
  d.fillStyle = "#000";
  for (const r of rects) d.fillRect(r.x - sel.x, r.y - sel.y, r.w, r.h);
  ctx.drawImage(dimCanvas, sel.x, sel.y);
}

function drawArrow(ctx: CanvasRenderingContext2D, a: Pt, b: Pt, lw: number) {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy);
  if (len < 1) return;
  const ux = dx / len;
  const uy = dy / len;
  const head = Math.min(len * 0.6, lw * 5.5);
  const half = head * 0.5;
  const bx = b.x - ux * head;
  const by = b.y - uy * head;
  ctx.beginPath();
  ctx.moveTo(a.x, a.y);
  ctx.lineTo(bx + ux * 1, by + uy * 1);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(b.x, b.y);
  ctx.lineTo(bx - uy * half, by + ux * half);
  ctx.lineTo(bx + uy * half, by - ux * half);
  ctx.closePath();
  ctx.fill();
}

function drawPen(ctx: CanvasRenderingContext2D, points: Pt[], lw: number) {
  if (points.length === 1) {
    ctx.beginPath();
    ctx.arc(points[0].x, points[0].y, lw / 2, 0, Math.PI * 2);
    ctx.fill();
    return;
  }
  // 用中点做二次曲线，线条更顺
  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let i = 1; i < points.length - 1; i++) {
    const mx = (points[i].x + points[i + 1].x) / 2;
    const my = (points[i].y + points[i + 1].y) / 2;
    ctx.quadraticCurveTo(points[i].x, points[i].y, mx, my);
  }
  const last = points[points.length - 1];
  ctx.lineTo(last.x, last.y);
  ctx.stroke();
}

export function textFont(dpr: number): string {
  return `500 ${TEXT_SIZE * dpr}px ${TEXT_FONT}`;
}

function drawText(ctx: CanvasRenderingContext2D, at: Pt, text: string, dpr: number) {
  ctx.font = textFont(dpr);
  ctx.textBaseline = "middle";
  const lh = TEXT_SIZE * TEXT_LINE_HEIGHT * dpr;
  text.split("\n").forEach((line, i) => ctx.fillText(line, at.x, at.y + i * lh + lh / 2));
}

/**
 * 在 ctx 上画标注（ctx 的原点对应选区左上角）。不清空画布：
 * 显示时画在透明的标注层上，导出时画在已经画好底图的画布上，两者结果一致。
 * 层次：马赛克 → 聚光灯压暗 → 箭头/铅笔/文字（按添加顺序）。
 */
export function renderShapes(
  ctx: CanvasRenderingContext2D,
  sel: Rect,
  shapes: Shape[],
  dpr: number,
  mosaic: () => MosaicSource,
) {
  ctx.save();
  ctx.translate(-sel.x, -sel.y);

  for (const s of shapes) {
    if (s.kind === "mosaic") drawMosaic(ctx, s.points, mosaic(), dpr);
  }
  const spots = shapes.flatMap((s) => (s.kind === "spotlight" ? [normRect(s.a, s.b)] : []));
  if (spots.length) drawSpotlights(ctx, sel, spots);

  const lw = LINE_WIDTH * dpr;
  ctx.strokeStyle = COLOR;
  ctx.fillStyle = COLOR;
  ctx.lineWidth = lw;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  for (const s of shapes) {
    if (s.kind === "arrow") drawArrow(ctx, s.a, s.b, lw);
    else if (s.kind === "pen") drawPen(ctx, s.points, lw);
    else if (s.kind === "text") drawText(ctx, s.at, s.text, dpr);
  }
  ctx.restore();
}

/** 画完时是否算一个有效图形（太小的忽略） */
export function isMeaningful(s: Shape, dpr: number): boolean {
  const min = 3 * dpr;
  switch (s.kind) {
    case "spotlight":
      return Math.abs(s.a.x - s.b.x) >= min && Math.abs(s.a.y - s.b.y) >= min;
    case "arrow":
      return Math.hypot(s.a.x - s.b.x, s.a.y - s.b.y) >= min;
    case "text":
      return s.text.trim().length > 0;
    default:
      return s.points.length > 0;
  }
}
