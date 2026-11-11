import { type CSSProperties, forwardRef, useMemo } from "react";
import type { OcrLine, OcrWord } from "../lib/ipc";

const FONT = '"Segoe UI", "Microsoft YaHei UI", sans-serif';
let measureCtx: CanvasRenderingContext2D | null = null;

function measure(text: string, size: number): number {
  measureCtx ??= document.createElement("canvas").getContext("2d")!;
  measureCtx.font = `${size}px ${FONT}`;
  return measureCtx.measureText(text).width;
}

/** 每个词一个透明 span，盖在原文位置上，横向拉伸到和原文一样宽，这样框选范围和看到的文字对得上 */
function wordStyle(w: OcrWord, scale: number): CSSProperties {
  const h = w.h * scale;
  const size = h * 0.9;
  const natural = measure(w.text, size) || 1;
  return {
    left: w.x * scale,
    top: w.y * scale,
    height: h,
    lineHeight: `${h}px`,
    fontSize: size,
    transform: `scaleX(${(w.w * scale) / natural})`,
  };
}

interface Props {
  lines: OcrLine[];
  /** 图片像素 → CSS 像素 */
  scale: number;
  /** 不显示识别出的文字框，只在选中时高亮 */
  quiet?: boolean;
  style?: CSSProperties;
}

/** OCR 文字层：可以框选、复制。选中文本用 ocr/text.ts 的 selectedText 读取 */
export const TextLayer = forwardRef<HTMLDivElement, Props>(function TextLayer({ lines, scale, quiet, style }, ref) {
  const words = useMemo(
    () => lines.map((line) => line.words.map((w) => ({ text: w.text, style: wordStyle(w, scale) }))),
    [lines, scale],
  );
  return (
    <div ref={ref} className={`text-layer ${quiet ? "text-layer-quiet" : ""}`} style={style}>
      {words.map((line, li) => (
        <div key={li}>
          {line.map((w, wi) => (
            <span key={wi} data-word="" data-line={li} style={w.style}>
              {w.text}
            </span>
          ))}
        </div>
      ))}
    </div>
  );
});
