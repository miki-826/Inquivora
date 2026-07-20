import { useEffect, useRef, useState } from "react";
import { useTaskStore } from "../../stores/useTaskStore";
import "../tasks/taskColors.css";
import "./editorEnhancements.css";
import {
  PRIORITY_LABELS,
  TASK_COLOR_LABELS,
  TASK_COLOR_VALUES,
  buildDueAtUtc,
  type TaskColor,
  type TaskPriority,
} from "../tasks/taskModel";

export type SelectionTaskDraft = {
  text: string;
  filePath: string;
};

function initialTitle(text: string): string {
  return text.split(/\r?\n/).find((line) => line.trim())?.trim().slice(0, 200) ?? text.slice(0, 200);
}

export function SelectionTaskDialog({
  draft,
  onClose,
}: {
  draft: SelectionTaskDraft;
  onClose: () => void;
}) {
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const createTask = useTaskStore((state) => state.createTask);
  const [title, setTitle] = useState(() => initialTitle(draft.text));
  const [dueDate, setDueDate] = useState("");
  const [dueTime, setDueTime] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("medium");
  const [color, setColor] = useState<TaskColor>("blue");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const dialog = dialogRef.current;
    const first = dialog?.querySelector<HTMLInputElement>("input[type='text']");
    first?.focus();
    first?.select();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
      if (event.key !== "Tab" || !dialog) return;
      const focusable = Array.from(
        dialog.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), select:not(:disabled)"),
      );
      if (focusable.length === 0) return;
      const firstItem = focusable[0];
      const lastItem = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === firstItem) {
        event.preventDefault();
        lastItem.focus();
      } else if (!event.shiftKey && document.activeElement === lastItem) {
        event.preventDefault();
        firstItem.focus();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!title.trim()) {
      setError("タイトルを入力してください");
      return;
    }
    setSaving(true);
    setError(null);
    const task = await createTask({
      title: title.trim(),
      description: draft.text.trim() === title.trim() ? null : draft.text.trim(),
      dueAtUtc: buildDueAtUtc(dueDate, dueTime),
      priority,
      color,
      linkedFilePath: draft.filePath,
    });
    setSaving(false);
    if (task) onClose();
    else setError(useTaskStore.getState().error ?? "タスクを作成できませんでした");
  };

  return (
    <div className="modal-backdrop" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <div
        ref={dialogRef}
        className="modal selection-task-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="selection-task-title"
      >
        <form onSubmit={(event) => void submit(event)}>
          <h2 id="selection-task-title">選択範囲をタスクにする</h2>
          <label>
            タイトル
            <input type="text" value={title} maxLength={500} onChange={(event) => setTitle(event.target.value)} />
          </label>
          <div className="selection-task-dialog__grid">
            <label>
              期日
              <input type="date" value={dueDate} onChange={(event) => setDueDate(event.target.value)} />
            </label>
            <label>
              時刻
              <input type="time" value={dueTime} disabled={!dueDate} onChange={(event) => setDueTime(event.target.value)} />
            </label>
            <label>
              優先度
              <select value={priority} onChange={(event) => setPriority(event.target.value as TaskPriority)}>
                {(Object.keys(PRIORITY_LABELS) as TaskPriority[]).map((value) => (
                  <option key={value} value={value}>{PRIORITY_LABELS[value]}</option>
                ))}
              </select>
            </label>
          </div>
          <fieldset className="task-color-picker">
            <legend>色</legend>
            {(Object.keys(TASK_COLOR_LABELS) as TaskColor[]).map((value) => (
              <label key={value} title={TASK_COLOR_LABELS[value]}>
                <input type="radio" name="selection-task-color" checked={color === value} onChange={() => setColor(value)} />
                <span style={{ backgroundColor: TASK_COLOR_VALUES[value] }} aria-hidden />
                <span className="sr-only">{TASK_COLOR_LABELS[value]}</span>
              </label>
            ))}
          </fieldset>
          <p className="selection-task-dialog__source" title={draft.filePath}>関連ファイル: {draft.filePath}</p>
          {error && <p className="task-list__error" role="alert">{error}</p>}
          <div className="modal__actions">
            <button type="button" onClick={onClose}>キャンセル</button>
            <button type="submit" className="button-primary" disabled={saving}>{saving ? "保存中…" : "タスクを作成"}</button>
          </div>
        </form>
      </div>
    </div>
  );
}
