import { Anchor, Avatar, Badge, Button, Card, Flex, Table, Title, Text, Tooltip } from '@mantine/core';
import { Action, Service, State } from './types';
import React, { Dispatch } from 'react';
import { getServiceUrl, isGitHubUser } from './util';
import { IconExternalLink, IconHandStop } from '@tabler/icons-react';
import DateTime from './DateTime';
import CommitHash from './CommitHash';
import { PullRequest } from './PullRequest';

interface PullTableProps {
  state: State;
  dispatch: Dispatch<Action>;
}

function renderState(service: Service) {
  switch (service.state) {
    case 'running':
      return <Badge variant="outline" color="green">Running</Badge>
    case 'pending':
      return <Badge variant="outline" color="orange">Pending</Badge>
    case 'error':
      return (
        <Tooltip label={service.error}>
          <Badge variant="outline" color="red">Error</Badge>
        </Tooltip>
      );
    default:
      return <Badge variant="outline" color="grey">Unknown</Badge>
  }
}

export function ServiceTable({ state, dispatch }: PullTableProps) {
  if (state.services.length === 0) {
    return null;
  }

  return (
    <Card withBorder mt="lg">
      <Flex gap="md">
        <Title order={2} mb="md">
          Running services
        </Title>
        <Badge size="xl" variant="light">{state.services.length}</Badge>
      </Flex>
      <Table verticalSpacing="xs" horizontalSpacing="xs" striped withTableBorder>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>URL</Table.Th>
            <Table.Th>Creator</Table.Th>
            <Table.Th>Commit</Table.Th>
            <Table.Th>Created</Table.Th>
            <Table.Th>State</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>
          {state.services.map((service: Service) => (
            <Table.Tr key={service.name}>
              <Table.Td>
                <Anchor href={getServiceUrl(service.name)} target="_blank">
                  {getServiceUrl(service.name)}
                </Anchor>
              </Table.Td>
              <Table.Td>
                {isGitHubUser(service.creator) ? (
                  <Tooltip label={service.creator.name} key={service.creator.login}>
                    <Avatar src={service.creator.avatar_url} alt={service.creator.name} size="2rem" />
                  </Tooltip>
                ) : (
                  <Text size="xs" c="dimmed">anonymous</Text>
                )}
              </Table.Td>
              <Table.Td>
                <Flex gap="sm">
                  <CommitHash baseUrl={state.baseUrl} commitHash={service.executable.hash} />
                  {state.github.pulls
                    .filter((pull) => pull.commit.hash === service.executable.triggerHash)
                    .map((pull) => (
                      <PullRequest
                        baseUrl={state.baseUrl}
                        number={pull.number}
                        title={pull.title}
                      />
                    ))}
                </Flex>
              </Table.Td>
              <Table.Td>
                <DateTime date={service.createdAt} />
              </Table.Td>
              <Table.Td>{renderState(service)}</Table.Td>
              <Table.Td>
                <Flex gap="xs" justify="end" wrap="wrap">
                  {(service.creator === state.user || state.isAdmin) && (
                    <Button
                      leftSection={<IconHandStop size={14} />}
                      color={service.state === 'running' ? 'red' : 'gray'}
                      variant="outline"
                      onClick={() => dispatch({
                        type: 'stop_service',
                        name: service.name,
                      })}
                    >
                      Stop
                    </Button>
                  )}
                  {service?.state === 'running' && (
                    <Button
                      component="a"
                      target="_blank"
                      rightSection={<IconExternalLink size={14} />}
                      color="darkblue"
                      variant="light"
                      href={getServiceUrl(service.name)}
                      loading={service.state !== 'running'}
                    >
                      Open
                    </Button>
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
