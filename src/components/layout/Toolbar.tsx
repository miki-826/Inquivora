import { useLocation } from "react-router-dom";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { VerticalNav } from "./VerticalNav";
import { NAV_ITEMS } from "./navItems";

export function Toolbar() {
  const location = useLocation();
  const navigationPosition = useSettingsStore((s) => s.navigationPosition);
  const current = NAV_ITEMS.find((item) => location.pathname.startsWith(item.path));
  return (
    <header className="toolbar">
      <span className="toolbar__brand">Inquivora</span>
      <span className="toolbar__screen">{current?.label ?? ""}</span>
      {navigationPosition === "top" && <VerticalNav placement="top" />}
    </header>
  );
}
