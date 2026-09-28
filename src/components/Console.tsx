import { useEffect, useRef } from "react";

import type { LogLevel } from "../api";
import { useI18n } from "../i18n";

export interface LogLine {
  id: number;
  time: string;
  level: LogLevel;
  msg: string;
}

export function Console({ lines, onClear }: { lines: LogLine[]; onClear: () => void }) {
  const { t } = useI18n();
  const bodyRef = useRef<HTMLDivElement>(null);
  const pinnedRef = useRef(true);

  useEffect(() => {
    const element = bodyRef.current;
    if (!element || !pinnedRef.current) return;
    element.scrollTop = element.scrollHeight;
  }, [lines]);

  return (
    <section className="panel">
      <div className="panel-head">
        <button type="button" className="mini-button" onClick={onClear} disabled={lines.length === 0}>
          {t("console.clear")}
        </button>
      </div>
      <div
        className="panel-body"
        ref={bodyRef}
        onScroll={(event) => {
          const element = event.currentTarget;
          pinnedRef.current = element.scrollHeight - element.scrollTop - element.clientHeight < 48;
        }}
      >
        {lines.length === 0 ? (
          <div className="console-empty">{t("console.empty")}</div>
        ) : (
          <div className="console">
            {lines.map((line) => (
              <div key={line.id} className={`log-line ${line.level}`}>
                <span className="time">{line.time}</span>
                <span className="msg">{line.msg}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
