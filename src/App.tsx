import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api, formatBytes, formatCount, onProgress, type CleanReport, type Environment, type Preview, type Target } from "./api";
import { fileLabel, useI18n } from "./i18n";
import { Console, type LogLine } from "./components/Console";
import { GroupList, type GroupView } from "./components/GroupList";
import { Titlebar } from "./components/Titlebar";

export default function App() {
  const { t, lang } = useI18n();
  const [env, setEnv] = useState<Environment | null>(null);
  const [targets, setTargets] = useState<Target[]>([]);
  const [selection, setSelection] = useState<Record<string, boolean>>({});
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [lines, setLines] = useState<LogLine[]>([]);
  const [phase, setPhase] = useState<"idle" | "scan" | "preview" | "clean">("idle");
  const [error, setError] = useState("");
  const [report, setReport] = useState<CleanReport | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [visibleFiles, setVisibleFiles] = useState(100);
  const [path, setPath] = useState("");
  const [progress, setProgress] = useState(0);
  const lock = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const reviewButton = useRef<HTMLButtonElement>(null);
  const lineId = useRef(0);
  const busy = phase !== "idle";
  const native = isTauri();
  const log = useCallback((level: LogLine["level"], msg: string) => {
    const line = { id: ++lineId.current, level, msg, time: new Date().toLocaleTimeString("en-GB") };
    setLines(previous => [...previous.slice(-399), line]);
  }, []);
  const fail = (failure: unknown) => { setError(String(failure)); log("err", String(failure)); };

  useEffect(() => {
    if (!native) return;
    let disposed = false;
    let remove: (() => void) | undefined;
    void onProgress(event => {
      if (!disposed) setProgress(event.total ? Math.round(event.index / event.total * 100) : 0);
    }).then(unlisten => { if (disposed) unlisten(); else remove = unlisten; }).catch(e => log("err", String(e)));
    return () => { disposed = true; remove?.(); };
  }, [native, log]);

  useEffect(() => {
    if (preview && !dialog.current?.open) { dialog.current?.showModal(); setVisibleFiles(100); }
    if (!preview && dialog.current?.open) { dialog.current.close(); reviewButton.current?.focus(); }
  }, [preview]);

  const scan = async (customPath = path) => {
    if (lock.current || !native) return;
    lock.current = true; setPhase("scan"); setError(""); setReport(null); setPreview(null); setProgress(0);
    try {
      const snapshot = await api.snapshot(customPath || null);
      setEnv(snapshot.environment);
      setTargets(snapshot.targets);
      const result = await api.scan();
      setTargets(result.targets);
      setSelection(previous => Object.fromEntries(result.targets.map(target => [target.id, target.exists && (previous[target.id] ?? target.defaultOn)])));
      log("ok", `${t("ready")}: ${formatBytes(result.totalBytes)}`);
      for (const item of result.errors) log("warn", `${item.path}: ${item.message}`);
    } catch (e) { fail(e); }
    finally { lock.current = false; setPhase("idle"); }
  };
  const selected = useMemo(() => targets.filter(target => target.exists && selection[target.id]), [targets, selection]);
  const selectedBytes = selected.reduce((sum, target) => sum + target.size, 0);
  const groups = useMemo<GroupView[]>(() => ["logs", "caches", "crash", "launcher", "updates", "account"].map(id => {
    const items = targets.filter(target => target.group === id);
    return { id, targets: items, bytes: items.reduce((n, t) => n + t.size, 0), files: items.reduce((n, t) => n + t.files, 0), exists: items.some(t => t.exists) };
  }).filter(g => g.targets.length), [targets]);

  const review = async () => {
    if (lock.current || !selected.length) return;
    lock.current = true; setPhase("preview"); setError("");
    try {
      const result = await api.preview(selected.map(target => target.id));
      setPreview(result);
      log("info", `${t("preview.title")}: ${result.files.length}, ${formatBytes(result.bytes)}`);
    } catch (e) { fail(e); }
    finally { lock.current = false; setPhase("idle"); }
  };
  const clean = async () => {
    if (lock.current || !preview?.files.length) return;
    lock.current = true; setPhase("clean"); setError(""); setProgress(0);
    try {
      const result = await api.clean(preview.planId);
      setReport(result); setTargets(result.targets); setPreview(null);
      log(result.errors.length ? "warn" : "ok", `${t("done")}: ${formatBytes(result.bytes)}, ${t("skipped")}: ${result.skipped}`);
      for (const item of result.errors) log("warn", `${item.path}: ${item.message}`);
    } catch (e) { fail(e); }
    finally { lock.current = false; setPhase("idle"); }
  };
  const errorText = error ? t(`error.${error}`, t("error.generic")) : "";
  const status = busy ? t(phase === "scan" ? "scanning" : phase === "preview" ? "preparing" : "cleaning") : targets.some(t => t.scanned) ? t("ready") : t("initial");
  const setPreset = (mode: "all" | "none" | "logs") => setSelection(Object.fromEntries(targets.map(t => [t.id, t.exists && mode !== "none" && (mode === "all" || t.group === "logs")])));

  return <div className="app">
    <Titlebar />
    <main className="content" aria-label={t("nav.clean")}>
      {!native && <p className="notice" role="status">{t("preview.browser")}</p>}
      {error && !preview && <p className="notice error" role="alert">{errorText}</p>}
      <details className="location">
        <summary><span>{t("path.label")}</span><code>{env?.gameRoots[0] ?? t("path.auto")}</code></summary>
        <form className="path-form" onSubmit={e => { e.preventDefault(); void scan(); }}><label htmlFor="game-path">{t("path.help")}</label><input id="game-path" value={path} disabled={busy} placeholder={t("path.placeholder")} onChange={e => setPath(e.target.value)} spellCheck={false} /><div><button className="button" disabled={busy || !native}>{t("path.apply")}</button><button type="button" className="button" disabled={busy || !native} onClick={() => { setPath(""); void scan(""); }}>{t("path.auto")}</button></div></form>
        {env && !env.isAdmin && <button className="text-button" disabled={busy} onClick={() => { void api.elevate().catch(fail); }}>{t("status.admin.action")}</button>}
      </details>
      {report && <div className="report" role="status"><span>{t("removed")}: {formatBytes(report.bytes)}. {t("skipped")}: {report.skipped}</span><button aria-label={t("dismiss")} onClick={() => setReport(null)}>×</button></div>}
      <div className="selection-bar"><button disabled={busy || !targets.length} onClick={() => setPreset(selected.length === targets.filter(t => t.exists).length ? "none" : "all")}>{t(selected.length && selected.length === targets.filter(t => t.exists).length ? "groups.none" : "groups.all")}</button><button disabled={busy || !targets.length} onClick={() => setPreset("logs")}>{t("groups.safe")}</button></div>
      {env ? <GroupList groups={groups} selection={selection} expanded={expanded} busy={busy}
        onToggleTarget={id => setSelection(p => ({ ...p, [id]: !p[id] }))}
        onToggleGroup={(id, value) => setSelection(p => ({ ...p, ...Object.fromEntries(targets.filter(t => t.group === id).map(t => [t.id, t.exists && value])) }))}
        onToggleExpanded={id => setExpanded(p => ({ ...p, [id]: !p[id] }))}
        onReveal={id => { void api.reveal(id).catch(fail); }} /> : <div className="empty">{t("initial")}</div>}
      {!!env?.running.length && <p className="notice">{t("status.running")}</p>}
      <details className="activity"><summary>{t("nav.activity")}</summary><Console lines={lines} onClear={() => setLines([])} /></details>
    </main>
    <footer className="footer"><div role="status">{busy ? status : `${t("selected")}: ${formatBytes(selectedBytes)}`}{busy && <progress aria-label={status} max={100} value={progress || undefined} />}</div><div className="footer-actions"><button className="button" disabled={busy || !native} onClick={() => void scan()}>{t("scan")}</button><button ref={reviewButton} className="button primary" disabled={busy || !selected.length || !native} onClick={() => void review()}>{t("review")}</button></div></footer>
    <dialog ref={dialog} className="review-dialog" aria-labelledby="review-heading" onCancel={e => { if (busy) e.preventDefault(); else setPreview(null); }}>
      {preview && <><h2 id="review-heading">{t("preview.title")}</h2><p>{t("preview.note")}</p><div className="preview-total">{formatBytes(preview.bytes)} <span>{formatCount(preview.files.length)} {fileLabel(preview.files.length, lang)}</span></div>
        {error && <p className="notice error" role="alert">{errorText}</p>}
        <div className="preview-list"><table><thead><tr><th scope="col">{t("file.path")}</th><th scope="col">{t("file.size")}</th></tr></thead><tbody>{preview.files.slice(0, visibleFiles).map(file => <tr key={file.path}><td>{file.path}</td><td>{formatBytes(file.size)}</td></tr>)}</tbody></table>
          {!preview.files.length && <p>{t("preview.empty")}</p>}
          {visibleFiles < preview.files.length && <button className="button" onClick={() => setVisibleFiles(n => n + 100)}>{t("more")}</button>}
          {preview.errors.length > 0 && <details><summary>{t("skipped")}: {preview.errors.length}</summary>{preview.errors.map((item, i) => <p key={i}>{item.path}: {item.message}</p>)}</details>}
        </div><div className="dialog-actions"><button autoFocus className="button" disabled={busy} onClick={() => setPreview(null)}>{t("cancel")}</button><button className="button primary" disabled={busy || !preview.files.length} onClick={() => void clean()}>{busy ? t("cleaning") : t("confirm")}</button></div></>}
    </dialog>
  </div>;
}
