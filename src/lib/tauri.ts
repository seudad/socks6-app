import { invoke } from "@tauri-apps/api/core";
import type { Profile, ProxyStatus, GeneratedKeys } from "./types";

export async function listProfiles(): Promise<Profile[]> {
  return invoke("list_profiles");
}

export async function saveProfile(profile: Profile): Promise<void> {
  return invoke("save_profile", { profile });
}

export async function deleteProfile(id: string): Promise<void> {
  return invoke("delete_profile", { id });
}

export async function connect(profileId: string): Promise<void> {
  return invoke("connect", { profileId });
}

export async function disconnect(): Promise<void> {
  return invoke("disconnect");
}

export async function getStatus(): Promise<ProxyStatus> {
  return invoke("get_status");
}

export async function setSystemProxy(
  enable: boolean,
  listenAddr: string,
): Promise<void> {
  return invoke("set_system_proxy", { enable, listenAddr });
}

export async function generateKeys(): Promise<GeneratedKeys> {
  return invoke("generate_keys");
}
