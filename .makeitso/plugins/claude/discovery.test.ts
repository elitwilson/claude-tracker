import { describe, it, beforeEach, afterEach } from "https://deno.land/std@0.224.0/testing/bdd.ts";
import { assertEquals, assertRejects } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { discoverCoreWorkflows, discoverStacks, discoverStackWorkflows } from "./discovery.ts";

describe("discoverCoreWorkflows", () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("fetches core workflow files from GitHub API", async () => {
    globalThis.fetch = async (url: string | URL | Request) => {
      assertEquals(
        url.toString(),
        "https://api.github.com/repos/test/repo/contents/core?ref=main"
      );

      const mockResponse = [
        { name: "tdd-workflow.md", path: "core/tdd-workflow.md", type: "file" },
        { name: "doc-patterns.md", path: "core/doc-patterns.md", type: "file" },
        { name: "README.md", path: "core/README.md", type: "file" },
        { name: "setup.py", path: "core/setup.py", type: "file" },
      ];

      return new Response(JSON.stringify(mockResponse), { status: 200 });
    };

    const result = await discoverCoreWorkflows("test", "repo", "main");

    assertEquals(result, [
      { name: "tdd-workflow.md", path: "core/tdd-workflow.md" },
      { name: "doc-patterns.md", path: "core/doc-patterns.md" },
    ]);
  });

  it("filters out README.md files", async () => {
    globalThis.fetch = async () => {
      const mockResponse = [
        { name: "README.md", path: "core/README.md", type: "file" },
        { name: "workflow.md", path: "core/workflow.md", type: "file" },
      ];
      return new Response(JSON.stringify(mockResponse), { status: 200 });
    };

    const result = await discoverCoreWorkflows("test", "repo", "main");

    assertEquals(result, [
      { name: "workflow.md", path: "core/workflow.md" },
    ]);
  });

  it("only includes .md files", async () => {
    globalThis.fetch = async () => {
      const mockResponse = [
        { name: "workflow.md", path: "core/workflow.md", type: "file" },
        { name: "script.py", path: "core/script.py", type: "file" },
        { name: "config.json", path: "core/config.json", type: "file" },
      ];
      return new Response(JSON.stringify(mockResponse), { status: 200 });
    };

    const result = await discoverCoreWorkflows("test", "repo", "main");

    assertEquals(result, [
      { name: "workflow.md", path: "core/workflow.md" },
    ]);
  });

  it("throws error when API request fails", async () => {
    globalThis.fetch = async () => {
      return new Response("Not Found", { status: 404, statusText: "Not Found" });
    };

    await assertRejects(
      async () => {
        await discoverCoreWorkflows("test", "repo", "main");
      },
      Error,
      "Failed to fetch"
    );
  });
});

describe("discoverStacks", () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("fetches stack subdirectories from GitHub API", async () => {
    globalThis.fetch = async (url: string | URL | Request) => {
      assertEquals(
        url.toString(),
        "https://api.github.com/repos/test/repo/contents/stacks?ref=main"
      );

      const mockResponse = [
        { name: "python", path: "stacks/python", type: "dir" },
        { name: "typescript", path: "stacks/typescript", type: "dir" },
        { name: "go", path: "stacks/go", type: "dir" },
        { name: "README.md", path: "stacks/README.md", type: "file" },
      ];

      return new Response(JSON.stringify(mockResponse), { status: 200 });
    };

    const result = await discoverStacks("test", "repo", "main");

    assertEquals(result, [
      { name: "python", displayName: "python" },
      { name: "typescript", displayName: "typescript" },
      { name: "go", displayName: "go" },
    ]);
  });

  it("only includes directories, not files", async () => {
    globalThis.fetch = async () => {
      const mockResponse = [
        { name: "python", path: "stacks/python", type: "dir" },
        { name: "setup.py", path: "stacks/setup.py", type: "file" },
        { name: "rust", path: "stacks/rust", type: "dir" },
      ];
      return new Response(JSON.stringify(mockResponse), { status: 200 });
    };

    const result = await discoverStacks("test", "repo", "main");

    assertEquals(result, [
      { name: "python", displayName: "python" },
      { name: "rust", displayName: "rust" },
    ]);
  });

  it("throws error when API request fails", async () => {
    globalThis.fetch = async () => {
      return new Response("Not Found", { status: 404, statusText: "Not Found" });
    };

    await assertRejects(
      async () => {
        await discoverStacks("test", "repo", "main");
      },
      Error,
      "Failed to fetch"
    );
  });
});

describe("discoverStackWorkflows", () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("fetches workflow files for a specific stack", async () => {
    globalThis.fetch = async (url: string | URL | Request) => {
      assertEquals(
        url.toString(),
        "https://api.github.com/repos/test/repo/contents/stacks/python?ref=main"
      );

      const mockResponse = [
        { name: "code-style.md", path: "stacks/python/code-style.md", type: "file" },
        { name: "testing.md", path: "stacks/python/testing.md", type: "file" },
        { name: "README.md", path: "stacks/python/README.md", type: "file" },
        { name: "setup.py", path: "stacks/python/setup.py", type: "file" },
      ];

      return new Response(JSON.stringify(mockResponse), { status: 200 });
    };

    const result = await discoverStackWorkflows("test", "repo", "main", "python");

    assertEquals(result, [
      { name: "code-style.md", path: "stacks/python/code-style.md" },
      { name: "testing.md", path: "stacks/python/testing.md" },
    ]);
  });

  it("filters out README.md files", async () => {
    globalThis.fetch = async () => {
      const mockResponse = [
        { name: "README.md", path: "stacks/go/README.md", type: "file" },
        { name: "testing.md", path: "stacks/go/testing.md", type: "file" },
      ];
      return new Response(JSON.stringify(mockResponse), { status: 200 });
    };

    const result = await discoverStackWorkflows("test", "repo", "main", "go");

    assertEquals(result, [
      { name: "testing.md", path: "stacks/go/testing.md" },
    ]);
  });

  it("only includes .md files", async () => {
    globalThis.fetch = async () => {
      const mockResponse = [
        { name: "testing.md", path: "stacks/rust/testing.md", type: "file" },
        { name: "setup.py", path: "stacks/rust/setup.py", type: "file" },
        { name: "config.toml", path: "stacks/rust/config.toml", type: "file" },
      ];
      return new Response(JSON.stringify(mockResponse), { status: 200 });
    };

    const result = await discoverStackWorkflows("test", "repo", "main", "rust");

    assertEquals(result, [
      { name: "testing.md", path: "stacks/rust/testing.md" },
    ]);
  });

  it("throws error when API request fails", async () => {
    globalThis.fetch = async () => {
      return new Response("Not Found", { status: 404, statusText: "Not Found" });
    };

    await assertRejects(
      async () => {
        await discoverStackWorkflows("test", "repo", "main", "python");
      },
      Error,
      "Failed to fetch"
    );
  });
});
