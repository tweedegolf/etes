import { Title, Text, Card, Anchor, Flex, Table } from '@mantine/core';
import { Action, State } from "./types";
import { Dispatch } from 'preact/hooks';
import DateTime from './DateTime';
import CommitHash from './CommitHash';
import { RunButton } from './RunButton';
import { OpenButton } from './OpenButton';

interface ReleasesProps {
  state: State;
  dispatch: Dispatch<Action>;
}

export default function Releases({ state, dispatch }: ReleasesProps) {
  return (
    <>
      {state.github.releases.map((release) => (
        <Card key={release.tagName} withBorder>
          <Flex direction="column" gap="sm">
            <Title order={3}>
              Release build
            </Title>
            <Table verticalSpacing="xs" horizontalSpacing="xs" striped withTableBorder>
              <Table.Tbody>
                <Table.Tr>
                  <Table.Th scope="row">Tag</Table.Th>
                  <Table.Td>
                    <Anchor href={release.url} target="_blank" style={{ whiteSpace: 'nowrap' }}>
                      <Text size="sm">
                        {release.tagName}
                      </Text>
                    </Anchor>
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th scope="row">Created</Table.Th>
                  <Table.Td>
                    <DateTime date={release.createdAt} />
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th scope="row">Commit</Table.Th>
                  <Table.Td>
                    <CommitHash baseUrl={state.baseUrl} commitHash={release.commit.hash} />
                    (<DateTime date={release.commit.date} />)
                  </Table.Td>
                </Table.Tr>
              </Table.Tbody>
            </Table>
            <Flex gap="xs" wrap="wrap" justify="end">
              <OpenButton
                state={state}
                commitHash={release.commit.hash}
              />
              {state.words.length > 0 && (
                <RunButton
                  commit={release.commit}
                  words={state.words}
                  services={state.services}
                  executables={state.executables}
                  dispatch={dispatch}
                />
              )}
            </Flex>
          </Flex>
        </Card >
      ))
      }
    </>
  );
}
