import { useEffect, useRef, useState } from "react";
import AppShell from "./components/layout/AppShell";
import "./App.css";
import MainPage from "./pages/MainPage";
import SettingsPage from "./pages/SettingsPage";
import ResultsPage from "./pages/ResultsPage";
import { AnalysisProvider } from "./features/analysis/store";
import { isTauriRuntime } from "./lib/tauriRuntime";
import { platform } from "./lib/platform";

function App() {
  const [active, setActive] = useState<"main" | "results" | "settings">("main");
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    const stored = localStorage.getItem("theme");
    if (stored === "light" || stored === "dark") return stored;
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  });
  const tauriConfiguredRef = useRef(false);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("theme", theme);
  }, [theme]);

  useEffect(() => {
    const shouldLockDebugging = import.meta.env.PROD || isTauriRuntime();
    if (!shouldLockDebugging) return;

    const preventContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };

    const preventDebugShortcuts = (e: KeyboardEvent) => {
      const key = e.key ?? "";
      const lower = key.toLowerCase();
      const ctrlOrMeta = e.ctrlKey || e.metaKey;

      if (key === "F12") {
        e.preventDefault();
        return;
      }

      if (ctrlOrMeta && lower === "u") {
        e.preventDefault();
        return;
      }

      const isCtrlShift = e.ctrlKey && e.shiftKey;
      const isMetaAlt = e.metaKey && e.altKey;
      if ((isCtrlShift || isMetaAlt) && ["i", "j", "c", "k"].includes(lower)) {
        e.preventDefault();
      }
    };

    window.addEventListener("contextmenu", preventContextMenu, true);
    window.addEventListener("keydown", preventDebugShortcuts, true);
    return () => {
      window.removeEventListener("contextmenu", preventContextMenu, true);
      window.removeEventListener("keydown", preventDebugShortcuts, true);
    };
  }, []);

  useEffect(() => {
    const tauriDetected = isTauriRuntime();
    if (tauriDetected && !tauriConfiguredRef.current) {
      document.body.classList.add("tauri-app");
      import("@tauri-apps/api/window").then(({ getCurrentWindow, PhysicalSize }) => {
        const appWindow = getCurrentWindow();
        appWindow.setSize(new PhysicalSize(1280, 840));
        appWindow.setMinSize(new PhysicalSize(1280, 840));
        appWindow.setMaxSize(new PhysicalSize(1280, 840));
        appWindow.setResizable(false);
        appWindow.setTitle("img-sort");
      });
      if (platform.isMac) {
        import("@tauri-apps/api/menu")
          .then(async ({ Menu, PredefinedMenuItem, Submenu }) => {
            const hide = await PredefinedMenuItem.new({ item: "Hide", text: "Hide img-sort" });
            const hideOthers = await PredefinedMenuItem.new({ item: "HideOthers" });
            const showAll = await PredefinedMenuItem.new({ item: "ShowAll" });
            const separator = await PredefinedMenuItem.new({ item: "Separator" });
            const quit = await PredefinedMenuItem.new({ item: "Quit", text: "Quit img-sort" });
            const app = await Submenu.new({
              text: "img-sort",
              items: [hide, hideOthers, showAll, separator, quit],
            });
            const menu = await Menu.new({ items: [app] });
            await menu.setAsAppMenu();
          })
          .catch(() => {});
      }
      const preventKeys = (e: KeyboardEvent) => {
        if (e.metaKey || e.ctrlKey) {
          const blocked = ["r", "R", "w", "W", "p", "P", "l", "L", ",", "s", "S"];
          if (blocked.includes(e.key)) {
            e.preventDefault();
          }
        }
      };
      window.addEventListener("keydown", preventKeys);
      tauriConfiguredRef.current = true;
      return () => window.removeEventListener("keydown", preventKeys);
    }
  }, []);

  const navItems = [
    { id: "main", label: "분석" },
    { id: "results", label: "결과" },
    { id: "settings", label: "설정" },
  ];

  return (
    <AnalysisProvider>
      <AppShell
        navItems={navItems}
        active={active}
        onNavigate={(id) => setActive(id as any)}
      >
        {active === "main" ? (
          <MainPage />
        ) : active === "results" ? (
          <ResultsPage />
        ) : (
          <SettingsPage theme={theme} onToggleTheme={() => setTheme(theme === "light" ? "dark" : "light")} />
        )}
      </AppShell>
    </AnalysisProvider>
  );
}

export default App;
