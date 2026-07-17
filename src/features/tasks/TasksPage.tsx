import { useEffect } from "react";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { useTaskStore } from "../../stores/useTaskStore";
import { TaskDetailPanel } from "./TaskDetailPanel";
import { TaskFilterPanel } from "./TaskFilterPanel";
import { TaskList } from "./TaskList";

export function TasksPage() {
  const load = useTaskStore((s) => s.load);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <ThreePaneLayout left={<TaskFilterPanel />} right={<TaskDetailPanel />}>
      <TaskList />
    </ThreePaneLayout>
  );
}
