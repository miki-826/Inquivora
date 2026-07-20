import { NavLink } from "react-router-dom";
import { preloadRoute } from "../../app/routePreload";
import { NAV_ITEMS } from "./navItems";

function preloadProps(path: string) {
  return {
    onFocus: () => preloadRoute(path),
    onPointerEnter: () => preloadRoute(path),
    onPointerDown: () => preloadRoute(path),
  };
}

export function VerticalNav() {
  const mainItems = NAV_ITEMS.filter((item) => item.id !== "settings");
  const settingsItem = NAV_ITEMS.find((item) => item.id === "settings");
  return (
    <nav className="vertical-nav" aria-label="メインナビゲーション">
      {mainItems.map((item) => (
        <NavLink
          key={item.id}
          to={item.path}
          className={({ isActive }) => `vertical-nav__item${isActive ? " active" : ""}`}
          title={item.label}
          aria-label={item.label}
          {...preloadProps(item.path)}
        >
          <item.icon size={22} />
          <span className="vertical-nav__label">{item.label}</span>
        </NavLink>
      ))}
      <div className="vertical-nav__spacer" />
      {settingsItem && (
        <NavLink
          to={settingsItem.path}
          className={({ isActive }) => `vertical-nav__item${isActive ? " active" : ""}`}
          title={settingsItem.label}
          aria-label={settingsItem.label}
          {...preloadProps(settingsItem.path)}
        >
          <settingsItem.icon size={22} />
          <span className="vertical-nav__label">{settingsItem.label}</span>
        </NavLink>
      )}
    </nav>
  );
}
