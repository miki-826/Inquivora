import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useCalendarStore } from "../../stores/useCalendarStore";
import { useTaskStore } from "../../stores/useTaskStore";
import { parseDeepLink } from "./notificationModel";

/// §14.3 通知クリック等のinquivora://リンクで対象画面へ遷移し対象レコードを選択する。
export function useDeepLink() {
  const navigate = useNavigate();

  useEffect(() => {
    const handle = (url: string) => {
      const target = parseDeepLink(url);
      if (!target) return;
      if (target.type === "task") {
        useTaskStore.getState().select(target.id);
        void useTaskStore.getState().load();
        navigate("/tasks");
      } else if (target.type === "event") {
        useCalendarStore.getState().setFocusEventId(target.id);
        navigate("/calendar");
      } else {
        navigate("/meetings");
      }
    };
    const unlisten = onOpenUrl((urls) => urls.forEach(handle));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [navigate]);
}
