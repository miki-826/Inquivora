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
  useDeepLink();

  useEffect(() => {
    initMeetingListeners();
    initSearchIndexListener();
    void useThemeStore.getState().init();

    // 起動直後のアイドル時に重い画面チャンク（Monaco等）を裏で先読みし、
    // 初回のナビ切替でも待たされないようにする
    const warmup = () => {
      void import("../features/workspace/WorkspacePage");
      void import("../features/meetings/MeetingsPage");
      void import("../features/calendar/CalendarPage");
    };
    const ric = (
      window as Window & { requestIdleCallback?: (cb: () => void) => number }
    ).requestIdleCallback;
    if (ric) {
      ric(warmup);
    } else {
      setTimeout(warmup, 1500);
    }
  }, []);

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
          <Suspense fallback={<div className="app-shell__loading">読み込み中…</div>}>
            <Outlet />
          </Suspense>
        </main>
      </div>
      <StatusBar />
    </div>
  );
}
