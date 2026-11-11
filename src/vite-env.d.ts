/// <reference types="vite/client" />

/** WebView2 的 SharedBuffer 接口（截图像素从 Rust 直接共享过来） */
interface SharedBufferReceivedEvent extends Event {
  getBuffer(): ArrayBuffer;
  additionalData: unknown;
}

interface Window {
  chrome?: {
    webview?: {
      addEventListener(type: "sharedbufferreceived", listener: (e: SharedBufferReceivedEvent) => void): void;
      removeEventListener(type: "sharedbufferreceived", listener: (e: SharedBufferReceivedEvent) => void): void;
      releaseBuffer(buffer: ArrayBuffer): void;
    };
  };
}
