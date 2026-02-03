import { describe, it, beforeEach, afterEach } from "https://deno.land/std@0.224.0/testing/bdd.ts";
import { assertEquals, assertThrows } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { getHomeDir, denoFs } from "./deno-fs.ts";
import { getTargetDirectory, type Scope, type FileSystem } from "./lib.ts";
import { checkDuplicateFile } from "./lib.ts";

describe("getHomeDir", () => {
  it("returns HOME environment variable value", () => {
    const homeDir = getHomeDir();
    const expected = Deno.env.get("HOME");

    assertEquals(homeDir, expected);
  });

  it("throws error when HOME is not set", () => {
    // Save original HOME value
    const originalHome = Deno.env.get("HOME");

    // Temporarily unset HOME
    Deno.env.delete("HOME");

    try {
      assertThrows(
        () => getHomeDir(),
        Error,
        "HOME environment variable is not set"
      );
    } finally {
      // Restore HOME
      if (originalHome) {
        Deno.env.set("HOME", originalHome);
      }
    }
  });
});

describe("Scope selection integration", () => {
  it("uses project directory when project scope selected", () => {
    const scope: Scope = "project";
    const projectRoot = "/Users/testuser/myproject";
    const homeDir = "/Users/testuser";

    const result = getTargetDirectory(scope, projectRoot, homeDir);

    assertEquals(result, "/Users/testuser/myproject/.claude");
  });

  it("uses global directory when global scope selected", () => {
    const scope: Scope = "global";
    const projectRoot = "/Users/testuser/myproject";
    const homeDir = "/Users/testuser";

    const result = getTargetDirectory(scope, projectRoot, homeDir);

    assertEquals(result, "/Users/testuser/.claude");
  });

  it("global scope ignores project root", () => {
    const scope: Scope = "global";
    const projectRoot = "/completely/different/path";
    const homeDir = "/Users/testuser";

    const result = getTargetDirectory(scope, projectRoot, homeDir);

    // Should use homeDir, not projectRoot
    assertEquals(result, "/Users/testuser/.claude");
  });
});

describe("Phase 3: Add command with scope", () => {
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await Deno.makeTempDir();
  });

  afterEach(async () => {
    await Deno.remove(tempDir, { recursive: true });
  });

  it("detects duplicate file in same scope", async () => {
    // Create a file in the target directory
    const projectRoot = tempDir;
    const scope: Scope = "project";
    const homeDir = "/Users/testuser";

    const targetDir = getTargetDirectory(scope, projectRoot, homeDir);
    const rulesDir = `${targetDir}/rules`;

    await Deno.mkdir(rulesDir, { recursive: true });
    await Deno.writeTextFile(`${rulesDir}/tdd-workflow.md`, "existing content");

    // Check if duplicate exists
    const isDuplicate = await checkDuplicateFile("tdd-workflow.md", rulesDir, denoFs);

    assertEquals(isDuplicate, true);
  });

  it("allows installing file when no duplicate exists", async () => {
    const projectRoot = tempDir;
    const scope: Scope = "project";
    const homeDir = "/Users/testuser";

    const targetDir = getTargetDirectory(scope, projectRoot, homeDir);
    const rulesDir = `${targetDir}/rules`;

    await Deno.mkdir(rulesDir, { recursive: true });

    // Check if duplicate exists (should be false)
    const isDuplicate = await checkDuplicateFile("tdd-workflow.md", rulesDir, denoFs);

    assertEquals(isDuplicate, false);
  });

  it("allows same file in different scopes (global and project)", async () => {
    // This test verifies that duplicates ACROSS scopes are allowed
    const projectRoot = tempDir;
    const homeDir = `${tempDir}/home`;

    // Create file in project scope
    const projectDir = getTargetDirectory("project", projectRoot, homeDir);
    const projectRulesDir = `${projectDir}/rules`;
    await Deno.mkdir(projectRulesDir, { recursive: true });
    await Deno.writeTextFile(`${projectRulesDir}/tdd-workflow.md`, "project content");

    // Create file in global scope
    const globalDir = getTargetDirectory("global", projectRoot, homeDir);
    const globalRulesDir = `${globalDir}/rules`;
    await Deno.mkdir(globalRulesDir, { recursive: true });
    await Deno.writeTextFile(`${globalRulesDir}/tdd-workflow.md`, "global content");

    // Both should exist independently
    const projectExists = await checkDuplicateFile("tdd-workflow.md", projectRulesDir, denoFs);
    const globalExists = await checkDuplicateFile("tdd-workflow.md", globalRulesDir, denoFs);

    assertEquals(projectExists, true);
    assertEquals(globalExists, true);
  });
});
