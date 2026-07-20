/* eslint-disable react-refresh/only-export-components */
import { lazy } from "react";
import { createHashRouter } from "react-router-dom";
import { RedirectToLastScreen } from "../components/common/RedirectToLastScreen";
import { AppShell } from "./AppShell";

// 画面ごとにコード分割し、開いた画面だけを読み込む（低スペックPCでの起動負荷・メモリを軽減）
const WorkspacePage = lazy(() =>
  import("../features/workspace/WorkspacePage").then((m) => ({ default: m.WorkspacePage })),
);
const SearchPage = lazy(() =>
  import("../features/search/SearchPage").then((m) => ({ default: m.SearchPage })),
);
const MeetingsPage = lazy(() =>
  import("../features/meetings/MeetingsPage").then((m) => ({ default: m.MeetingsPage })),
);
const TasksPage = lazy(() =>
  import("../features/tasks/TasksPage").then((m) => ({ default: m.TasksPage })),
);
const CalendarPage = lazy(() =>
  import("../features/calendar/CalendarPage").then((m) => ({ default: m.CalendarPage })),
);
const SettingsPage = lazy(() =>
  import("../features/settings/SettingsPage").then((m) => ({ default: m.SettingsPage })),
);
const AiSettingsPage = lazy(() =>
  import("../features/settings/AiSettingsPage").then((m) => ({ default: m.AiSettingsPage })),
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
