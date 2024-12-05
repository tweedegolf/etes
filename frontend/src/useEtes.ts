import { useEffect, useReducer } from 'react';
import { State, Action, Service } from './types';
import { randomString } from './util';

function reducer(state: State, action: Action) {
  if (action.type === 'stop_service') {
    const services = state.services.map((service) => {
      if (service.name === action.name) {
        return {
          ...service,
          state: 'stopping',
        };
      }

      return service;
    }) as Service[];

    return {
      ...state,
      services,
    };
  } else if (action.type === 'github_refresh') {
    return {
      ...state,
      githubLoading: true,
    };
  } else if (action.type === 'executables_state') {
    return {
      ...state,
      executables: action.executables,
    };
  } else if (action.type === 'initial_state') {
    return {
      ...state,
      isAdmin: action.isAdmin,
      user: action.user,
      title: action.title,
      words: action.words,
      memory: action.memory,
      baseUrl: action.baseUrl,
      executables: action.executables,
      githubLoading: false,
      github: action.github,
      services: action.services,
    };
  } else if (action.type === 'github_state') {
    return {
      ...state,
      githubLoading: false,
      github: action.payload,
    };
  } else if (action.type === 'service_state') {
    return {
      ...state,
      services: action.services,
    };
  } else if (action.type === 'websocket') {
    return {
      ...state,
      websocket: action.websocket,
    };
  } else if (action.type === 'error') {
    return {
      ...state,
      error: action.message,
    };
  } else if (action.type === 'clear_error') {
    return {
      ...state,
      error: null,
    };
  } else if (action.type === 'memory_state') {
    return {
      ...state,
      memory: {
        used: action.used,
        total: action.total,
      },
    };
  }

  return state;
}

// Initialize the caller id
export const caller = window.localStorage.getItem('caller_id') || randomString(24);
window.localStorage.setItem('caller_id', caller);

/**
 * Connect to the websocket server
 * @param localDispatch Local dispatch function
 * @returns void
 */
function connectWebsocket(localDispatch: (action: Action) => void) {
  const websocket = new WebSocket(`${window.location.protocol === 'http:' ? 'ws' : 'wss'}://${window.location.host}/etes/api/v1/ws/${caller}`);

  websocket.addEventListener("open", () => {
    localDispatch({ type: 'websocket', websocket });
  });

  websocket.addEventListener("message", (event) => {
    try {
      const action = JSON.parse(event.data);
      localDispatch(action);
    } catch (e) {
      console.error('Failed to parse message', e);
    }
  });

  websocket.addEventListener("close", () => {
    localDispatch({ type: 'websocket', websocket: null });
  });

  const interval = setInterval(() => {
    if (websocket.readyState === WebSocket.CLOSED) {
      console.log('Reconnecting websocket');
      clearInterval(interval);
      localDispatch({ type: 'websocket', websocket: connectWebsocket(localDispatch) });
    }
  }, 4000);

  return websocket;
}

export function useEtes() {
  const [state, localDispatch] = useReducer(reducer, {
    isAdmin: false,
    user: caller,
    words: [],
    title: document.title,
    githubLoading: false,
    baseUrl: '',
    github: { releases: [], pulls: [] },
    services: [],
    executables: [],
    websocket: null,
    error: null,
    memory: null,
  });

  // Fetch initial (github) state
  useEffect(() => {
    const controller = new AbortController();

    const fetchState = async () => {
      const response = await fetch(`/etes/api/v1/data/${caller}`, { signal: controller.signal });
      const data = await response.json();
      localDispatch({ type: 'initial_state', ...data });
    };

    fetchState();

    return () => controller.abort();
  }, []);

  const reconnect = () => connectWebsocket(localDispatch);

  // Create websocket connection
  useEffect(() => {
    reconnect()

    return () => state.websocket?.close()
  }, []);

  // Dispatch event to the server and the local reducer
  const dispatch = (action: Action) => {
    if (state.websocket?.readyState === WebSocket.OPEN) {
      state.websocket!.send(JSON.stringify({ ...action, user: state.user }));
    }

    localDispatch(action);
  };

  return {
    state,
    dispatch,
    localDispatch,
  };
}