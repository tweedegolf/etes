import { GitHubUser, WorkflowStatus } from "./types";

export function formatFileSize(size: number) {
  if (size < 1024) {
    return `${size} bytes`;
  }

  if (size < 1024 ** 2) {
    return `${(size / 1024).toFixed(0)} KB`;
  }

  if (size < 1024 ** 3) {
    return `${(size / 1024 ** 2).toFixed(1)} MB`;
  }

  return `${(size / 1024 ** 3).toFixed(.1)} GB`;
}

export function getServiceUrl(name: string) {
  return `${window.location.protocol}//${name}.${window.location.host}`;
}

const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

export function randomString(length: number): string {
  let result = '';
  const charactersLength = characters.length;

  for (let i = 0; i < length; i++) {
    result += characters.charAt(Math.floor(Math.random() * charactersLength));
  }

  return result;
}

export function isGitHubUser(object: any): object is GitHubUser {
  if (object === null || typeof object !== 'object') {
    return false;
  }

  return 'login' in object && 'avatar_url' in object && 'name' in object;
}

export function statusColor(status: WorkflowStatus) {
  switch (status) {
    case 'SUCCESS':
      return 'green';
    case 'FAILURE':
      return 'red';
    case 'PENDING':
      return 'gray';
    case 'ERROR':
      return 'red';
    case 'EXPECTED':
      return 'blue';
  }
}

export function generateName(words: string[], len: number = 3): string {
  const randomWords: string[] = [];

  if (words.length < len) {
    return 'test';
  }

  while (randomWords.length < len) {
    const word = words[Math.floor(Math.random() * words.length)];

    if (!randomWords.includes(word)) {
      randomWords.push(word);
    }
  }

  return randomWords.join('-');
}