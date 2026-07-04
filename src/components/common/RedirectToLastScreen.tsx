import { Navigate } from "react-router-dom";
import { useSettingsStore } from "../../stores/useSettingsStore";

export function RedirectToLastScreen() {
  const lastScreen = useSettingsStore((s) => s.lastScreen);
  return <Navigate to={lastScreen} replace />;
}
