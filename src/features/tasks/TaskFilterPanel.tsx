import { useTaskStore } from "../../stores/useTaskStore";
import { PRIORITY_LABELS, type TaskPreset, type TaskPriority } from "./taskModel";

const PRESETS: { key: TaskPreset; label: string }[] = [
  { key: "all", label: "すべて" },
  { key: "open", label: "未完了" },
  { key: "inProgress", label: "進行中" },
  { key: "completed", label: "完了" },
  { key: "today", label: "今日" },
  { key: "thisWeek", label: "今週" },
  { key: "overdue", label: "期限切れ" },
];

function distinct(values: (string | null)[]): string[] {
  return [...new Set(values.filter((v): v is string => Boolean(v)))].sort((a, b) =>
    a.localeCompare(b, "ja"),
  );
}

export function TaskFilterPanel() {
  const filter = useTaskStore((s) => s.filter);
  const allTasks = useTaskStore((s) => s.allTasks);
  const setFilter = useTaskStore((s) => s.setFilter);

  const projects = distinct(allTasks.map((t) => t.projectName));
  const assignees = distinct(allTasks.map((t) => t.assignee));

  return (
    <div className="task-filter">
      <h2 className="pane-title">フィルター</h2>
      <nav className="task-filter__presets" aria-label="タスクフィルター">
        {PRESETS.map((preset) => (
          <button
            key={preset.key}
            type="button"
            className={`task-filter__preset${
              (filter.preset ?? "all") === preset.key ? " task-filter__preset--active" : ""
            }`}
            onClick={() => setFilter({ ...filter, preset: preset.key })}
          >
            {preset.label}
          </button>
        ))}
      </nav>
      <label className="task-filter__field">
        優先度
        <select
          value={filter.priority ?? ""}
          onChange={(e) =>
            setFilter({
              ...filter,
              priority: e.target.value ? (e.target.value as TaskPriority) : undefined,
            })
          }
        >
          <option value="">すべて</option>
          {(Object.keys(PRIORITY_LABELS) as TaskPriority[]).map((p) => (
            <option key={p} value={p}>
              {PRIORITY_LABELS[p]}
            </option>
          ))}
        </select>
      </label>
      <label className="task-filter__field">
        プロジェクト
        <select
          value={filter.projectName ?? ""}
          onChange={(e) =>
            setFilter({ ...filter, projectName: e.target.value || undefined })
          }
        >
          <option value="">すべて</option>
          {projects.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
      </label>
      <label className="task-filter__field">
        担当
        <select
          value={filter.assignee ?? ""}
          onChange={(e) => setFilter({ ...filter, assignee: e.target.value || undefined })}
        >
          <option value="">すべて</option>
          {assignees.map((a) => (
            <option key={a} value={a}>
              {a}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
