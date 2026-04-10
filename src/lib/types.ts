export interface Profile {
  id: string;
  name: string;
  server: string;
  server_name: string;
  secret: string;
  short_id: string;
  auth_user: string;
  auth_pass: string;
  listen: string;
  max_tls_parallel: number;
  auth_time_offset_secs: number;
}

export interface ProxyStatus {
  connected: boolean;
  profile_id: string | null;
  listen_addr: string | null;
  server: string | null;
  uptime_secs: number;
  connections: number;
}

export interface GeneratedKeys {
  secret: string;
  short_id: string;
}

export type LogLevel = "info" | "warn" | "error" | "debug";

export interface LogEntry {
  timestamp: number;
  level: LogLevel;
  message: string;
}
