import { createHashRouter } from "react-router-dom";
import { AppShell } from "./AppShell";
import { WorkspacePage } from "../features/workspace/WorkspacePage";
import { SearchPage } from "../features/search/SearchPage";
import { MeetingsPage } from "../features/meetings/MeetingsPage";
import { TasksPage } from "../features/tasks/TasksPage";
import { CalendarPage } from "../features/calendar/CalendarPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { RedirectToLastScreen } from "../components/common/RedirectToLastScreen";

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
