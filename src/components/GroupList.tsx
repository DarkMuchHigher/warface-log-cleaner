import { useEffect, useRef } from "react";
import { formatBytes, formatCount, type Target } from "../api";
import { fileLabel, useI18n } from "../i18n";

export interface GroupView { id: string; targets: Target[]; bytes: number; files: number; exists: boolean }
interface Props {
  groups: GroupView[];
  selection: Record<string, boolean>;
  expanded: Record<string, boolean>;
  busy: boolean;
  onToggleTarget: (id: string) => void;
  onToggleGroup: (id: string, value: boolean) => void;
  onToggleExpanded: (id: string) => void;
  onReveal: (id: string) => void;
}
function Check({ checked, mixed = false, disabled, label, onChange }: {
  checked: boolean; mixed?: boolean; disabled: boolean; label: string; onChange: () => void;
}) {
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => { if (ref.current) ref.current.indeterminate = mixed; }, [mixed]);
  return <input ref={ref} className="check" type="checkbox" checked={checked} disabled={disabled} aria-label={label} onChange={onChange} />;
}
export function GroupList({ groups, selection, expanded, busy, onToggleTarget, onToggleGroup, onToggleExpanded, onReveal }: Props) {
  const { t, lang } = useI18n();
  if (!groups.length) return <div className="empty">{t("groups.empty")}</div>;
  return <div className="group-list">{groups.map(group => {
    const available = group.targets.filter(target => target.exists);
    const count = available.filter(target => selection[target.id]).length;
    const all = count > 0 && count === available.length;
    const title = t(`group.${group.id}.title`);
    const open = !!expanded[group.id];
    return <section className="group" key={group.id}>
      <div className="group-head">
        <Check checked={all} mixed={count > 0 && !all} disabled={busy || !available.length} label={title} onChange={() => onToggleGroup(group.id, !all)} />
        <button className="group-expand" aria-expanded={open} aria-controls={`group-${group.id}`} onClick={() => onToggleExpanded(group.id)}>
          <span><strong>{title}</strong></span>
          <span className="group-size"><b>{group.targets.some(t => t.scanned) ? formatBytes(group.bytes) : "—"}</b><small>{formatCount(group.files)} {fileLabel(group.files, lang)}</small></span>
          <span aria-hidden="true" className="chevron">{open ? "−" : "+"}</span>
        </button>
      </div>
      <div id={`group-${group.id}`} hidden={!open} className="targets"><p className="group-note">{t(`group.${group.id}.desc`)}</p>{group.targets.map(target => {
        const name = t(`target.${target.id.split("@")[0]}`, target.id.split("@")[0]);
        return <div className="target" key={target.id}>
          <Check checked={!!selection[target.id] && target.exists} disabled={busy || !target.exists} label={name} onChange={() => onToggleTarget(target.id)} />
          <div className="target-main"><div className="target-name">{name}</div><div className="target-path" title={target.path}>{target.path}</div></div>
          <span className="target-size">{target.scanned ? formatBytes(target.size) : "—"}</span>
          <button className="reveal" aria-label={`${t("status.reveal")}: ${name}`} title={t("status.reveal")} disabled={busy || !target.exists} onClick={() => onReveal(target.id)}>…</button>
        </div>;
      })}</div>
    </section>;
  })}</div>;
}
