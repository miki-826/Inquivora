import {
  CalendarDays,
  FileAudio,
  FolderTree,
  Search,
  Settings,
  SquareCheckBig,
  type LucideIcon,
} from "lucide-react";

export type NavItem = {
  id: string;
  path: string;
  label: string;
  icon: LucideIcon;
};

export const NAV_ITEMS: NavItem[] = [
  { id: "workspace", path: "/workspace", label: "ワークスペース", icon: FolderTree },
  { id: "search", path: "/search", label: "全文検索", icon: Search },
  { id: "meetings", path: "/meetings", label: "議事録", icon: FileAudio },
  { id: "tasks", path: "/tasks", label: "タスク", icon: SquareCheckBig },
  { id: "calendar", path: "/calendar", label: "カレンダー", icon: CalendarDays },
  { id: "settings", path: "/settings", label: "設定", icon: Settings },
];
