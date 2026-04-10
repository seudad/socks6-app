import { useState } from "react";
import Dashboard from "./pages/Dashboard";
import Profiles from "./pages/Profiles";
import Settings from "./pages/Settings";

type Page = "dashboard" | "profiles" | "settings";

const NAV_ITEMS: { id: Page; label: string; icon: string }[] = [
  { id: "dashboard", label: "Dashboard", icon: "⚡" },
  { id: "profiles", label: "Profiles", icon: "☰" },
  { id: "settings", label: "Settings", icon: "⚙" },
];

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");

  return (
    <div className="flex h-screen">
      <nav className="w-56 shrink-0 bg-surface-1 border-r border-surface-3 flex flex-col">
        <div className="px-5 py-5 border-b border-surface-3">
          <h1 className="text-lg font-bold tracking-tight">SOCKS6</h1>
          <p className="text-xs text-muted mt-0.5">Reality Client</p>
        </div>
        <div className="flex-1 py-3 px-3 space-y-1">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              onClick={() => setPage(item.id)}
              className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors ${
                page === item.id
                  ? "bg-accent/15 text-accent-hover"
                  : "text-gray-400 hover:text-white hover:bg-surface-3"
              }`}
            >
              <span className="text-base">{item.icon}</span>
              {item.label}
            </button>
          ))}
        </div>
        <div className="px-5 py-4 border-t border-surface-3">
          <p className="text-[10px] text-muted">v1.0.0</p>
        </div>
      </nav>
      <main className="flex-1 overflow-y-auto">
        {page === "dashboard" && <Dashboard />}
        {page === "profiles" && <Profiles />}
        {page === "settings" && <Settings />}
      </main>
    </div>
  );
}
