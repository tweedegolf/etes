import React from 'react';
import { Tooltip } from "@mantine/core";

const DATE_FORMAT = new Intl.DateTimeFormat('sv-SE', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
});

function timeAgo(date: Date): string {
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) {
    return `${days} day${days > 1 ? 's' : ''} ago`;
  }

  if (hours > 0) {
    return `${hours} hour${hours > 1 ? 's' : ''} ago`;
  }

  if (minutes > 0) {
    return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
  }

  return `${seconds} second${seconds > 1 ? 's' : ''} ago`;
}

export default function DateTime({ date }: { date: string }) {
  // international date / time format (apparently Sweden does it right)
  const full = DATE_FORMAT.format(new Date(date));

  return (
    <Tooltip label={full}>
      <time dateTime={date} style={{ cursor: 'default' }}>{timeAgo(new Date(date))}</time>
    </Tooltip>
  );
}
