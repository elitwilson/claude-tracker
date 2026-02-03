/**
 * Deno runtime adapter for FileSystem operations
 * This file contains Deno-specific implementations and should only be imported
 * by Deno-based entry points (init.ts, upgrade.ts, add.ts)
 */

import type { FileSystem, DirEntry } from "./lib.ts";

/**
 * Gets the home directory from the HOME environment variable
 * Throws an error if HOME is not set
 */
export function getHomeDir(): string {
  const home = Deno.env.get("HOME");

  if (!home) {
    throw new Error("HOME environment variable is not set");
  }

  return home;
}

/**
 * Deno implementation of FileSystem interface
 */
export const denoFs: FileSystem = {
  async mkdir(path: string, recursive: boolean): Promise<void> {
    await Deno.mkdir(path, { recursive });
  },

  async exists(path: string): Promise<boolean> {
    try {
      await Deno.stat(path);
      return true;
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        return false;
      }
      throw error;
    }
  },

  async writeFile(path: string, content: string): Promise<void> {
    await Deno.writeTextFile(path, content);
  },

  async readFile(path: string): Promise<string> {
    return await Deno.readTextFile(path);
  },

  async readDir(path: string): Promise<DirEntry[]> {
    const entries: DirEntry[] = [];

    for await (const entry of Deno.readDir(path)) {
      entries.push({
        name: entry.name,
        isFile: entry.isFile,
        isDirectory: entry.isDirectory,
      });
    }

    return entries;
  },
};
