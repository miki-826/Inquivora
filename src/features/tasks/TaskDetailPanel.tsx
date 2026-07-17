import { formatInTimeZone } from "date-fns-tz";
import { Copy, Trash2 } from "lucide-react";
import { useState } from "react";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";
import { useTaskStore } from "../../stores/useTaskStore";
import {
  PRIORITY_LABELS,
  STATUS_LABELS,
  TOKYO_TZ,
  buildDueAtUtc,
  splitDueAtUtc,
  type Task,
  type TaskPriority,
  type TaskStatus,
} from "./taskModel";

function DetailForm({ task }: { task: Task }) {
  const updateTask = useTaskStore((s) => s.updateTask);
  const removeTask = useTaskStore((s) => s.removeTask);
  const duplicateTask = useTaskStore((s) => s.duplicateTask);
  const initial = splitDueAtUtc(task.dueAtUtc);
  const [dueDate, setDueDate] = useState(initial.date);
  const [dueTime, setDueTime] = useState(initial.time);

  const commitDue = (date: string, time: string) => {
    setDueDate(date);
    setDueTime(time);
    void updateTask(task.id, { dueAtUtc: buildDueAtUtc(date, time) });
  };

  const commitText = (
    field: "title" | "description" | "assignee" | "projectName",
    value: string,
  ) => {
    const trimmed = value.trim();
    if (field === "title") {
      if (trimmed && trimmed !== task.title) void updateTask(task.id, { title: trimmed });
      return;
    }
    const current = task[field] ?? "";
    if (trimmed !== current) void updateTask(task.id, { [field]: trimmed || null });
  };

  return (
    <div className="task-detail">
      <h2 className="pane-title">タスク詳細</h2>
      <input
        className="task-detail__title"
        type="text"
        defaultValue={task.title}
        aria-label="タイトル"
        onBlur={(e) => commitText("title", e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
        }}
      />
      <div className="task-detail__grid">
        <label>
          状態
          <select
            value={task.status}
            onChange={(e) => void updateTask(task.id, { status: e.target.value as TaskStatus })}
          >
            {(Object.keys(STATUS_LABELS) as TaskStatus[]).map((s) => (
              <option key={s} value={s}>
                {STATUS_LABELS[s]}
              </option>
            ))}
          </select>
        </label>
        <label>
          優先度
          <select
            value={task.priority}
            onChange={(e) =>
              void updateTask(task.id, { priority: e.target.value as TaskPriority })
            }
          >
            {(Object.keys(PRIORITY_LABELS) as TaskPriority[]).map((p) => (
              <option key={p} value={p}>
                {PRIORITY_LABELS[p]}
              </option>
            ))}
          </select>
        </label>
        <label>
          期日
          <input
            type="date"
            value={dueDate}
            onChange={(e) => commitDue(e.target.value, e.target.value ? dueTime : "")}
          />
        </label>
        <label>
          時刻
          <input
            type="time"
            value={dueTime}
            disabled={!dueDate}
            onChange={(e) => commitDue(dueDate, e.target.value)}
          />
        </label>
        <label>
          担当
          <input
            type="text"
            defaultValue={task.assignee ?? ""}
            onBlur={(e) => commitText("assignee", e.target.value)}
          />
        </label>
        <label>
          プロジェクト
          <input
            type="text"
            defaultValue={task.projectName ?? ""}
            onBlur={(e) => commitText("projectName", e.target.value)}
          />
        </label>
      </div>
      <label className="task-detail__memo">
        メモ
        <textarea
          defaultValue={task.description ?? ""}
          rows={6}
          onBlur={(e) => commitText("description", e.target.value)}
        />
      </label>
      {task.linkedFilePath && (
        <p className="task-detail__meta">関連ファイル: {task.linkedFilePath}</p>
      )}
      <p className="task-detail__meta">
        作成: {formatInTimeZone(task.createdAt, TOKYO_TZ, "yyyy年M月d日 HH:mm")}
        {task.completedAt &&
          ` ／ 完了: ${formatInTimeZone(task.completedAt, TOKYO_TZ, "yyyy年M月d日 HH:mm")}`}
      </p>
      <div className="task-detail__actions">
        <button type="button" onClick={() => void duplicateTask(task.id)}>
          <Copy size={13} aria-hidden /> 複製
        </button>
        <button
          type="button"
          className="task-detail__delete"
          onClick={() => void removeTask(task.id)}
        >
          <Trash2 size={13} aria-hidden /> 削除
        </button>
      </div>
    </div>
  );
}

export function TaskDetailPanel() {
  const selectedId = useTaskStore((s) => s.selectedId);
  const task = useTaskStore(
    (s) => s.allTasks.find((t) => t.id === s.selectedId) ?? s.tasks.find((t) => t.id === s.selectedId),
  );

  if (!selectedId || !task) {
    return (
      <PanePlaceholder title="タスク詳細" description="タスクを選択すると詳細を編集できます" />
    );
  }
  return <DetailForm key={`${task.id}:${task.updatedAt}`} task={task} />;
}
