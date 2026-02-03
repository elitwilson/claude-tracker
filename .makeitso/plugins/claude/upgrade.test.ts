import { describe, it, beforeEach, afterEach } from "https://deno.land/std@0.224.0/testing/bdd.ts";
import { assertEquals, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import {
  calculateChecksum,
  type FileSystem,
} from "./lib.ts";
import { performUpgrade, isNewerVersion } from "./upgrade.ts";

/**
 * Upgrade workflow integration tests
 * These test the orchestration logic for the upgrade command
 */

describe("upgrade workflow integration", () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("detects when remote version is newer than local version", () => {
    assertEquals(isNewerVersion("0.1.0", "0.2.0"), true);
    assertEquals(isNewerVersion("0.2.0", "0.1.0"), false);
    assertEquals(isNewerVersion("0.1.0", "0.1.0"), false);
    assertEquals(isNewerVersion("0.9.0", "0.10.0"), true);
    assertEquals(isNewerVersion("1.0.0", "2.0.0"), true);
  });

  it("skips files that are already up to date", async () => {
    const localContent = `---
version: 0.2.0
updated: 2026-01-30
---
# Content`;

    // Remote returns same version — fetch will be called but version check skips upgrade
    globalThis.fetch = async () => {
      return new Response(JSON.stringify({ content: btoa(localContent) }), { status: 200 });
    };

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules" || path === ".claude/.metadata.json",
      writeFile: async () => {},
      readFile: async (path: string) => {
        if (path === ".claude/.metadata.json") {
          return JSON.stringify({ files: { "tdd-workflow.md": { source: "core/tdd-workflow.md" } } });
        }
        if (path === ".claude/rules/tdd-workflow.md") {
          return localContent;
        }
        return "";
      },
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [{ name: "tdd-workflow.md", isFile: true, isDirectory: false }];
        }
        return [];
      },
    };

    const result = await performUpgrade(
      ".claude/rules",
      ".claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      false,
      mockFs
    );

    assertEquals(result.skipped.length, 1);
    assertEquals(result.skipped[0].reason.includes("Already up to date"), true);
  });

  it("detects local modifications by checksum comparison", async () => {
    const originalContent = `---
version: 0.1.0
updated: 2026-01-30
---
# Original content`;

    const modifiedContent = `---
version: 0.1.0
updated: 2026-01-30
---
# Modified by user`;

    const originalChecksum = await calculateChecksum(originalContent);
    const modifiedChecksum = await calculateChecksum(modifiedContent);

    assertEquals(originalChecksum !== modifiedChecksum, true);
  });

  it("skips modified files without --force flag", async () => {
    // Local file was modified after install — checksum won't match the stored one
    const localContent = `---
version: 0.1.0
updated: 2026-01-30
---
# User modified this file`;

    const remoteContent = `---
version: 0.2.0
updated: 2026-01-31
---
# Updated content`;

    globalThis.fetch = async () => {
      return new Response(JSON.stringify({ content: btoa(remoteContent) }), { status: 200 });
    };

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules" || path === ".claude/.metadata.json",
      writeFile: async () => {},
      readFile: async (path: string) => {
        if (path === ".claude/.metadata.json") {
          // Stored checksum is from original install — does NOT match localContent
          return JSON.stringify({ files: { "tdd-workflow.md": { source: "core/tdd-workflow.md", checksum: "b7c07757233df2f01a909b4d5e6c926ac190f2cf45ee313a6f1368adfb3919eb" } } });
        }
        if (path === ".claude/rules/tdd-workflow.md") {
          return localContent;
        }
        return "";
      },
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [{ name: "tdd-workflow.md", isFile: true, isDirectory: false }];
        }
        return [];
      },
    };

    const result = await performUpgrade(
      ".claude/rules",
      ".claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false, // force=false
      false,
      mockFs
    );

    assertEquals(result.skipped.length, 1);
    assertEquals(result.skipped[0].reason.includes("Modified locally"), true);
  });

  it("updates modified files with --force flag", async () => {
    const localContent = `---
version: 0.1.0
updated: 2026-01-30
---
# User modified this file`;

    const remoteContent = `---
version: 0.2.0
updated: 2026-01-31
---
# Updated content`;

    globalThis.fetch = async () => {
      return new Response(JSON.stringify({ content: btoa(remoteContent) }), { status: 200 });
    };

    const writtenFiles = new Map<string, string>();

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules" || path === ".claude/.metadata.json",
      writeFile: async (path: string, content: string) => {
        writtenFiles.set(path, content);
      },
      readFile: async (path: string) => {
        if (path === ".claude/.metadata.json") {
          return JSON.stringify({ files: { "tdd-workflow.md": { source: "core/tdd-workflow.md", checksum: "b7c07757233df2f01a909b4d5e6c926ac190f2cf45ee313a6f1368adfb3919eb" } } });
        }
        if (path === ".claude/rules/tdd-workflow.md") {
          return localContent;
        }
        return "";
      },
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [{ name: "tdd-workflow.md", isFile: true, isDirectory: false }];
        }
        return [];
      },
    };

    const result = await performUpgrade(
      ".claude/rules",
      ".claude",
      "https://api.github.com/repos/test/repo",
      "main",
      true, // force=true
      false,
      mockFs
    );

    assertEquals(result.upgraded.length, 1);
    assertEquals(result.upgraded[0], "tdd-workflow.md");
    assertEquals(writtenFiles.get(".claude/rules/tdd-workflow.md"), remoteContent);
  });

  it("upgrades without checking modification when no stored checksum exists", async () => {
    // Metadata has no checksum — file was installed before checksum tracking existed.
    // Should upgrade without blocking, same as isModified=false.
    const localContent = `---
version: 0.1.0
updated: 2026-01-30
---
# Some content`;

    const remoteContent = `---
version: 0.2.0
updated: 2026-01-31
---
# Updated content`;

    globalThis.fetch = async () => {
      return new Response(JSON.stringify({ content: btoa(remoteContent) }), { status: 200 });
    };

    const writtenFiles = new Map<string, string>();

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules" || path === ".claude/.metadata.json",
      writeFile: async (path: string, content: string) => {
        writtenFiles.set(path, content);
      },
      readFile: async (path: string) => {
        if (path === ".claude/.metadata.json") {
          // No checksum field — legacy metadata
          return JSON.stringify({ files: { "tdd-workflow.md": { source: "core/tdd-workflow.md" } } });
        }
        if (path === ".claude/rules/tdd-workflow.md") {
          return localContent;
        }
        return "";
      },
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [{ name: "tdd-workflow.md", isFile: true, isDirectory: false }];
        }
        return [];
      },
    };

    const result = await performUpgrade(
      ".claude/rules",
      ".claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      false,
      mockFs
    );

    assertEquals(result.upgraded.length, 1);
    assertEquals(result.upgraded[0], "tdd-workflow.md");
    assertEquals(writtenFiles.get(".claude/rules/tdd-workflow.md"), remoteContent);
  });

  it("updates unmodified files with newer versions", async () => {
    const localContent = `---
version: 0.1.0
updated: 2026-01-30
---
# Original content`;

    const remoteContent = `---
version: 0.2.0
updated: 2026-01-31
---
# Updated content`;

    globalThis.fetch = async () => {
      return new Response(JSON.stringify({ content: btoa(remoteContent) }), { status: 200 });
    };

    const writtenFiles = new Map<string, string>();

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules" || path === ".claude/.metadata.json",
      writeFile: async (path: string, content: string) => {
        writtenFiles.set(path, content);
      },
      readFile: async (path: string) => {
        if (path === ".claude/.metadata.json") {
          return JSON.stringify({ files: { "tdd-workflow.md": { source: "core/tdd-workflow.md" } } });
        }
        if (path === ".claude/rules/tdd-workflow.md") {
          return localContent;
        }
        return "";
      },
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [{ name: "tdd-workflow.md", isFile: true, isDirectory: false }];
        }
        return [];
      },
    };

    const result = await performUpgrade(
      ".claude/rules",
      ".claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      false,
      mockFs
    );

    assertEquals(result.upgraded.length, 1);
    assertEquals(result.upgraded[0], "tdd-workflow.md");
    assertEquals(writtenFiles.get(".claude/rules/tdd-workflow.md"), remoteContent);
  });

  it("reports summary of upgraded, skipped, and error files", async () => {
    // No files installed — result should have empty arrays
    globalThis.fetch = async () => {
      return new Response("", { status: 200 });
    };

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => false,
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async () => [],
    };

    const result = await performUpgrade(
      ".claude/rules",
      ".claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      false,
      mockFs
    );

    assertEquals(Array.isArray(result.upgraded), true);
    assertEquals(Array.isArray(result.skipped), true);
    assertEquals(Array.isArray(result.errors), true);
  });

  it("supports dry-run mode for upgrade preview", async () => {
    const localContent = `---
version: 0.1.0
updated: 2026-01-30
---
# Content`;

    const remoteContent = `---
version: 0.2.0
updated: 2026-01-31
---
# Updated content`;

    globalThis.fetch = async () => {
      return new Response(JSON.stringify({ content: btoa(remoteContent) }), { status: 200 });
    };

    const writtenFiles: string[] = [];

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules" || path === ".claude/.metadata.json",
      writeFile: async (path: string) => {
        writtenFiles.push(path);
      },
      readFile: async (path: string) => {
        if (path === ".claude/.metadata.json") {
          return JSON.stringify({ files: { "tdd-workflow.md": { source: "core/tdd-workflow.md" } } });
        }
        if (path === ".claude/rules/tdd-workflow.md") {
          return localContent;
        }
        return "";
      },
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [{ name: "tdd-workflow.md", isFile: true, isDirectory: false }];
        }
        return [];
      },
    };

    const result = await performUpgrade(
      ".claude/rules",
      ".claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      true, // dry-run
      mockFs
    );

    // Dry-run: file reported as upgraded but nothing written
    assertEquals(writtenFiles.length, 0);
    assertEquals(result.upgraded.length, 1);
  });
});

describe("Phase 4: Upgrade both scopes", () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("upgrades files in both global and project directories independently", async () => {
    const localContent = `---
version: 0.1.0
updated: 2026-01-30
---
# Content`;

    const remoteContent = `---
version: 0.2.0
updated: 2026-01-31
---
# Updated content`;

    globalThis.fetch = async () => {
      return new Response(JSON.stringify({ content: btoa(remoteContent) }), { status: 200 });
    };

    const writtenFiles = new Map<string, string>();

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => {
        return path.includes(".claude/rules") || path.includes(".metadata.json");
      },
      writeFile: async (path: string, content: string) => {
        writtenFiles.set(path, content);
      },
      readFile: async (path: string) => {
        if (path.includes(".metadata.json")) {
          return JSON.stringify({ files: { "tdd-workflow.md": { source: "core/tdd-workflow.md" } } });
        }
        if (path.includes("tdd-workflow.md")) {
          return localContent;
        }
        return "";
      },
      readDir: async (path: string) => {
        if (path.includes("/rules")) {
          return [{ name: "tdd-workflow.md", isFile: true, isDirectory: false }];
        }
        return [];
      },
    };

    // Upgrade global scope
    const globalResult = await performUpgrade(
      "/home/user/.claude/rules",
      "/home/user/.claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      false,
      mockFs
    );

    // Upgrade project scope
    const projectResult = await performUpgrade(
      "/project/.claude/rules",
      "/project/.claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      false,
      mockFs
    );

    // Both should upgrade independently
    assertEquals(globalResult.upgraded.length, 1);
    assertEquals(projectResult.upgraded.length, 1);
    assertEquals(writtenFiles.has("/home/user/.claude/rules/tdd-workflow.md"), true);
    assertEquals(writtenFiles.has("/project/.claude/rules/tdd-workflow.md"), true);
  });

  it("skips global directory if it doesn't exist", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => {
        // Global directory doesn't exist
        if (path.includes("/home/user/.claude")) {
          return false;
        }
        return true;
      },
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async () => [],
    };

    const result = await performUpgrade(
      "/home/user/.claude/rules",
      "/home/user/.claude",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      false,
      mockFs
    );

    // Should handle gracefully with no upgrades
    assertEquals(result.upgraded.length, 0);
  });

  it("returns separate results for global and project upgrades", async () => {
    // This test will verify that upgradeAll function (to be implemented)
    // returns separate results for each scope
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => true,
      writeFile: async () => {},
      readFile: async () => JSON.stringify({ files: {} }),
      readDir: async () => [],
    };

    // Import the new function that doesn't exist yet
    const { upgradeAll } = await import("./upgrade.ts");

    const results = await upgradeAll(
      "/project",
      "/home/user",
      "https://api.github.com/repos/test/repo",
      "main",
      false,
      false,
      mockFs
    );

    // Should return results for both scopes
    assert(results.global !== undefined);
    assert(results.project !== undefined);
    assertEquals(Array.isArray(results.global.upgraded), true);
    assertEquals(Array.isArray(results.project.upgraded), true);
  });
});