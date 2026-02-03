/**
 * Directory entry information
 */
export interface DirEntry {
  name: string;
  isFile: boolean;
  isDirectory: boolean;
}

/**
 * FileSystem abstraction for cross-runtime compatibility
 * Implementations: deno-fs.ts (Deno), node-fs.ts (Node.js/npx)
 */
export interface FileSystem {
  mkdir(path: string, recursive: boolean): Promise<void>;
  exists(path: string): Promise<boolean>;
  writeFile(path: string, content: string): Promise<void>;
  readFile(path: string): Promise<string>;
  readDir(path: string): Promise<DirEntry[]>;
}

/**
 * Logger function type
 */
export type LogFn = (message: string) => void;

/**
 * Scope type for directory resolution
 */
export type Scope = "project" | "global";

/**
 * Frontmatter metadata structure
 */
export interface Frontmatter {
  version: string;
  updated: string;
}

/**
 * Builds the GitHub API base URL for a given repo.
 * Single source of truth — call sites pass owner/repo, not a URL.
 */
export function buildRepoUrl(owner: string, repo: string): string {
  return `https://api.github.com/repos/${owner}/${repo}`;
}

/**
 * Fetches a workflow file from the GitHub Contents API.
 * Uses api.github.com instead of raw.githubusercontent.com to avoid
 * Fastly CDN caching (which has a 5-minute TTL that query-param busters don't bypass).
 */
export async function fetchWorkflowFile(
  repoUrl: string,
  branch: string,
  filePath: string
): Promise<string> {
  const url = `${repoUrl}/contents/${filePath}?ref=${branch}`;

  try {
    const response = await fetch(url);

    if (!response.ok) {
      throw new Error(
        `Failed to fetch ${filePath}: HTTP ${response.status} ${response.statusText}`
      );
    }

    const data = await response.json();
    // GitHub API returns base64 content with newlines every 60 chars
    return atob(data.content.replace(/\n/g, ""));
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("Failed to fetch")) {
      throw error;
    }
    throw new Error(
      `Failed to fetch ${filePath}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Ensures a directory exists, creating it if necessary
 * Portable - works with any FileSystem implementation
 */
export async function ensureDirectory(
  path: string,
  dryRun: boolean,
  fs: FileSystem,
  log: LogFn
): Promise<void> {
  if (dryRun) {
    log(`Would create directory: ${path}`);
    return;
  }

  const dirExists = await fs.exists(path);
  if (!dirExists) {
    await fs.mkdir(path, true);
    log(`Created directory: ${path}`);
  }
}

/**
 * Writes a workflow file to the specified path
 * Portable - works with any FileSystem implementation
 */
export async function writeWorkflowFile(
  path: string,
  content: string,
  dryRun: boolean,
  fs: FileSystem,
  log: LogFn
): Promise<void> {
  if (dryRun) {
    log(`Would write file: ${path}`);
    return;
  }

  await fs.writeFile(path, content);
  log(`Wrote file: ${path}`);
}

/**
 * Parses YAML frontmatter from markdown file content
 * Returns null if frontmatter is missing or invalid
 */
export function parseFrontmatter(content: string): Frontmatter | null {
  // Normalize line endings to \n
  const normalized = content.replace(/\r\n/g, "\n");

  // Match frontmatter block: --- at start, content, then ---
  const frontmatterRegex = /^---\n([\s\S]*?)\n---/;
  const match = normalized.match(frontmatterRegex);

  if (!match) {
    return null;
  }

  const frontmatterBlock = match[1];

  // Extract version and updated fields
  const versionMatch = frontmatterBlock.match(/^version:\s*(.+)$/m);
  const updatedMatch = frontmatterBlock.match(/^updated:\s*(.+)$/m);

  if (!versionMatch) {
    return null;
  }

  return {
    version: versionMatch[1].trim(),
    updated: updatedMatch ? updatedMatch[1].trim() : "",
  };
}

/**
 * Calculates SHA-256 checksum of content
 * Uses Web Crypto API for cross-platform compatibility
 */
export async function calculateChecksum(content: string): Promise<string> {
  // Convert string to Uint8Array
  const encoder = new TextEncoder();
  const data = encoder.encode(content);

  // Calculate SHA-256 hash
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);

  // Convert ArrayBuffer to hex string
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map(b => b.toString(16).padStart(2, "0")).join("");

  return hashHex;
}

/**
 * Discovers installed workflow files in the specified directory
 * Returns array of file paths for all .md files
 */
export async function discoverInstalledWorkflows(
  rulesDir: string,
  fs: FileSystem
): Promise<string[]> {
  // Check if directory exists
  const dirExists = await fs.exists(rulesDir);
  if (!dirExists) {
    return [];
  }

  // Read directory contents
  const entries = await fs.readDir(rulesDir);

  // Filter for .md files only
  const mdFiles = entries
    .filter(entry => entry.isFile && entry.name.endsWith(".md"))
    .map(entry => `${rulesDir}/${entry.name}`);

  return mdFiles;
}

/**
 * Expands home directory path (~) to the actual home directory
 * Examples:
 *   "~" -> "/Users/username"
 *   "~/.claude" -> "/Users/username/.claude"
 *   "/absolute/path" -> "/absolute/path" (unchanged)
 */
export function resolveHomePath(path: string, homeDir: string): string {
  if (path === "~") {
    return homeDir;
  }

  if (path.startsWith("~/")) {
    return homeDir + path.slice(1);
  }

  return path;
}

/**
 * Determines the target .claude directory based on installation scope
 * - project scope: {projectRoot}/.claude
 * - global scope: {homeDir}/.claude
 */
export function getTargetDirectory(
  scope: Scope,
  projectRoot: string,
  homeDir: string
): string {
  if (scope === "project") {
    return `${projectRoot}/.claude`;
  }

  return `${homeDir}/.claude`;
}

/**
 * Checks if a file already exists in the target directory
 * Used for duplicate detection within the same scope
 */
export async function checkDuplicateFile(
  fileName: string,
  rulesDir: string,
  fs: FileSystem
): Promise<boolean> {
  const filePath = `${rulesDir}/${fileName}`;
  return await fs.exists(filePath);
}
