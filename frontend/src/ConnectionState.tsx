import React from 'react';
import { State } from "./types";
import { Badge, Loader } from '@mantine/core';
import { IconLine, IconPlugConnectedX } from '@tabler/icons-react';

interface ConnectionStateProps {
  state: State;
}

export function ConnectionState({ state }: ConnectionStateProps) {
  const connected = state.websocket?.readyState === WebSocket.OPEN;

  return (
    <Badge
      variant="outline"
      leftSection={connected ? <IconLine size={14} /> : <IconPlugConnectedX size={14} />}
      rightSection={!connected && <Loader color="gray" size={12} />}
      color={state.websocket?.readyState === WebSocket.OPEN ? 'green.8' : 'gray'}
    >
      {!state.websocket && 'Disconnected'}
      {state.websocket?.readyState === WebSocket.OPEN && 'Connected'}
      {state.websocket?.readyState === WebSocket.CONNECTING && 'Connecting'}
      {state.websocket?.readyState === WebSocket.CLOSED && 'Closed'}
      {state.websocket?.readyState === WebSocket.CLOSING && 'Closing'}
    </Badge>
  )
}
