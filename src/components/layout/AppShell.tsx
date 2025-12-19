import { ReactNode } from "react";
import Sidebar, { NavItem } from "../nav/Sidebar";
import Toaster from "../toast/Toaster";
import TitleBar from "./TitleBar";
import { platform } from "../../lib/platform";
import { isTauriRuntime } from "../../lib/tauriRuntime";

interface AppShellProps {
  navItems: NavItem[];
  active: string;
  onNavigate: (id: string) => void;
  children: ReactNode;
}

function AppShell({
  navItems,
  active,
  onNavigate,
  children,
}: AppShellProps) {
  const showMacTitlebar = platform.isMac && isTauriRuntime();

  return (
    <div className={`app-shell ${showMacTitlebar ? "app-shell--mac-titlebar" : ""}`}>
      {showMacTitlebar && (
        <div className="app-shell-titlebar">
          <TitleBar title="img-sort" />
        </div>
      )}
      <Sidebar items={navItems} active={active} onNavigate={onNavigate} />
      <div className="app-main">
        <main className="app-content">
          {children}
          <Toaster />
        </main>
      </div>
    </div>
  );
}

export default AppShell;
