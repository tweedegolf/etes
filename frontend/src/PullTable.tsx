import { Anchor, Avatar, Badge, Button, Card, Flex, Loader, Text, Table, Title, Tooltip, Box } from '@mantine/core';
import { Action, Pull, State, WorkflowStatus } from "./types";
import { RunButton } from './RunButton';
import React, { Dispatch } from 'react';
import { IconAlertTriangle, IconCheck, IconRefresh } from '@tabler/icons-react';
import DateTime from './DateTime';
import CommitHash from './CommitHash';
import { OpenButton } from './OpenButton';
import { PullRequest } from './PullRequest';
import { statusColor } from './util';
import { useMediaQuery } from '@mantine/hooks';

interface PullTableProps {
  state: State;
  dispatch: Dispatch<Action>;
}

function statusIcon(status: WorkflowStatus) {
  switch (status) {
    case 'SUCCESS':
      return <IconCheck size={14} />;
    case 'FAILURE':
    case 'ERROR':
      return <IconAlertTriangle size={14} />;
    case 'PENDING':
    case 'EXPECTED':
      return <Loader color={statusColor(status)} size={10} />;
  }
}

export function PullTable({ state, dispatch }: PullTableProps) {
  const isMobile = useMediaQuery(`(max-width: 768px)`);

  return (
    <Card withBorder mt="lg">
      <Flex justify="space-between">
        <Flex gap="md">
          <Title order={2} mb="md">
            Pull requests
          </Title>
          <Badge size="xl" variant="light">{state.github.pulls.length}</Badge>
        </Flex>
        <Button
          leftSection={<IconRefresh size={14} />}
          onClick={() => dispatch({ type: 'github_refresh' })}
          loading={state.githubLoading}
          variant="outline"
        >
          Sync pull requests
        </Button>
      </Flex>
      <Table verticalSpacing="xs" horizontalSpacing="xs" striped withTableBorder>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>PR</Table.Th>
            <Table.Th>Assignees</Table.Th>
            <Table.Th>Latest commit</Table.Th>
            <Table.Th>Build commit</Table.Th>
            <Table.Th>Created</Table.Th>
            <Table.Th>Checks</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>
          {state.github.pulls.map((pull: Pull) => (
            <Table.Tr key={pull.number}>
              <Table.Td>
                <PullRequest baseUrl={state.baseUrl} number={pull.number} />
                <Box display="inline-block">
                  <Anchor href={`${state.baseUrl}/pull/${pull.number}`} target="_blank">
                    <Text truncate="end" maw={isMobile ? '200px' : '500px'}>
                      {pull.title}
                    </Text>
                  </Anchor>
                </Box>
              </Table.Td>
              <Table.Td>
                {pull.assignees.map(assignee => (
                  <Tooltip label={assignee.name} key={assignee.login}>
                    <Avatar src={assignee.avatarUrl} alt={assignee.name} size="2rem" />
                  </Tooltip>
                ))}
              </Table.Td>
              <Table.Td>
                <CommitHash baseUrl={state.baseUrl} commitHash={pull.commit.hash} />
              </Table.Td>
              <Table.Td>
                {state.executables.filter((e) => e.triggerHash === pull.commit.hash).map((e) => (
                  <CommitHash key={e.hash} baseUrl={state.baseUrl} commitHash={e.hash} />
                ))}
              </Table.Td>
              <Table.Td>
                <DateTime date={pull.createdAt} />
              </Table.Td>
              <Table.Td>
                <Anchor
                  href={`${state.baseUrl}/pull/${pull.number}/checks`}
                  target="_blank"
                >
                  <Badge
                    variant="outline"
                    style={{ cursor: 'pointer' }}
                    color={statusColor(pull.status)}
                    leftSection={statusIcon(pull.status)}
                  >
                    {pull.status}
                  </Badge>
                </Anchor>
              </Table.Td>
              <Table.Td ta="right">
                <Flex gap="xs" justify="end" wrap="wrap">
                  <OpenButton state={state} commitHash={pull.commit.hash} />
                  {state.words.length > 0 && (
                    <RunButton
                      commit={pull.commit}
                      words={state.words}
                      services={state.services}
                      executables={state.executables}
                      dispatch={dispatch}
                    />
                  )}
                </Flex>
              </Table.Td>
            </Table.Tr>
          ))}
        </Table.Tbody>
      </Table>
    </Card>
  )
}
