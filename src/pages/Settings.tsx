import { useState, useEffect, useCallback } from "react";
import * as api from "../lib/tauri";
import type { ProxyStatus } from "../lib/types";

export default function Settings() {
  const [status, setStatus] = useState<ProxyStatus | null>(null);
  const [sysProxyEnabled, setSysProxyEnabled] = useState(false);
  const [sysProxyLoading, setSysProxyLoading] = useState(false);
  const [sysProxyError, setSysProxyError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await api.getStatus();
      setStatus(s);
    } catch {
      /* noop */
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const toggleSystemProxy = async () => {
    const next = !sysProxyEnabled;
    setSysProxyLoading(true);
    setSysProxyError(null);
    try {
      const addr = status?.listen_addr ?? "127.0.0.1:1080";
      await api.setSystemProxy(next, addr);
      setSysProxyEnabled(next);
    } catch (e) {
      setSysProxyError(String(e));
    } finally {
      setSysProxyLoading(false);
    }
  };

  return (
    <div className="p-6 max-w-3xl mx-auto space-y-6">
      <h2 className="text-xl font-semibold">Settings</h2>

      {/* System Proxy */}
      <section className="bg-surface-1 border border-surface-3 rounded-xl p-5 space-y-4">
        <h3 className="text-sm font-medium">System Proxy</h3>
        <p className="text-xs text-muted">
          Route all system traffic through the SOCKS proxy. On macOS this
          configures the SOCKS proxy via <code>networksetup</code>. On Windows
          it modifies the registry.
        </p>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm">
              {sysProxyEnabled ? "Enabled" : "Disabled"}
            </p>
            {status?.connected && status.listen_addr && (
              <p className="text-[11px] text-muted font-mono mt-0.5">
                {status.listen_addr}
              </p>
            )}
          </div>
          <button
            onClick={toggleSystemProxy}
            disabled={sysProxyLoading || !status?.connected}
            className={`relative w-12 h-6 rounded-full transition-colors ${
              sysProxyEnabled ? "bg-accent" : "bg-surface-4"
            } disabled:opacity-40`}
          >
            <span
              className={`absolute top-0.5 w-5 h-5 rounded-full bg-white transition-transform ${
                sysProxyEnabled ? "translate-x-6" : "translate-x-0.5"
              }`}
            />
          </button>
        </div>
        {!status?.connected && (
          <p className="text-xs text-warning">
            Connect to a proxy first before enabling system proxy.
          </p>
        )}
        {sysProxyError && (
          <p className="text-xs text-danger bg-danger/10 rounded-lg px-3 py-2">
            {sysProxyError}
          </p>
        )}
      </section>

      {/* About */}
      <section className="bg-surface-1 border border-surface-3 rounded-xl p-5 space-y-3">
        <h3 className="text-sm font-medium">About</h3>
        <div className="text-xs text-muted space-y-1">
          <p>SOCKS6 Reality Client v1.0.0</p>
          <p>
            Cross-platform GUI for the SOCKS6 proxy protocol with Reality
            transport.
          </p>
          <p>
            Platforms: macOS, Windows, iOS, Android
          </p>
        </div>
      </section>
    </div>
  );
}
