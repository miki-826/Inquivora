import { useEffect } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { Toolbar } from "../components/layout/Toolbar";
import { VerticalNav } from "../components/layout/VerticalNav";
import { StatusBar } from "../components/statusbar/StatusBar";
import { useSettingsStore } from "../stores/useSettingsStore";

export function AppShell() {
  const location = useLocation();
  const setLastScreen = useSettingsStore((s) => s.setLastScreen);

  useEffect(() => {
    if (location.pathname !== "/") {
      setLastScreen(location.pathname);
    }
  }, [location.pathname, setLastScreen]);

  return (
    <div className="app-shell">
      <Toolbar />
      <div className="app-shell__body">
        <VerticalNav />
        <main className="app-shell__main">
          <Outlet />
        </main>
      </div>
      <StatusBar />
    </div>
  );
}
