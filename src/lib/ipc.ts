import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { OcrLine } from "./bindings/OcrLine";
import type { OcrWord } from "./bindings/OcrWord";

export type { OcrLine, OcrWord };

/** 与 Rust 之间传图片的格式：[宽 u32 LE][高 u32 LE][RGBA...] */
export interface RawImage {
  width: number;
  height: number;
  data: Uint8ClampedArray<ArrayBuffer>;
}

export function decodeImage(buf: ArrayBuffer): RawImage {
  const view = new DataView(buf);
  const width = view.getUint32(0, true);
  const height = view.getUint32(4, true);
  return { width, height, data: new Uint8ClampedArray(buf, 8, width * height * 4) };
}

export function encodeImage(img: ImageData): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(8 + img.data.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, img.width, true);
  view.setUint32(4, img.height, true);
  out.set(img.data, 8);
  return out;
}

export function toBitmap(img: RawImage): Promise<ImageBitmap> {
  return createImageBitmap(new ImageData(img.data, img.width, img.height));
}

export interface CaptureMeta {
  kind: "capture";
  width: number;
  height: number;
  /** 第几次截图，用来匹配“已隐藏”通知 */
  session: number;
}

/** 截图窗口被 Rust 隐藏了（payload 是那次截图的 session） */
export function onHidden(cb: (session: number) => void): Promise<UnlistenFn> {
  return listen<number>("shard://hidden", (e) => cb(e.payload));
}

export const FADE_MS = 120;
export const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/**
 * 截图像素通过 WebView2 SharedBuffer 送过来（自上而下的 BGRX）。
 * 用 VideoFrame 让 GPU 做 BGRX→RGB 转换，转成 ImageBitmap 后立刻释放共享内存
 */
export function onCapture(cb: (bitmap: ImageBitmap, meta: CaptureMeta) => void): () => void {
  const webview = window.chrome?.webview;
  if (!webview) return () => {};
  const listener = async (e: SharedBufferReceivedEvent) => {
    const meta = e.additionalData as CaptureMeta;
    const buffer = e.getBuffer();
    let frame: VideoFrame | null = null;
    try {
      frame = new VideoFrame(new Uint8Array(buffer), {
        format: "BGRX",
        codedWidth: meta.width,
        codedHeight: meta.height,
        timestamp: 0,
      });
      webview.releaseBuffer(buffer);
      const bitmap = await createImageBitmap(frame);
      cb(bitmap, meta);
    } catch (err) {
      console.error(err);
      invoke("close_overlay");
    } finally {
      frame?.close();
    }
  };
  webview.addEventListener("sharedbufferreceived", listener);
  return () => webview.removeEventListener("sharedbufferreceived", listener);
}

const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

/**
 * 显示当前窗口：隐身显示 → 等新内容真正合成出来 → 取消隐身并聚焦（见 platform/window.rs）。
 * 等帧加了超时兜底，万一隐身期间不出帧也不会卡住。
 */
export async function presentWindow(): Promise<void> {
  await invoke("present");
  await Promise.race([nextFrame().then(nextFrame), sleep(80)]);
  await invoke("reveal");
}

export const ipc = {
  closeOverlay: () => invoke<void>("close_overlay"),
  overlayCopy: (img: ImageData) => invoke<void>("overlay_copy", encodeImage(img)),
  overlayPin: (img: ImageData, x: number, y: number) =>
    invoke<void>("overlay_pin", encodeImage(img), { headers: { "x-pos": `${x},${y}` } }),
  /** 返回是否真的保存了（取消对话框为 false） */
  overlaySave: (img: ImageData) => invoke<boolean>("overlay_save", encodeImage(img)),
  overlayOcr: (img: ImageData) => invoke<OcrLine[]>("overlay_ocr", encodeImage(img)),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  getPinImage: () => invoke<ArrayBuffer>("get_pin_image"),
  pinCopy: () => invoke<void>("pin_copy"),
  pinSave: () => invoke<boolean>("pin_save"),
  pinOcr: () => invoke<OcrLine[]>("pin_ocr"),
  closePin: () => invoke<void>("close_pin"),
};

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
