import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n, type Lang } from "../i18n";

export function Titlebar() {
  const { t, lang, setLang } = useI18n();
  const windowAction = (action: "minimize" | "toggleMaximize" | "close") => {
    if (isTauri()) void getCurrentWindow()[action]().catch(console.error);
  };
  return <header className="titlebar">
    <div className="brand" data-tauri-drag-region onDoubleClick={() => windowAction("toggleMaximize")}>
      <span className="brand-name" data-tauri-drag-region>WARFACE <span>LOG CLEANER</span></span>
    </div>
    <div className="lang-switch" role="group" aria-label={t("app.lang")}>
      {(["ru", "en"] as Lang[]).map(code => <button key={code} aria-pressed={lang === code} onClick={() => setLang(code)}>{code.toUpperCase()}</button>)}
    </div>
    {isTauri() && <div className="win-controls">
      <button aria-label={t("minimize")} onClick={() => windowAction("minimize")}>−</button>
      <button aria-label={t("maximize")} onClick={() => windowAction("toggleMaximize")}>□</button>
      <button className="close" aria-label={t("close")} onClick={() => windowAction("close")}>×</button>
    </div>}
  </header>;
}
