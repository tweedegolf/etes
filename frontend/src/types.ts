
export type ServiceState = 'pending' | 'running' | 'error';

export type WorkflowStatus = 'PENDING' | 'ERROR' | 'EXPECTED' | 'FAILURE' | 'SUCCESS';

export interface Commit {
  date: string;
  hash: string;
}

export interface Executable {
  hash: string;
  triggerHash: string;
}

export interface Release {
  name: string;
  url: string;
  tagName: string,
  createdAt: string;
  commit: Commit;
}

export interface Assignee {
  avatarUrl: string,
  login: string,
  name: string,
}

export interface Pull {
  number: number,
  createdAt: string,
  isDraft: boolean,
  title: string,
  url: string,
  assignees: Assignee[],
  status: WorkflowStatus,
  commit: Commit,
}

export interface GitHubState {
  releases: Release[],
  pulls: Pull[];
}

export interface State {
  isAdmin: boolean,
  user: User,
  title: string;
  words: string[];
  github: GitHubState;
  githubLoading: boolean;
  baseUrl: string;
  websocket: WebSocket | null;
  services: Service[];
  executables: Executable[];
  error: string | null;
  memory: null | {
    used: number;
    total: number;
  };
}

export interface Service {
  name: string;
  port?: number;
  executable: Executable,
  creator: User;
  createdAt: string,
  state: ServiceState;
  error?: string | null;
}

export interface GitHubUser {
  avatar_url: string,
  login: string,
  name: string,
}

export type User = string | GitHubUser;

export type Action = {
  type: 'initial_state',
  isAdmin: boolean,
  user: User,
  title: string,
  executables: Executable[],
  baseUrl: string,
  words: string[],
  memory: {
    used: number;
    total: number;
  };
  github: GitHubState,
  services: Service[],
} | {
  type: 'service_state',
  services: Service[],
} | {
  type: 'executables_state',
  executables: Executable[],
} | {
  type: 'github_state',
  payload: GitHubState
} | {
  type: 'memory_state',
  used: number,
  total: number,
} | {
  type: 'github_refresh',
  // added to the event on websocket forward
  user?: User,
} | {
  type: 'start_service',
  executable: Executable,
  name: string,
  // added to the event on websocket forward
  user?: User,
} | {
  type: 'stop_service',
  name: string,
  // added to the event on websocket forward
  user?: User,
} | {
  type: 'websocket',
  websocket: WebSocket | null,
} | {
  type: 'error',
  message: string,
  caller: string,
} | {
  type: 'clear_error',
};