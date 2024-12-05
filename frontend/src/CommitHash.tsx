import { Anchor, Code, Tooltip } from "@mantine/core";

export default function CommitHash({ baseUrl, commitHash }: { baseUrl: string, commitHash: string }) {
  return (
    <Anchor
      href={`${baseUrl}/commit/${commitHash}`}
      target="_blank"
      mr="sm"
    >
      <Tooltip label={commitHash}>
        <Code className="commit">
          {commitHash.slice(0, 8)}
        </Code>
      </Tooltip>
    </Anchor>
  );
}
