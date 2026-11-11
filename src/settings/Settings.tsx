import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorText } from "../lib/ipc";
import { shortcutFromEvent, shortcutKeys } from "./shortcut";

interface Settings {
  hotkey: string;
  registered: boolean;
  autostart: boolean;
}

function Keys({ keys }: { keys: string[] }) {
  return (
    <span className="flex items-center gap-1">
      {keys.map((k, i) => (
        <kbd key={i} className="kbd">
          {k}
        </kbd>
      ))}
    </span>
  );
}

function HotkeyField({ value, registered, onSaved }: { value: string; registered: boolean; onSaved: () => void }) {
  const [recording, setRecording] = useState(false);
  const [preview, setPreview] = useState<string[]>([]);
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null);
  const ref = useRef<HTMLButtonElement>(null);

  const start = async () => {
    setMessage(null);
    setPreview([]);
    try {
      await invoke("suspend_hotkey", { suspended: true });
      setRecording(true);
    } catch (e) {
      setMessage({ text: errorText(e), error: true });
    }
  };

  const stop = async () => {
    setRecording(false);
    setPreview([]);
    await invoke("suspend_hotkey", { suspended: false }).catch(() => {});
    onSaved();
  };

  const save = async (hotkey: string) => {
    setRecording(false);
    setPreview([]);
    try {
      await invoke("set_hotkey", { hotkey });
      setMessage({ text: "已保存", error: false });
    } catch (e) {
      setMessage({ text: errorText(e), error: true });
    }
    await invoke("suspend_hotkey", { suspended: false }).catch(() => {});
    onSaved();
  };

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      // PrintScreen 只有 keyup，其余键在 keydown 处理
      if (e.type === "keyup" && e.code !== "PrintScreen") {
        if (!e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) setPreview([]);
        return;
      }
      if (e.code === "Escape" && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) return void stop();
      const r = shortcutFromEvent(e);
      setPreview(r.preview);
      if (r.error) setMessage({ text: r.error, error: true });
      else if (r.value) save(r.value);
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("keyup", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("keyup", onKey, true);
    };
  }, [recording]);

  return (
    <div className="flex flex-col items-end gap-1">
      <button
        ref={ref}
        type="button"
        className={`hotkey ${recording ? "hotkey-recording" : ""}`}
        onClick={() => (recording ? stop() : start())}
        onBlur={() => recording && stop()}
        title={recording ? "按 Esc 取消" : "点击后按下新的快捷键"}
      >
        {recording ? (
          preview.length ? <Keys keys={preview} /> : <span className="muted">按下新的快捷键…</span>
        ) : (
          <Keys keys={shortcutKeys(value)} />
        )}
      </button>
      <span className={`hint ${message?.error || (!registered && !message) ? "hint-error" : ""}`}>
        {message?.text ?? (registered ? "点击后按下新的组合键" : "当前快捷键未生效，可能被其他程序占用")}
      </span>
    </div>
  );
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button type="button" role="switch" aria-checked={checked} className="toggle" onClick={() => onChange(!checked)}>
      <span className="toggle-thumb" />
    </button>
  );
}

export function SettingsPage() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [autostartError, setAutostartError] = useState<string | null>(null);

  const load = () => invoke<Settings>("get_settings").then(setSettings).catch(console.error);
  useEffect(() => {
    load();
  }, []);

  const setAutostart = async (enabled: boolean) => {
    setAutostartError(null);
    try {
      await invoke("set_autostart", { enabled });
    } catch (e) {
      setAutostartError(errorText(e));
    }
    load();
  };

  if (!settings) return null;
  return (
    <main className="page">
      <section className="card">
        <div className="row">
          <div>
            <div className="label">截图快捷键</div>
          </div>
          <HotkeyField value={settings.hotkey} registered={settings.registered} onSaved={load} />
        </div>
        <div className="divider" />
        <div className="row">
          <div>
            <div className="label">开机启动</div>
            {autostartError && <div className="hint hint-error">{autostartError}</div>}
          </div>
          <Toggle checked={settings.autostart} onChange={setAutostart} />
        </div>
      </section>
    </main>
  );
}
