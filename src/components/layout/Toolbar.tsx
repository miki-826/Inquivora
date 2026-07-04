import { useLocation } from "react-router-dom";
import { NAV_ITEMS } from "./navItems";

export function Toolbar() {
  const location = useLocation();
  const current = NAV_ITEMS.find((item) => location.pathname.startsWith(item.path));
  return (
    <header className="toolbar">
      <span className="toolbar__brand">Inquivora</span>
      <span className="toolbar__screen">{current?.label ?? ""}</span>
    </header>
  );
}
