import { Suspense, useEffect } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { Toolbar } from "../components/layout/Toolbar";
import { VerticalNav } from "../components/layout/VerticalNav";
import { StatusBar } from "../components/statusbar/StatusBar";
import { useDeepLink } from "../features/notifications/useDeepLink";
import { initSearchIndexListener } from "../stores/searchIndexListener";
import { initMeetingListeners } from "../stores/useMeetingStore";
import { useSettingsStore } from "../stores/useSettingsStore";
import { useThemeStore } from "../stores/useThemeStore";

export function AppShell() {
  const location = useLocation();
  const setLastScreen = useSettingsStore((s) => s.setLastScreen);
  const navigationPosition = useSettingsStore((s) => s.navigationPosition);
  const uiDensity = useSettingsStore((s) => s.uiDensity);
  const showStatusBar = useSettingsStore((s) => s.showStatusBar);
  const reduceMotion = useSettingsStore((s) => s.reduceMotion);
  useDeepLink();

  useEffect(() => {
    initMeetingListeners();
    initSearchIndexListener();
    void useThemeStore.getState().init();
  }, []);

  useEffect(() => {
    if (location.pathname !== "/") {
      setLastScreen(location.pathname);
    }
  }, [location.pathname, setLastScreen]);

  return (
    <div
      className={`app-shell${uiDensity === "compact" ? " app-shell--compact" : ""}${reduceMotion ? " app-shell--reduced-motion" : ""}`}
    >
      <Toolbar />
      <div className="app-shell__body">
        {navigationPosition === "side" && <VerticalNav placement="side" />}
        <main className="app-shell__main">
          <Suspense fallback={<div className="app-shell__loading">読み込み中…</div>}>
            <Outlet />
          </Suspense>
        </main>
        {navigationPosition === "right" && <VerticalNav placement="right" />}
      </div>
      {navigationPosition === "bottom" && <VerticalNav placement="bottom" />}
      {showStatusBar && <StatusBar />}
    </div>
  );
}
