import { describe, it } from "https://deno.land/std@0.224.0/testing/bdd.ts";
import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { resolveHomePath, getTargetDirectory } from "./lib.ts";

describe("resolveHomePath", () => {
  it("expands ~ to home directory", () => {
    const result = resolveHomePath("~", "/Users/testuser");
    assertEquals(result, "/Users/testuser");
  });

  it("expands ~/path to home directory with path", () => {
    const result = resolveHomePath("~/.claude/rules", "/Users/testuser");
    assertEquals(result, "/Users/testuser/.claude/rules");
  });

  it("returns path unchanged when no ~ prefix", () => {
    const result = resolveHomePath("/absolute/path", "/Users/testuser");
    assertEquals(result, "/absolute/path");
  });

  it("handles ~/ at start of path", () => {
    const result = resolveHomePath("~/", "/Users/testuser");
    assertEquals(result, "/Users/testuser/");
  });
});

describe("getTargetDirectory", () => {
  it("returns project .claude directory for project scope", () => {
    const result = getTargetDirectory("project", "/Users/testuser/myproject", "/Users/testuser");
    assertEquals(result, "/Users/testuser/myproject/.claude");
  });

  it("returns global ~/.claude directory for global scope", () => {
    const result = getTargetDirectory("global", "/Users/testuser/myproject", "/Users/testuser");
    assertEquals(result, "/Users/testuser/.claude");
  });

  it("resolves ~ in global scope path", () => {
    const result = getTargetDirectory("global", "/any/project", "/home/user");
    assertEquals(result, "/home/user/.claude");
  });
});
