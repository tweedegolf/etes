import { Alert, Box, Flex, MantineProvider, Title } from '@mantine/core';
import { useEtes } from "./useEtes";
import theme from './theme';
import { PullTable } from './PullTable';
import { ServiceTable } from './ServiceTable';
import Releases from './Releases';
import Server from './Server';
import { IconCircleX } from '@tabler/icons-react';
import React, { useEffect } from 'react';
import Commits from './Commits';

export function App() {
  const { state, dispatch, localDispatch } = useEtes();

  useEffect(() => {
    window.document.title = state.title;
  }, [state.title]);

  return (
    <MantineProvider theme={theme}>
      <Box p="md" maw={1600} mx="auto" miw="968">
        <Flex
          justify="space-between"
          align="center"
          mb="lg"
        >
          <Flex
            gap="md"
            justify="flex-start"
            align="center"
            direction="row"
          >
            <Title c="darkblue" order={1}>{state.title}</Title>
          </Flex>
        </Flex>
        <Flex gap="md" wrap="wrap">
          <Commits state={state} dispatch={dispatch} />
          <Releases state={state} dispatch={dispatch} />
          <Server state={state} />
        </Flex>
        <Box>
          {state.error && (
            <Alert
              icon={<IconCircleX size={16} />}
              color="red"
              title="Error"
              onClose={() => localDispatch({ type: 'clear_error' })}
              withCloseButton
              my="lg"
            >
              {state.error}
            </Alert>
          )}
          <PullTable state={state} dispatch={dispatch} />
          <ServiceTable state={state} dispatch={dispatch} />
        </Box>
      </Box>
    </MantineProvider >
  )
}
