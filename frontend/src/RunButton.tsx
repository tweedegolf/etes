import { Action, Commit, Executable, Service } from "./types";
import { Button } from '@mantine/core';
import { IconRocket } from "@tabler/icons-react";
import React, { Dispatch, useEffect, useState } from 'react';
import { generateName, getServiceUrl } from "./util";

interface RunButtonProps {
  commit: Commit;
  words: string[];
  services: Service[];
  executables: Executable[];
  dispatch: Dispatch<Action>;
}

export function RunButton({ commit, words, services, executables, dispatch }: RunButtonProps) {
  const [name, setName] = useState(generateName(words));
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (loading) {
      const service = services.find((s) => s.executable.triggerHash === commit.hash && s.name === name);

      if (service && service.state === 'running') {
        window.open(getServiceUrl(service.name), '_blank');
        setLoading(false);
        setName(generateName(words));
      }
    }
  }, [loading, services]);

  const executable = executables.find((e: Executable) => e.triggerHash === commit.hash);

  if (!executable) {
    return (
      <Button
        color="gray"
        disabled
      >
        Executable not found
      </Button>
    );
  }

  const onclick = () => {
    setLoading(true);
    dispatch({
      type: 'start_service',
      executable,
      name,
    });
  }

  return (
    <Button
      leftSection={<IconRocket size="24" />}
      color="darkblue"
      onClick={onclick}
      loading={loading}
    >
      Create new
    </Button>
  );
}
