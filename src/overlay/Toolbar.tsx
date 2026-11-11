import { type ComponentType, useLayoutEffect, useRef } from "react";
import {
  Check,
  Copy,
  Grid3x3,
  type LucideProps,
  MoveUpRight,
  Pencil,
  Pin,
  ScanText,
  Spotlight,
  Type,
  Undo2,
  Download,
  X,
} from "lucide-react";
import type { OcrLine } from "../lib/ipc";
import type { Tool } from "./annotate";

export type OcrState = { status: "loading" } | { status: "done"; lines: OcrLine[] } | { status: "error"; message: string };

/** 选区在视口里的位置（CSS 像素） */
export interface Anchor {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

const TOOLS: { tool: Tool; icon: ComponentType<LucideProps>; label: string }[] = [
  { tool: "spotlight", icon: Spotlight, label: "聚光灯" },
  { tool: "arrow", icon: MoveUpRight, label: "箭头" },
  { tool: "pen", icon: Pencil, label: "铅笔" },
  { tool: "text", icon: Type, label: "文字" },
  { tool: "mosaic", icon: Grid3x3, label: "马赛克" },
];

const GAP = 8;
const MARGIN = 4;

interface Props {
  anchor: Anchor;
  tool: Tool | null;
  onTool: (t: Tool | null) => void;
  canUndo: boolean;
  onUndo: () => void;
  ocr: OcrState | null;
  onOcr: () => void;
  onCopyText: () => void;
  onPin: () => void;
  onSave: () => void;
  onClose: () => void;
  onCopy: () => void;
}

function Button({
  icon: Icon,
  label,
  active,
  disabled,
  busy,
  onClick,
}: {
  icon: ComponentType<LucideProps>;
  label: string;
  active?: boolean;
  disabled?: boolean;
  busy?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={active}
      disabled={disabled}
      onClick={onClick}
      className={`tb-btn ${active ? "tb-btn-active" : ""} ${busy ? "animate-pulse" : ""}`}
    >
      <Icon size={18} strokeWidth={1.9} />
    </button>
  );
}

const Divider = () => <div className="mx-1 h-5 w-px bg-[var(--divider)]" />;

/** 选区下方右对齐；下面放不下放上方；都放不下放进选区底部 */
export function Toolbar(props: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const { anchor, tool, ocr } = props;

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const w = el.offsetWidth;
    const h = el.offsetHeight;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let top = anchor.bottom + GAP;
    if (top + h > vh - MARGIN) top = anchor.top - GAP - h;
    if (top < MARGIN) top = Math.min(anchor.bottom, vh) - GAP - h;
    const left = Math.min(Math.max(anchor.right - w, MARGIN), vw - w - MARGIN);
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
  });

  return (
    <div ref={ref} className="toolbar" onPointerDown={(e) => e.stopPropagation()}>
      {TOOLS.map((t) => (
        <Button
          key={t.tool}
          icon={t.icon}
          label={t.label}
          active={tool === t.tool}
          onClick={() => props.onTool(tool === t.tool ? null : t.tool)}
        />
      ))}
      <Divider />
      <Button icon={Undo2} label="撤销 (Ctrl+Z)" disabled={!props.canUndo} onClick={props.onUndo} />
      <Divider />
      <Button
        icon={ScanText}
        label={ocr?.status === "done" ? "隐藏识别结果" : "识别文字"}
        active={ocr?.status === "done"}
        busy={ocr?.status === "loading"}
        onClick={props.onOcr}
      />
      {ocr?.status === "done" &&
        (ocr.lines.length ? (
          <button type="button" className="tb-text" onClick={props.onCopyText} title="复制全部文字">
            <Copy size={15} strokeWidth={1.9} />
            复制全部文字
          </button>
        ) : (
          <span className="tb-note">未识别到文字</span>
        ))}
      {ocr?.status === "error" && (
        <span className="tb-note tb-error" title={ocr.message}>
          {ocr.message}
        </span>
      )}
      <Button icon={Pin} label="贴到桌面" onClick={props.onPin} />
      <Button icon={Download} label="保存" onClick={props.onSave} />
      <Divider />
      <Button icon={X} label="退出 (Esc)" onClick={props.onClose} />
      <Button icon={Check} label="复制 (Enter)" onClick={props.onCopy} />
    </div>
  );
}
