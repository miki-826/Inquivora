import { NavLink } from "react-router-dom";

export function SettingsNav() {
  return (
    <div className="pane-section">
      <div className="pane-section__title">設定</div>
      <nav className="settings-nav">
        <NavLink to="/settings" end className="settings-nav__link">
          通知・一般
        </NavLink>
        <NavLink to="/settings/ai" className="settings-nav__link">
          AI・API
        </NavLink>
        <NavLink to="/settings/licenses" className="settings-nav__link">
          ライセンス
        </NavLink>
      </nav>
    </div>
  );
}
