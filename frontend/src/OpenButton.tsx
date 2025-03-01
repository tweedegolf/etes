import React from 'react';
import { State } from "./types";
import { Button } from '@mantine/core';
import { IconExternalLink, } from "@tabler/icons-react";
import { getServiceUrl } from "./util";

interface OpenButtonProps {
  state: State
  commitHash: string,
}

export function OpenButton({ state, commitHash }: OpenButtonProps) {
  const service = state.services.find((s) => s.executable.triggerHash === commitHash && s.state === 'running');

  if (!service) {
    return null;
  }

  return (
    <Button
      component="a"
      target="_blank"
      rightSection={<IconExternalLink size={14} />}
      color="darkblue"
      variant="light"
      href={getServiceUrl(service.name)}
    >
      Open
    </Button>
  );
}
