import type { ProxyStatus } from "../lib/types";

function formatUptime(secs: number): string {
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  if (m < 60) return `${m}m ${s}s`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
}

export default function StatusCard({
  status,
  profileName,
}: {
  status: ProxyStatus;
  profileName?: string;
}) {
  return (
    <div
      className={`rounded-xl border p-5 transition-colors ${
        status.connected
          ? "bg-success/5 border-success/30"
          : "bg-surface-1 border-surface-3"
      }`}
    >
      <div className="flex items-center gap-3 mb-4">
        <div
          className={`w-3 h-3 rounded-full ${
            status.connected ? "bg-success animate-pulse" : "bg-muted"
          }`}
        />
        <span className="font-semibold text-sm">
          {status.connected ? "Connected" : "Disconnected"}
        </span>
      </div>

      {status.connected && (
        <div className="grid grid-cols-2 gap-3 text-sm">
          {profileName && (
            <Stat label="Profile" value={profileName} />
          )}
          <Stat label="Server" value={status.server ?? "—"} />
          <Stat label="Listen" value={status.listen_addr ?? "—"} />
          <Stat label="Uptime" value={formatUptime(status.uptime_secs)} />
          <Stat label="Connections" value={String(status.connections)} />
        </div>
      )}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] text-muted uppercase tracking-wider">
        {label}
      </p>
      <p className="text-white font-mono text-xs mt-0.5 truncate">{value}</p>
    </div>
  );
}
