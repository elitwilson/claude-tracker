import { describe, it } from "https://deno.land/std@0.224.0/testing/bdd.ts";
import { assertEquals, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import {
  loadMetadata,
  saveMetadata,
  addFileToMetadata,
  getFileSource,
  getFileChecksum,
  type Metadata,
  type FileSystem,
} from "./metadata.ts";

describe("loadMetadata", () => {
  it("returns empty metadata when file doesn't exist", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => false,
      writeFile: async () => {},
      readFile: async () => "",
      readDir: async () => [],
    };

    const metadata = await loadMetadata("/project/.claude", mockFs);

    assertEquals(metadata, { files: {} });
  });

  it("parses existing metadata file", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => true,
      writeFile: async () => {},
      readFile: async () => JSON.stringify({
        files: {
          "tdd-workflow.md": { source: "core/tdd-workflow.md" },
          "code-style.md": { source: "stacks/python/code-style.md" },
        },
      }),
      readDir: async () => [],
    };

    const metadata = await loadMetadata("/project/.claude", mockFs);

    assertEquals(metadata.files["tdd-workflow.md"].source, "core/tdd-workflow.md");
    assertEquals(metadata.files["code-style.md"].source, "stacks/python/code-style.md");
  });

  it("handles corrupted metadata gracefully", async () => {
    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => true,
      writeFile: async () => {},
      readFile: async () => "invalid json{",
      readDir: async () => [],
    };

    const metadata = await loadMetadata("/project/.claude", mockFs);

    assertEquals(metadata, { files: {} });
  });
});

describe("saveMetadata", () => {
  it("writes metadata as formatted JSON", async () => {
    let writtenPath = "";
    let writtenContent = "";

    const mockFs: FileSystem = {
      mkdir: async () => {},
      exists: async () => true,
      writeFile: async (path: string, content: string) => {
        writtenPath = path;
        writtenContent = content;
      },
      readFile: async () => "",
      readDir: async () => [],
    };

    const metadata: Metadata = {
      files: {
        "tdd-workflow.md": { source: "core/tdd-workflow.md" },
      },
    };

    await saveMetadata("/project/.claude", metadata, mockFs);

    assertEquals(writtenPath, "/project/.claude/.metadata.json");
    const parsed = JSON.parse(writtenContent);
    assertEquals(parsed.files["tdd-workflow.md"].source, "core/tdd-workflow.md");
  });
});

describe("addFileToMetadata", () => {
  it("adds a new file entry to metadata", () => {
    const metadata: Metadata = { files: {} };

    addFileToMetadata(metadata, "tdd-workflow.md", "core/tdd-workflow.md");

    assertEquals(metadata.files["tdd-workflow.md"].source, "core/tdd-workflow.md");
  });

  it("overwrites existing file entry", () => {
    const metadata: Metadata = {
      files: {
        "tdd-workflow.md": { source: "core/tdd-workflow.md" },
      },
    };

    addFileToMetadata(metadata, "tdd-workflow.md", "stacks/python/tdd-workflow.md");

    assertEquals(metadata.files["tdd-workflow.md"].source, "stacks/python/tdd-workflow.md");
  });

  it("stores checksum when provided", () => {
    const metadata: Metadata = { files: {} };

    addFileToMetadata(metadata, "tdd-workflow.md", "core/tdd-workflow.md", "abc123");

    assertEquals(metadata.files["tdd-workflow.md"].source, "core/tdd-workflow.md");
    assertEquals(metadata.files["tdd-workflow.md"].checksum, "abc123");
  });

  it("stores entry without checksum when not provided", () => {
    const metadata: Metadata = { files: {} };

    addFileToMetadata(metadata, "tdd-workflow.md", "core/tdd-workflow.md");

    assertEquals(metadata.files["tdd-workflow.md"].source, "core/tdd-workflow.md");
    assertEquals(metadata.files["tdd-workflow.md"].checksum, undefined);
  });
});

describe("getFileChecksum", () => {
  it("returns checksum for tracked file that has one", () => {
    const metadata: Metadata = {
      files: {
        "tdd-workflow.md": { source: "core/tdd-workflow.md", checksum: "abc123" },
      },
    };

    assertEquals(getFileChecksum(metadata, "tdd-workflow.md"), "abc123");
  });

  it("returns null for tracked file without checksum", () => {
    const metadata: Metadata = {
      files: {
        "tdd-workflow.md": { source: "core/tdd-workflow.md" },
      },
    };

    assertEquals(getFileChecksum(metadata, "tdd-workflow.md"), null);
  });

  it("returns null for untracked file", () => {
    const metadata: Metadata = { files: {} };

    assertEquals(getFileChecksum(metadata, "unknown.md"), null);
  });
});

describe("getFileSource", () => {
  it("returns source path for tracked file", () => {
    const metadata: Metadata = {
      files: {
        "tdd-workflow.md": { source: "core/tdd-workflow.md" },
      },
    };

    const source = getFileSource(metadata, "tdd-workflow.md");

    assertEquals(source, "core/tdd-workflow.md");
  });

  it("returns null for untracked file", () => {
    const metadata: Metadata = { files: {} };

    const source = getFileSource(metadata, "unknown.md");

    assertEquals(source, null);
  });
});
