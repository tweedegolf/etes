import React from 'react';
import { Anchor, Badge, Tooltip } from '@mantine/core';

interface PullRequestProps {
  baseUrl: string;
  number: number;
  title?: string;
}

export function PullRequest({ baseUrl, number, title }: PullRequestProps) {
  const badge = (
    <Anchor href={`${baseUrl}/pull/${number}`} target="_blank">
      <Badge mr="md" className="cursor-pointer">#{number}</Badge>
    </Anchor>
  );

  if (!title) {
    return badge;
  }

  return (
    <Tooltip label={title}>{badge}</Tooltip>
  );
}
