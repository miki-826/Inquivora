export const loadWorkspacePage = () => import("../features/workspace/WorkspacePage");
export const loadSearchPage = () => import("../features/search/SearchPage");
export const loadMeetingsPage = () => import("../features/meetings/MeetingsPage");
export const loadTasksPage = () => import("../features/tasks/TasksPage");
export const loadCalendarPage = () => import("../features/calendar/CalendarPage");
export const loadSettingsPage = () => import("../features/settings/SettingsPage");
export const loadAiSettingsPage = () => import("../features/settings/AiSettingsPage");
export const loadLicenseSettingsPage = () => import("../features/settings/LicenseSettingsPage");

const pageLoaders: Record<string, () => Promise<unknown>> = {
  "/workspace": loadWorkspacePage,
  "/search": loadSearchPage,
  "/meetings": loadMeetingsPage,
  "/tasks": loadTasksPage,
  "/calendar": loadCalendarPage,
  "/settings": loadSettingsPage,
  "/settings/ai": loadAiSettingsPage,
  "/settings/licenses": loadLicenseSettingsPage,
};

export function preloadRoute(path: string): void {
  void pageLoaders[path]?.();
}
