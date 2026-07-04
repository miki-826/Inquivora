import { useEffect } from "react";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";

export function useRestoreWorkspaceOnMount() {
  const restoring = useWorkspaceStore((s) => s.restoring);
  useEffect(() => {
    if (useWorkspaceStore.getState().restoring && !useWorkspaceStore.getState().workspace) {
      useWorkspaceStore.getState().restoreLastWorkspace();
    }
  }, []);
  return restoring;
}
