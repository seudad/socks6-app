import { useEffect, useState, useCallback } from "react";
import * as api from "../lib/tauri";
import type { Profile } from "../lib/types";
import ProfileForm from "../components/ProfileForm";

export default function Profiles() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [editing, setEditing] = useState<Profile | null>(null);
  const [creating, setCreating] = useState(false);

  const refresh = useCallback(async () => {
    const list = await api.listProfiles();
    setProfiles(list);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleDelete = async (id: string) => {
    await api.deleteProfile(id);
    refresh();
  };

  const handleSaved = () => {
    setEditing(null);
    setCreating(false);
    refresh();
  };

  return (
    <div className="p-6 max-w-3xl mx-auto space-y-5">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Profiles</h2>
        <button
          onClick={() => setCreating(true)}
          className="px-4 py-2 rounded-lg text-sm font-medium bg-accent hover:bg-accent-hover transition-colors"
        >
          + New Profile
        </button>
      </div>

      {profiles.length === 0 && (
        <div className="text-center py-16">
          <p className="text-muted text-sm">
            No server profiles yet. Create one to get started.
          </p>
        </div>
      )}

      <div className="space-y-3">
        {profiles.map((p) => (
          <div
            key={p.id}
            className="bg-surface-1 border border-surface-3 rounded-xl p-4 flex items-center justify-between group"
          >
            <div className="min-w-0 flex-1">
              <p className="font-medium text-sm truncate">{p.name}</p>
              <p className="text-xs text-muted mt-0.5 font-mono truncate">
                {p.server} &middot; SNI: {p.server_name}
              </p>
              <p className="text-[10px] text-muted mt-0.5 font-mono truncate">
                Listen: {p.listen}
                {p.auth_user ? ` · Auth: ${p.auth_user}` : ""}
              </p>
            </div>
            <div className="flex gap-2 shrink-0 ml-4 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                onClick={() => setEditing(p)}
                className="px-3 py-1.5 rounded-lg text-xs bg-surface-3 hover:bg-surface-4 transition-colors"
              >
                Edit
              </button>
              <button
                onClick={() => handleDelete(p.id)}
                className="px-3 py-1.5 rounded-lg text-xs text-danger bg-danger/10 hover:bg-danger/20 transition-colors"
              >
                Delete
              </button>
            </div>
          </div>
        ))}
      </div>

      {(creating || editing) && (
        <ProfileForm
          initial={editing ?? undefined}
          onSave={handleSaved}
          onCancel={() => {
            setCreating(false);
            setEditing(null);
          }}
        />
      )}
    </div>
  );
}
