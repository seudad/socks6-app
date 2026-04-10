import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../lib/tauri";
import type { ProxyStatus, Profile, LogEntry } from "../lib/types";
import StatusCard from "../components/StatusCard";
import LogViewer from "../components/LogViewer";

export default function Dashboard() {
  const [status, setStatus] = useState<ProxyStatus>({
    connected: false,
    profile_id: null,
    listen_addr: null,
    server: null,
    uptime_secs: 0,
    connections: 0,
  });
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selectedId, setSelectedId] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const refreshStatus = useCallback(async () => {
    try {
      const s = await api.getStatus();
      setStatus(s);
    } catch {
      /* noop */
    }
  }, []);

  useEffect(() => {
    api.listProfiles().then((p) => {
      setProfiles(p);
      if (p.length > 0 && !selectedId) setSelectedId(p[0].id);
    });
    refreshStatus();
    const timer = setInterval(refreshStatus, 2000);
    return () => clearInterval(timer);
  }, [refreshStatus, selectedId]);

  useEffect(() => {
    const unlisten = listen<{ level: string; message: string }>(
      "proxy-log",
      (event) => {
        setLogs((prev) => [
          ...prev.slice(-199),
          {
            timestamp: Date.now(),
            level: event.payload.level as LogEntry["level"],
            message: event.payload.message,
          },
        ]);
      },
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleConnect = async () => {
    if (!selectedId) return;
    setLoading(true);
    setError(null);
    try {
      await api.connect(selectedId);
      await refreshStatus();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleDisconnect = async () => {
    setLoading(true);
    setError(null);
    try {
      await api.disconnect();
      await refreshStatus();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const activeProfile = profiles.find((p) => p.id === status.profile_id);

  return (
    <div className="p-6 max-w-3xl mx-auto space-y-6">
      <h2 className="text-xl font-semibold">Dashboard</h2>

      <StatusCard status={status} profileName={activeProfile?.name} />

      {/* Connect / Disconnect */}
      <div className="bg-surface-1 rounded-xl border border-surface-3 p-5 space-y-4">
        {!status.connected ? (
          <>
            <label className="block text-sm text-gray-400 mb-1">
              Server Profile
            </label>
            <select
              value={selectedId}
              onChange={(e) => setSelectedId(e.target.value)}
              className="w-full bg-surface-2 border border-surface-4 rounded-lg px-3 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-accent/50"
            >
              {profiles.length === 0 && (
                <option value="">No profiles — create one first</option>
              )}
              {profiles.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name} ({p.server})
                </option>
              ))}
            </select>
            <button
              onClick={handleConnect}
              disabled={loading || !selectedId}
              className="w-full py-3 rounded-lg font-semibold text-sm transition-all bg-accent hover:bg-accent-hover disabled:opacity-40 disabled:cursor-not-allowed"
            >
              {loading ? "Connecting..." : "Connect"}
            </button>
          </>
        ) : (
          <button
            onClick={handleDisconnect}
            disabled={loading}
            className="w-full py-3 rounded-lg font-semibold text-sm transition-all bg-danger/90 hover:bg-danger disabled:opacity-40"
          >
            {loading ? "Disconnecting..." : "Disconnect"}
          </button>
        )}

        {error && (
          <p className="text-danger text-xs mt-2 bg-danger/10 rounded-lg px-3 py-2">
            {error}
          </p>
        )}
      </div>

      <LogViewer logs={logs} onClear={() => setLogs([])} />
    </div>
  );
}
