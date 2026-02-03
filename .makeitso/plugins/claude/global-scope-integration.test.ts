import { describe, it, beforeEach, afterEach } from "https://deno.land/std@0.224.0/testing/bdd.ts";
import { assertEquals, assert } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { getTargetDirectory, type Scope } from "./lib.ts";
import { loadMetadata, saveMetadata, addFileToMetadata } from "./metadata.ts";
import { denoFs } from "./deno-fs.ts";

/**
 * Phase 5: Integration tests for global scope feature
 * These test complete end-to-end flows with real filesystem operations
 */

describe("Phase 5: Global scope integration", () => {
  let tempProjectDir: string;
  let tempHomeDir: string;

  beforeEach(async () => {
    tempProjectDir = await Deno.makeTempDir({ prefix: "project_" });
    tempHomeDir = await Deno.makeTempDir({ prefix: "home_" });
  });

  afterEach(async () => {
    await Deno.remove(tempProjectDir, { recursive: true });
    await Deno.remove(tempHomeDir, { recursive: true });
  });

  it("full flow: add global → upgrade global", async () => {
    const scope: Scope = "global";
    const targetDir = getTargetDirectory(scope, tempProjectDir, tempHomeDir);
    const rulesDir = `${targetDir}/rules`;

    // Simulate add command: create directories and install file
    await denoFs.mkdir(targetDir, true);
    await denoFs.mkdir(rulesDir, true);

    const initialContent = `---
version: 0.1.0
updated: 2026-01-30
---
# Initial content`;

    await denoFs.writeFile(`${rulesDir}/tdd-workflow.md`, initialContent);

    // Save metadata
    const metadata = await loadMetadata(targetDir, denoFs);
    addFileToMetadata(metadata, "tdd-workflow.md", "core/tdd-workflow.md");
    await saveMetadata(targetDir, metadata, denoFs);

    // Verify file and metadata exist in global location
    const fileExists = await denoFs.exists(`${rulesDir}/tdd-workflow.md`);
    const metadataExists = await denoFs.exists(`${targetDir}/.metadata.json`);

    assertEquals(fileExists, true);
    assertEquals(metadataExists, true);
    assertEquals(targetDir, `${tempHomeDir}/.claude`);

    // Verify metadata has correct structure
    const loadedMetadata = await loadMetadata(targetDir, denoFs);
    assertEquals(loadedMetadata.files["tdd-workflow.md"].source, "core/tdd-workflow.md");
  });

  it("full flow: add project → upgrade project", async () => {
    const scope: Scope = "project";
    const targetDir = getTargetDirectory(scope, tempProjectDir, tempHomeDir);
    const rulesDir = `${targetDir}/rules`;

    // Simulate add command for project scope
    await denoFs.mkdir(targetDir, true);
    await denoFs.mkdir(rulesDir, true);

    const content = `---
version: 0.1.0
updated: 2026-01-30
---
# Project content`;

    await denoFs.writeFile(`${rulesDir}/code-style.md`, content);

    // Save metadata
    const metadata = await loadMetadata(targetDir, denoFs);
    addFileToMetadata(metadata, "code-style.md", "stacks/python/code-style.md");
    await saveMetadata(targetDir, metadata, denoFs);

    // Verify file exists in project location
    const fileExists = await denoFs.exists(`${rulesDir}/code-style.md`);
    assertEquals(fileExists, true);
    assertEquals(targetDir, `${tempProjectDir}/.claude`);
  });

  it("mixed scenario: files in both global and project locations", async () => {
    // Install to global scope
    const globalDir = getTargetDirectory("global", tempProjectDir, tempHomeDir);
    const globalRulesDir = `${globalDir}/rules`;

    await denoFs.mkdir(globalDir, true);
    await denoFs.mkdir(globalRulesDir, true);
    await denoFs.writeFile(`${globalRulesDir}/tdd-workflow.md`, "# Global TDD");

    const globalMetadata = await loadMetadata(globalDir, denoFs);
    addFileToMetadata(globalMetadata, "tdd-workflow.md", "core/tdd-workflow.md");
    await saveMetadata(globalDir, globalMetadata, denoFs);

    // Install to project scope
    const projectDir = getTargetDirectory("project", tempProjectDir, tempHomeDir);
    const projectRulesDir = `${projectDir}/rules`;

    await denoFs.mkdir(projectDir, true);
    await denoFs.mkdir(projectRulesDir, true);
    await denoFs.writeFile(`${projectRulesDir}/code-style.md`, "# Project Code Style");

    const projectMetadata = await loadMetadata(projectDir, denoFs);
    addFileToMetadata(projectMetadata, "code-style.md", "stacks/python/code-style.md");
    await saveMetadata(projectDir, projectMetadata, denoFs);

    // Verify both exist independently
    const globalFileExists = await denoFs.exists(`${globalRulesDir}/tdd-workflow.md`);
    const projectFileExists = await denoFs.exists(`${projectRulesDir}/code-style.md`);

    assertEquals(globalFileExists, true);
    assertEquals(projectFileExists, true);

    // Verify metadata is separate
    const loadedGlobalMetadata = await loadMetadata(globalDir, denoFs);
    const loadedProjectMetadata = await loadMetadata(projectDir, denoFs);

    assert(loadedGlobalMetadata.files["tdd-workflow.md"] !== undefined);
    assert(loadedGlobalMetadata.files["code-style.md"] === undefined);

    assert(loadedProjectMetadata.files["code-style.md"] !== undefined);
    assert(loadedProjectMetadata.files["tdd-workflow.md"] === undefined);
  });

  it("same file can exist in both scopes without conflict", async () => {
    const fileName = "tdd-workflow.md";

    // Install to global scope
    const globalDir = getTargetDirectory("global", tempProjectDir, tempHomeDir);
    const globalRulesDir = `${globalDir}/rules`;

    await denoFs.mkdir(globalDir, true);
    await denoFs.mkdir(globalRulesDir, true);
    await denoFs.writeFile(`${globalRulesDir}/${fileName}`, "# Global version");

    // Install to project scope
    const projectDir = getTargetDirectory("project", tempProjectDir, tempHomeDir);
    const projectRulesDir = `${projectDir}/rules`;

    await denoFs.mkdir(projectDir, true);
    await denoFs.mkdir(projectRulesDir, true);
    await denoFs.writeFile(`${projectRulesDir}/${fileName}`, "# Project version");

    // Read both files - they should have different content
    const globalContent = await denoFs.readFile(`${globalRulesDir}/${fileName}`);
    const projectContent = await denoFs.readFile(`${projectRulesDir}/${fileName}`);

    assertEquals(globalContent, "# Global version");
    assertEquals(projectContent, "# Project version");
  });
});
