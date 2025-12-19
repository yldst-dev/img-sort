import classNames from "classnames";

export interface NavItem {
  id: string;
  label: string;
  icon?: string;
}

interface SidebarProps {
  items: NavItem[];
  active: string;
  onNavigate: (id: string) => void;
}

function Sidebar({ items, active, onNavigate }: SidebarProps) {
  return (
    <aside className="sidebar">
      <nav>
        {items.map((item) => (
          <button
            key={item.id}
            className={classNames("nav-item", { active: active === item.id })}
            onClick={() => onNavigate(item.id)}
          >
            <span className="nav-label">{item.label}</span>
          </button>
        ))}
      </nav>
    </aside>
  );
}

export default Sidebar;
