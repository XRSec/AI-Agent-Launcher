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
