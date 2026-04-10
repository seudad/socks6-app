import { useEffect, useRef } from "react";
import type { LogEntry } from "../lib/types";

const LEVEL_COLORS: Record<string, string> = {
  info: "text-accent-hover",
  warn: "text-warning",
  error: "text-danger",
  debug: "text-muted",
};

export default function LogViewer({
  logs,
  onClear,
}: {
  logs: LogEntry[];
  onClear: () => void;
}) {
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs.length]);

  return (
    <div className="bg-surface-1 rounded-xl border border-surface-3 overflow-hidden">
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-surface-3">
        <span className="text-xs font-medium text-gray-400">Log</span>
        <button
          onClick={onClear}
          className="text-[10px] text-muted hover:text-white transition-colors"
        >
          Clear
        </button>
      </div>
      <div className="h-48 overflow-y-auto px-4 py-2 font-mono text-[11px] leading-5 space-y-0.5">
        {logs.length === 0 && (
          <p className="text-muted py-4 text-center text-xs">
            No log entries yet
          </p>
        )}
        {logs.map((entry, i) => (
          <div key={i} className="flex gap-2">
            <span className="text-muted shrink-0">
              {new Date(entry.timestamp).toLocaleTimeString()}
            </span>
            <span
              className={`shrink-0 uppercase w-10 ${LEVEL_COLORS[entry.level] ?? "text-muted"}`}
            >
              {entry.level}
            </span>
            <span className="text-gray-300 break-all">{entry.message}</span>
          </div>
        ))}
        <div ref={endRef} />
      </div>
    </div>
  );
}
