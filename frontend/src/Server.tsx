import { Card, Title, Table, Flex, Button, Avatar, Tooltip, Text } from '@mantine/core';
import { State } from './types';
import { formatFileSize, isGitHubUser } from './util';
import { ConnectionState } from './ConnectionState';
import { IconBrandGithub, IconLogout } from '@tabler/icons-react';

interface ServerProps {
  state: State;
}

export default function Server({ state }: ServerProps) {
  return (
    <Card withBorder key="server">
      <Flex direction="column" gap="sm">
        <Title order={3}>Server</Title>
        <Table verticalSpacing="xs" horizontalSpacing="xs" striped withTableBorder>
          <Table.Tbody>
            <Table.Tr>
              <Table.Th scope="row">Live updates</Table.Th>
              <Table.Td>
                <ConnectionState state={state} />
              </Table.Td>
            </Table.Tr>
            <Table.Tr>
              <Table.Th scope="row">Running services</Table.Th>
              <Table.Td>
                {state.services.length}
              </Table.Td>
            </Table.Tr>
            {state.memory && (
              <Table.Tr>
                <Table.Th scope="row">Memory usage</Table.Th>
                <Table.Td>
                  {formatFileSize(state.memory.used)} / {formatFileSize(state.memory.total)}
                </Table.Td>
              </Table.Tr>
            )}
          </Table.Tbody>
        </Table>
        <Flex gap="xs" wrap="wrap" justify="end">
          {isGitHubUser(state.user) ? (
            <Flex gap="xs" align="center">
              <Text c="dimmed" size="sm">Logged in as:</Text>
              <Tooltip label={state.user.name} key={state.user.login}>
                <Avatar src={state.user.avatar_url} alt={state.user.name} size="2rem" />
              </Tooltip>
              <Button
                size="sm"
                component="a"
                href="/etes/logout"
                leftSection={<IconLogout size={14} />}
                color="darkblue"
                variant="outline"
                onClick={() => window.location.reload()}
              >
                Logout
              </Button>
            </Flex>
          ) : (
            <Button
              component="a"
              href="/etes/login"
              leftSection={<IconBrandGithub size={14} />}
              color="darkblue"
              variant="outline"
              onClick={() => window.location.reload()}
            >
              Login
            </Button>
          )}
        </Flex>
      </Flex>
    </Card >
  );
}   