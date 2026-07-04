import { useEffect } from "react";
import { RouterProvider } from "react-router-dom";
import { router } from "./router";
import { useSettingsStore } from "../stores/useSettingsStore";

export function App() {
  const hydrated = useSettingsStore((s) => s.hydrated);

  useEffect(() => {
    void useSettingsStore.getState().hydrate();
  }, []);

  if (!hydrated) {
    return null;
  }
  return <RouterProvider router={router} />;
}
