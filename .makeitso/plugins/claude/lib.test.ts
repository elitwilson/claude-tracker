import { describe, it, beforeEach, afterEach } from "https://deno.land/std@0.224.0/testing/bdd.ts";
import { assertEquals, assertRejects } from "https://deno.land/std@0.224.0/assert/mod.ts";
import {
  fetchWorkflowFile,
  ensureDirectory,
  writeWorkflowFile,
  parseFrontmatter,
  calculateChecksum,
  discoverInstalledWorkflows,
  type FileSystem
} from "./lib.ts";

describe("fetchWorkflowFile", () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("constructs correct URL and returns file content", async () => {
    const mockContent = "---\nversion: 0.1.0\n---\n# Test File";

    globalThis.fetch = async (url: string | URL | Request) => {
      const urlStr = url.toString();
      assertEquals(urlStr, "https://api.github.com/repos/test/repo/contents/path/to/file.md?ref=main");

      return new Response(JSON.stringify({ content: btoa(mockContent) }), { status: 200 });
    };

    const result = await fetchWorkflowFile(
      "https://api.github.com/repos/test/repo",
      "main",
      "path/to/file.md"
    );

    assertEquals(result, mockContent);
  });

  it("throws error with file path when HTTP request fails", async () => {
    globalThis.fetch = async () => {
      return new Response("Not Found", { status: 404, statusText: "Not Found" });
    };

    await assertRejects(
      async () => {
        await fetchWorkflowFile(
          "https://api.github.com/repos/test/repo",
          "main",
          "missing.md"
        );
      },
      Error,
      "Failed to fetch missing.md: HTTP 404 Not Found"
    );
  });

  it("throws error when network fails", async () => {
    globalThis.fetch = async () => {
      throw new TypeError("Network error");
    };

    await assertRejects(
      async () => {
        await fetchWorkflowFile(
          "https://api.github.com/repos/test/repo",
          "main",
          "file.md"
        );
      },
      Error,
      "Failed to fetch file.md"
    );
  });
});

describe("ensureDirectory", () => {
  it("creates directory when it doesn't exist", async () => {
    const logs: string[] = [];
    const createdDirs: string[] = [];

    const mockFs: FileSystem = {
      mkdir: async (path: string, recursive: boolean) => {
        createdDirs.push(path);
      },
      exists: async (path: string) => false,
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async () => [],
    };

    const log = (msg: string) => logs.push(msg);

    await ensureDirectory(".claude/rules", false, mockFs, log);

    assertEquals(createdDirs, [".claude/rules"]);
    assertEquals(logs, ["Created directory: .claude/rules"]);
  });

  it("does not create directory when it already exists", async () => {
    const logs: string[] = [];
    const createdDirs: string[] = [];

    const mockFs: FileSystem = {
      mkdir: async (path: string) => {
        createdDirs.push(path);
      },
      exists: async (path: string) => true,
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async () => [],
    };

    const log = (msg: string) => logs.push(msg);

    await ensureDirectory(".claude", false, mockFs, log);

    assertEquals(createdDirs, []);
    assertEquals(logs, []);
  });

  it("logs intended action in dry-run mode without creating directory", async () => {
    const logs: string[] = [];
    const createdDirs: string[] = [];

    const mockFs: FileSystem = {
      mkdir: async (path: string) => {
        createdDirs.push(path);
      },
      exists: async (path: string) => false,
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async () => [],
    };

    const log = (msg: string) => logs.push(msg);

    await ensureDirectory(".claude/rules", true, mockFs, log);

    assertEquals(createdDirs, []);
    assertEquals(logs, ["Would create directory: .claude/rules"]);
  });

  it("does not check existence in dry-run mode", async () => {
    const logs: string[] = [];
    let existsCalled = false;

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => {
        existsCalled = true;
        return false;
      },
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async () => [],
    };

    const log = (msg: string) => logs.push(msg);

    await ensureDirectory(".claude", true, mockFs, log);

    assertEquals(existsCalled, false);
    assertEquals(logs, ["Would create directory: .claude"]);
  });
});

describe("writeWorkflowFile", () => {
  it("writes file content to specified path", async () => {
    const logs: string[] = [];
    const writtenFiles = new Map<string, string>();

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => true,
      writeFile: async (path: string, content: string) => {
        writtenFiles.set(path, content);
      },
      readFile: async () => "",
      readDir: async () => [],
    };

    const log = (msg: string) => logs.push(msg);
    const content = "---\nversion: 0.1.0\n---\n# TDD Workflow";

    await writeWorkflowFile(".claude/rules/tdd-workflow.md", content, false, mockFs, log);

    assertEquals(writtenFiles.get(".claude/rules/tdd-workflow.md"), content);
    assertEquals(logs, ["Wrote file: .claude/rules/tdd-workflow.md"]);
  });

  it("logs intended action in dry-run mode without writing file", async () => {
    const logs: string[] = [];
    const writtenFiles = new Map<string, string>();

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => true,
      writeFile: async (path: string, content: string) => {
        writtenFiles.set(path, content);
      },
      readFile: async () => "",
      readDir: async () => [],
    };

    const log = (msg: string) => logs.push(msg);
    const content = "---\nversion: 0.1.0\n---\n# TDD Workflow";

    await writeWorkflowFile(".claude/rules/tdd-workflow.md", content, true, mockFs, log);

    assertEquals(writtenFiles.size, 0);
    assertEquals(logs, ["Would write file: .claude/rules/tdd-workflow.md"]);
  });

  it("preserves file content exactly as provided", async () => {
    const writtenFiles = new Map<string, string>();

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => true,
      writeFile: async (path: string, content: string) => {
        writtenFiles.set(path, content);
      },
      readFile: async () => "",
      readDir: async () => [],
    };

    const content = "Line 1\nLine 2\n\nLine 4 with    spaces\n\tTabbed line";

    await writeWorkflowFile("test.md", content, false, mockFs, () => {});

    assertEquals(writtenFiles.get("test.md"), content);
  });
});

describe("parseFrontmatter", () => {
  it("extracts version and updated date from valid frontmatter", () => {
    const content = `---
version: 0.1.0
updated: 2026-01-30
---

# TDD Workflow

Content here...`;

    const result = parseFrontmatter(content);

    assertEquals(result, {
      version: "0.1.0",
      updated: "2026-01-30",
    });
  });

  it("returns null for files without frontmatter", () => {
    const content = `# TDD Workflow

No frontmatter here.`;

    const result = parseFrontmatter(content);

    assertEquals(result, null);
  });

  it("returns null for files with incomplete frontmatter", () => {
    const content = `---
updated: 2026-01-30
---

# Missing version field`;

    const result = parseFrontmatter(content);

    assertEquals(result, null);
  });

  it("handles frontmatter with extra fields", () => {
    const content = `---
version: 0.2.0
updated: 2026-01-30
author: Test Author
tags: [workflow, tdd]
---

# Content`;

    const result = parseFrontmatter(content);

    assertEquals(result, {
      version: "0.2.0",
      updated: "2026-01-30",
    });
  });

  it("handles CRLF line endings", () => {
    const content = "---\r\nversion: 0.1.0\r\nupdated: 2026-01-30\r\n---\r\n\r\n# Content";

    const result = parseFrontmatter(content);

    assertEquals(result, {
      version: "0.1.0",
      updated: "2026-01-30",
    });
  });
});

describe("calculateChecksum", () => {
  it("calculates SHA-256 hash of file content", async () => {
    const content = "Hello, World!";

    const checksum = await calculateChecksum(content);

    // SHA-256 of "Hello, World!"
    assertEquals(checksum, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
  });

  it("produces different checksums for different content", async () => {
    const content1 = "Content A";
    const content2 = "Content B";

    const checksum1 = await calculateChecksum(content1);
    const checksum2 = await calculateChecksum(content2);

    assertEquals(checksum1 !== checksum2, true);
  });

  it("produces same checksum for identical content", async () => {
    const content = "Identical content";

    const checksum1 = await calculateChecksum(content);
    const checksum2 = await calculateChecksum(content);

    assertEquals(checksum1, checksum2);
  });

  it("includes frontmatter in checksum calculation", async () => {
    const content1 = `---
version: 0.1.0
---
Content`;

    const content2 = `---
version: 0.2.0
---
Content`;

    const checksum1 = await calculateChecksum(content1);
    const checksum2 = await calculateChecksum(content2);

    assertEquals(checksum1 !== checksum2, true);
  });

  it("is sensitive to whitespace changes", async () => {
    const content1 = "Line 1\nLine 2";
    const content2 = "Line 1\n\nLine 2";

    const checksum1 = await calculateChecksum(content1);
    const checksum2 = await calculateChecksum(content2);

    assertEquals(checksum1 !== checksum2, true);
  });
});

describe("discoverInstalledWorkflows", () => {
  it("discovers all .md files in .claude/rules directory", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules",
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [
            { name: "tdd-workflow.md", isFile: true, isDirectory: false },
            { name: "claude-code-usage.md", isFile: true, isDirectory: false },
            { name: "doc-patterns.md", isFile: true, isDirectory: false },
          ];
        }
        return [];
      },
    };

    const files = await discoverInstalledWorkflows(".claude/rules", mockFs);

    assertEquals(files, [
      ".claude/rules/tdd-workflow.md",
      ".claude/rules/claude-code-usage.md",
      ".claude/rules/doc-patterns.md",
    ]);
  });

  it("returns empty array when directory doesn't exist", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => false,
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async () => [],
    };

    const files = await discoverInstalledWorkflows(".claude/rules", mockFs);

    assertEquals(files, []);
  });

  it("ignores non-.md files", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules",
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [
            { name: "tdd-workflow.md", isFile: true, isDirectory: false },
            { name: "notes.txt", isFile: true, isDirectory: false },
            { name: "config.json", isFile: true, isDirectory: false },
            { name: "README", isFile: true, isDirectory: false },
          ];
        }
        return [];
      },
    };

    const files = await discoverInstalledWorkflows(".claude/rules", mockFs);

    assertEquals(files, [".claude/rules/tdd-workflow.md"]);
  });

  it("returns files with paths relative to base directory", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === "/project/.claude/rules",
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async (path: string) => {
        if (path === "/project/.claude/rules") {
          return [
            { name: "workflow.md", isFile: true, isDirectory: false },
          ];
        }
        return [];
      },
    };

    const files = await discoverInstalledWorkflows("/project/.claude/rules", mockFs);

    assertEquals(files, ["/project/.claude/rules/workflow.md"]);
  });

  it("ignores directories", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async (path: string) => path === ".claude/rules",
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async (path: string) => {
        if (path === ".claude/rules") {
          return [
            { name: "tdd-workflow.md", isFile: true, isDirectory: false },
            { name: "subfolder", isFile: false, isDirectory: true },
          ];
        }
        return [];
      },
    };

    const files = await discoverInstalledWorkflows(".claude/rules", mockFs);

    assertEquals(files, [".claude/rules/tdd-workflow.md"]);
  });
});
