/* eslint-disable react-refresh/only-export-components */
import { lazy } from "react";
import { createHashRouter } from "react-router-dom";
import { RedirectToLastScreen } from "../components/common/RedirectToLastScreen";
import { AppShell } from "./AppShell";
import {
  loadAiSettingsPage,
  loadCalendarPage,
  loadMeetingsPage,
  loadSearchPage,
  loadSettingsPage,
  loadTasksPage,
  loadWorkspacePage,
} from "./routePreload";

// 画面ごとにコード分割し、開いた画面だけを読み込む（低スペックPCでの起動負荷・メモリを軽減）
const WorkspacePage = lazy(() =>
  loadWorkspacePage().then((m) => ({ default: m.WorkspacePage })),
);
const SearchPage = lazy(() =>
  loadSearchPage().then((m) => ({ default: m.SearchPage })),
);
const MeetingsPage = lazy(() =>
  loadMeetingsPage().then((m) => ({ default: m.MeetingsPage })),
);
const TasksPage = lazy(() =>
  loadTasksPage().then((m) => ({ default: m.TasksPage })),
);
const CalendarPage = lazy(() =>
  loadCalendarPage().then((m) => ({ default: m.CalendarPage })),
);
const SettingsPage = lazy(() =>
  loadSettingsPage().then((m) => ({ default: m.SettingsPage })),
);
const AiSettingsPage = lazy(() =>
  loadAiSettingsPage().then((m) => ({ default: m.AiSettingsPage })),
);

export const router = createHashRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <RedirectToLastScreen /> },
      { path: "workspace", element: <WorkspacePage /> },
      { path: "search", element: <SearchPage /> },
      { path: "meetings", element: <MeetingsPage /> },
      { path: "tasks", element: <TasksPage /> },
      { path: "calendar", element: <CalendarPage /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "settings/ai", element: <AiSettingsPage /> },
      { path: "*", element: <RedirectToLastScreen /> },
    ],
  },
]);
