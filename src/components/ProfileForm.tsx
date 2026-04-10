import { useState } from "react";
import type { Profile, GeneratedKeys } from "../lib/types";
import * as api from "../lib/tauri";

const EMPTY_PROFILE: Profile = {
  id: "",
  name: "",
  server: "",
  server_name: "",
  secret: "",
  short_id: "",
  auth_user: "",
  auth_pass: "",
  listen: "127.0.0.1:1080",
  max_tls_parallel: 12,
  auth_time_offset_secs: 0,
};

export default function ProfileForm({
  initial,
  onSave,
  onCancel,
}: {
  initial?: Profile;
  onSave: (profile: Profile) => void;
  onCancel: () => void;
}) {
  const [form, setForm] = useState<Profile>(() => ({
    ...EMPTY_PROFILE,
    ...initial,
    id: initial?.id || crypto.randomUUID(),
  }));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const set = (key: keyof Profile, value: string | number) =>
    setForm((prev) => ({ ...prev, [key]: value }));

  const handleGenerateKeys = async () => {
    try {
      const keys: GeneratedKeys = await api.generateKeys();
      setForm((prev) => ({
        ...prev,
        secret: keys.secret,
        short_id: keys.short_id,
      }));
    } catch (e) {
      setError(String(e));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim()) return setError("Name is required");
    if (!form.server.trim()) return setError("Server address is required");
    if (!form.server_name.trim()) return setError("Server name (SNI) is required");
    if (!form.secret.trim()) return setError("Secret is required");
    if (!form.short_id.trim()) return setError("Short ID is required");

    setSaving(true);
    setError(null);
    try {
      await api.saveProfile(form);
      onSave(form);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
      <form
        onSubmit={handleSubmit}
        className="bg-surface-1 border border-surface-3 rounded-2xl w-full max-w-lg max-h-[90vh] overflow-y-auto"
      >
        <div className="px-6 py-4 border-b border-surface-3">
          <h3 className="font-semibold">
            {initial ? "Edit Profile" : "New Profile"}
          </h3>
        </div>

        <div className="px-6 py-5 space-y-4">
          <Field label="Name" value={form.name} onChange={(v) => set("name", v)} placeholder="My Server" />
          <Field label="Server (host:port)" value={form.server} onChange={(v) => set("server", v)} placeholder="1.2.3.4:443" />
          <Field label="Server Name (SNI)" value={form.server_name} onChange={(v) => set("server_name", v)} placeholder="www.google.com" />

          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <label className="text-xs text-gray-400">Reality Secret (base64)</label>
              <button
                type="button"
                onClick={handleGenerateKeys}
                className="text-[10px] text-accent hover:text-accent-hover transition-colors"
              >
                Generate Keys
              </button>
            </div>
            <input
              type="text"
              value={form.secret}
              onChange={(e) => set("secret", e.target.value)}
              className="w-full bg-surface-2 border border-surface-4 rounded-lg px-3 py-2 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-accent/50"
              placeholder="base64..."
            />
          </div>

          <Field label="Short ID (hex, 16 chars)" value={form.short_id} onChange={(v) => set("short_id", v)} placeholder="0123456789abcdef" mono />

          <div className="grid grid-cols-2 gap-3">
            <Field label="Auth User (optional)" value={form.auth_user} onChange={(v) => set("auth_user", v)} placeholder="user" />
            <Field label="Auth Pass (optional)" value={form.auth_pass} onChange={(v) => set("auth_pass", v)} placeholder="pass" type="password" />
          </div>

          <div className="grid grid-cols-3 gap-3">
            <Field label="Listen Address" value={form.listen} onChange={(v) => set("listen", v)} placeholder="127.0.0.1:1080" />
            <Field
              label="Max TLS"
              value={String(form.max_tls_parallel)}
              onChange={(v) => set("max_tls_parallel", parseInt(v) || 12)}
              placeholder="12"
            />
            <Field
              label="Time Offset (s)"
              value={String(form.auth_time_offset_secs)}
              onChange={(v) => set("auth_time_offset_secs", parseInt(v) || 0)}
              placeholder="0"
            />
          </div>

          {error && (
            <p className="text-danger text-xs bg-danger/10 rounded-lg px-3 py-2">
              {error}
            </p>
          )}
        </div>

        <div className="px-6 py-4 border-t border-surface-3 flex justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            className="px-4 py-2 rounded-lg text-sm text-gray-400 hover:text-white hover:bg-surface-3 transition-colors"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={saving}
            className="px-5 py-2 rounded-lg text-sm font-medium bg-accent hover:bg-accent-hover transition-colors disabled:opacity-40"
          >
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </form>
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
  mono = false,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  mono?: boolean;
}) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-gray-400">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className={`w-full bg-surface-2 border border-surface-4 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-accent/50 ${
          mono ? "font-mono" : ""
        }`}
      />
    </div>
  );
}
