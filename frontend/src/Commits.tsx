import { Title, Text, Card, Anchor, Flex, Table, Tooltip } from '@mantine/core';
import { Action, State } from "./types";
import React, { Dispatch } from 'react';
import DateTime from './DateTime';
import CommitHash from './CommitHash';
import { RunButton } from './RunButton';
import { OpenButton } from './OpenButton';

interface CommitsProps {
  state: State;
  dispatch: Dispatch<Action>;
}

export default function Commits({ state, dispatch }: CommitsProps) {
  return (
    <>
      {state.github.commits.map((commit) => (
        <Card key={commit.hash} withBorder>
          <Flex direction="column" gap="sm">
            <Title order={3}>
              Main build
            </Title>
            <Table verticalSpacing="xs" horizontalSpacing="xs" maw="100%" striped withTableBorder>
              <Table.Tbody>
                <Table.Tr>
                  <Table.Th scope="row">Message</Table.Th>
                  <Table.Td maw="180px">
                    <Anchor href={commit.url} target="_blank" className="no-wrap">
                      <Tooltip label={commit.message}>
                        <Text size="sm" truncate="end">
                          {commit.message}
                        </Text>
                      </Tooltip>
                    </Anchor>
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th scope="row">Created</Table.Th>
                  <Table.Td>
                    <DateTime date={commit.date} />
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th scope="row">Hash</Table.Th>
                  <Table.Td>
                    <CommitHash baseUrl={state.baseUrl} commitHash={commit.hash} />
                  </Table.Td>
                </Table.Tr>
              </Table.Tbody>
            </Table>
            <Flex gap="xs" wrap="wrap" justify="end">
              <OpenButton
                state={state}
                commitHash={commit.hash}
              />
              {state.words.length > 0 && (
                <RunButton
                  commit={commit}
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
