import { Check, Circle } from "lucide-react";
import { useState } from "react";
import { useTaskStore } from "../../stores/useTaskStore";
import {
  PRIORITY_LABELS,
  STATUS_LABELS,
  dueGroupOf,
  formatDueLabel,
  groupTasks,
  type Task,
} from "./taskModel";

function TaskRow({ task, now }: { task: Task; now: Date }) {
  const selectedId = useTaskStore((s) => s.selectedId);
  const select = useTaskStore((s) => s.select);
  const toggleComplete = useTaskStore((s) => s.toggleComplete);
  const completed = task.status === "completed";
  const overdue = dueGroupOf(task, now) === "overdue";

  return (
    <div
      className={`task-row${task.id === selectedId ? " task-row--selected" : ""}${
        completed ? " task-row--completed" : ""
      }`}
      role="button"
      tabIndex={0}
      onClick={() => select(task.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter") select(task.id);
      }}
    >
      <button
        type="button"
        className={`task-row__check${completed ? " task-row__check--done" : ""}`}
        aria-label={completed ? "未完了へ戻す" : "完了にする"}
        onClick={(e) => {
          e.stopPropagation();
          void toggleComplete(task);
        }}
      >
        {completed ? <Check size={13} aria-hidden /> : <Circle size={13} aria-hidden />}
      </button>
      <span className="task-row__title">{task.title}</span>
      {task.priority !== "medium" && (
        <span className={`task-row__badge task-row__badge--${task.priority}`}>
          {PRIORITY_LABELS[task.priority]}
        </span>
      )}
      {(task.status === "in_progress" || task.status === "on_hold") && (
        <span className="task-row__badge task-row__badge--status">
          {STATUS_LABELS[task.status]}
        </span>
      )}
      {task.projectName && <span className="task-row__badge">{task.projectName}</span>}
      {task.dueAtUtc && (
        <span className={`task-row__due${overdue ? " task-row__due--overdue" : ""}`}>
          {formatDueLabel(task.dueAtUtc, now)}
        </span>
      )}
    </div>
  );
}

export function TaskList() {
  const tasks = useTaskStore((s) => s.tasks);
  const loading = useTaskStore((s) => s.loading);
  const error = useTaskStore((s) => s.error);
  const createTask = useTaskStore((s) => s.createTask);
  const [newTitle, setNewTitle] = useState("");
  const now = new Date();

  const submit = () => {
    const title = newTitle.trim();
    if (!title) return;
    setNewTitle("");
    void createTask({ title });
  };

  return (
    <div className="task-list">
      <form
        className="task-list__add"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <input
          type="text"
          value={newTitle}
          placeholder="新しいタスクを追加（Enterで作成）"
          onChange={(e) => setNewTitle(e.target.value)}
        />
      </form>
      {error && (
        <p className="task-list__error" role="alert">
          {error}
        </p>
      )}
      {tasks.length === 0 && !loading ? (
        <p className="task-list__empty">条件に合うタスクはありません</p>
      ) : (
        groupTasks(tasks, now).map((group) => (
          <section key={group.group} className="task-group">
            <h3 className="task-group__label">
              {group.label}
              <span className="task-group__count">{group.tasks.length}</span>
            </h3>
            {group.tasks.map((task) => (
              <TaskRow key={task.id} task={task} now={now} />
            ))}
          </section>
        ))
      )}
    </div>
  );
}
