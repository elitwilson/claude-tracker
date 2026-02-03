/**
 * GitHub API directory discovery utilities
 */

export interface WorkflowFile {
  name: string;
  path: string;
}

export interface Stack {
  name: string;
  displayName: string;
}

interface GitHubContentItem {
  name: string;
  path: string;
  type: "file" | "dir";
}

/**
 * Fetches directory contents from GitHub API
 */
async function fetchGitHubDirectory(
  owner: string,
  repo: string,
  path: string,
  branch: string
): Promise<GitHubContentItem[]> {
  const url = `https://api.github.com/repos/${owner}/${repo}/contents/${path}?ref=${branch}`;

  const response = await fetch(url);

  if (!response.ok) {
    throw new Error(
      `Failed to fetch ${path}: HTTP ${response.status} ${response.statusText}`
    );
  }

  return await response.json();
}

/**
 * Discovers core workflow files from the core/ directory
 */
export async function discoverCoreWorkflows(
  owner: string,
  repo: string,
  branch: string
): Promise<WorkflowFile[]> {
  const items = await fetchGitHubDirectory(owner, repo, "core", branch);

  return items
    .filter((item) => item.type === "file")
    .filter((item) => item.name.endsWith(".md"))
    .filter((item) => item.name !== "README.md")
    .map((item) => ({
      name: item.name,
      path: item.path,
    }));
}

/**
 * Discovers available language stacks from the stacks/ directory
 */
export async function discoverStacks(
  owner: string,
  repo: string,
  branch: string
): Promise<Stack[]> {
  const items = await fetchGitHubDirectory(owner, repo, "stacks", branch);

  return items
    .filter((item) => item.type === "dir")
    .map((item) => ({
      name: item.name,
      displayName: item.name,
    }));
}

/**
 * Discovers workflow files for a specific stack
 */
export async function discoverStackWorkflows(
  owner: string,
  repo: string,
  branch: string,
  stackName: string
): Promise<WorkflowFile[]> {
  const items = await fetchGitHubDirectory(
    owner,
    repo,
    `stacks/${stackName}`,
    branch
  );

  return items
    .filter((item) => item.type === "file")
    .filter((item) => item.name.endsWith(".md"))
    .filter((item) => item.name !== "README.md")
    .map((item) => ({
      name: item.name,
      path: item.path,
    }));
}
