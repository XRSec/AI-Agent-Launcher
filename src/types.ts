export type RestartPolicy = "no" | "always" | "on-failure";

export interface AgentProfile {
  id: string;
  name: string;
  command: string;
  shell: string;
  workingDirectory: string;
  customPath: string;
  restartPolicy: RestartPolicy;
  restartDelay: number;
  autoStart: boolean;
}

export interface ProfileStatus {
  profileId: string;
  isRunning: boolean;
  statusText: string;
  pid: number | null;
  lastError: string | null;
}

export interface ProfileLogMessage {
  profileId: string;
  text: string;
  isError: boolean;
}

export interface UpdateInfo {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  release_name: string;
  release_notes: string;
  release_url: string;
  published_at: string;
  asset_name: string | null;
  asset_download_url: string | null;
  asset_size: number | null;
}

export interface DownloadProgressPayload {
  downloaded: number;
  total: number;
  percent: number;
}
