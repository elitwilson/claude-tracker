import type { FileSystem } from "./lib.ts";

// Re-export FileSystem for consumers
export type { FileSystem };

/**
 * Metadata file structure
 * Tracks source paths for installed workflow files
 */
export interface Metadata {
  files: {
    [fileName: string]: {
      source: string; // e.g., "core/tdd-workflow.md" or "stacks/python/code-style.md"
      checksum?: string; // SHA-256 of content at install time, for detecting local modifications
    };
  };
}

/**
 * Loads metadata from .claude/.metadata.json
 * Returns empty metadata if file doesn't exist or is corrupted
 */
export async function loadMetadata(
  claudeDir: string,
  fs: FileSystem
): Promise<Metadata> {
  const metadataPath = `${claudeDir}/.metadata.json`;

  try {
    const exists = await fs.exists(metadataPath);
    if (!exists) {
      return { files: {} };
    }

    const content = await fs.readFile(metadataPath);
    return JSON.parse(content);
  } catch {
    // If file is corrupted or unreadable, return empty metadata
    return { files: {} };
  }
}

/**
 * Saves metadata to .claude/.metadata.json
 */
export async function saveMetadata(
  claudeDir: string,
  metadata: Metadata,
  fs: FileSystem
): Promise<void> {
  const metadataPath = `${claudeDir}/.metadata.json`;
  const content = JSON.stringify(metadata, null, 2);
  await fs.writeFile(metadataPath, content);
}

/**
 * Adds or updates a file entry in metadata
 */
export function addFileToMetadata(
  metadata: Metadata,
  fileName: string,
  sourcePath: string,
  checksum?: string
): void {
  metadata.files[fileName] = checksum
    ? { source: sourcePath, checksum }
    : { source: sourcePath };
}

/**
 * Gets the source path for a tracked file
 * Returns null if file is not tracked
 */
export function getFileSource(
  metadata: Metadata,
  fileName: string
): string | null {
  return metadata.files[fileName]?.source ?? null;
}

/**
 * Gets the checksum for a tracked file
 * Returns null if file is not tracked or has no checksum
 */
export function getFileChecksum(
  metadata: Metadata,
  fileName: string
): string | null {
  return metadata.files[fileName]?.checksum ?? null;
}
