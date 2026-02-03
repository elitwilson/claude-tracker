// Import shared types and utilities from Make It So
import type { PluginContext } from "../../mis-types.d.ts";
import { mis } from "../../mis-plugin-api.ts";
import { Checkbox, Select } from "https://deno.land/x/cliffy@v1.0.0-rc.4/prompt/mod.ts";
import {
  discoverCoreWorkflows,
  discoverStacks,
  discoverStackWorkflows,
} from "./discovery.ts";
import {
  buildRepoUrl,
  fetchWorkflowFile,
  ensureDirectory,
  writeWorkflowFile,
  getTargetDirectory,
  type Scope,
} from "./lib.ts";
import { denoFs, getHomeDir } from "./deno-fs.ts";
import {
  loadMetadata,
  saveMetadata,
  addFileToMetadata,
} from "./metadata.ts";

try {
  const ctx: PluginContext = await mis.loadContext();

  console.log("🪄 Add Claude Workflows\n");

  // Scope selection
  const scope = await Select.prompt<Scope>({
    message: "Install to:",
    options: [
      { name: "Project (.claude/)", value: "project" },
      { name: "Global (~/.claude/)", value: "global" },
    ],
    default: "project",
  });

  console.log(`\nInstalling to ${scope} scope\n`);
  console.log("Discovering available workflows...\n");

  // Extract repo owner and name from config
  const owner = mis.getConfig(ctx, "repo_owner", "elitwilson");
  const repo = mis.getConfig(ctx, "repo_name", "claude-workflows");
  const branch = mis.getConfig(ctx, "default_branch", "main");

  // Discover available workflows
  const coreWorkflows = await discoverCoreWorkflows(owner, repo, branch);
  const availableStacks = await discoverStacks(owner, repo, branch);

  // Select core workflows
  const selectedCoreRaw = await Checkbox.prompt({
    message: "Select core workflows to install:",
    options: coreWorkflows.map((w) => ({ name: w.name, value: w.path })),
  });
  const selectedCore = Array.isArray(selectedCoreRaw)
    ? selectedCoreRaw.map(item => typeof item === 'string' ? item : item.value)
    : [];

  console.log(`\nSelected ${selectedCore.length} core workflow(s)\n`);

  // Select language stacks
  const selectedStackNamesRaw = await Checkbox.prompt({
    message: "Select language stacks to install:",
    options: availableStacks.map((s) => ({ name: s.displayName, value: s.name })),
  });
  const selectedStackNames = Array.isArray(selectedStackNamesRaw)
    ? selectedStackNamesRaw.map(item => typeof item === 'string' ? item : item.value)
    : [];

  // Collect all stack workflow files
  const stackFiles: string[] = [];
  for (const stackName of selectedStackNames) {
    const workflows = await discoverStackWorkflows(owner, repo, branch, stackName);
    stackFiles.push(...workflows.map((w) => w.path));
  }

  console.log(`\nSelected ${selectedStackNames.length} stack(s) (${stackFiles.length} files)\n`);

  // Summary
  const allFiles = [...selectedCore, ...stackFiles];

  // Get home directory and determine target directory based on scope
  const homeDir = getHomeDir();
  const claudeDir = getTargetDirectory(scope, ctx.project_root, homeDir);
  const rulesDir = `${claudeDir}/rules`;

  console.log("📋 Summary:");
  console.log(`  Total files to install: ${allFiles.length}`);
  console.log(`  Core workflows: ${selectedCore.length}`);
  console.log(`  Stack workflows: ${stackFiles.length}`);
  console.log(`  Target directory: ${rulesDir}`);

  // Get dry-run flag from context
  const dryRun = ctx.dry_run || false;

  if (dryRun) {
    console.log("\n🔍 Dry-run mode: No files will be created\n");
  } else {
    console.log("\n📦 Installing workflow files...\n");
  }

  await ensureDirectory(claudeDir, dryRun, denoFs, console.log);
  await ensureDirectory(rulesDir, dryRun, denoFs, console.log);

  const repoUrl = buildRepoUrl(owner, repo);

  // Load existing metadata
  const metadata = await loadMetadata(claudeDir, denoFs);

  // Download and write each selected file
  let successCount = 0;
  let errorCount = 0;

  for (const filePath of allFiles) {
    try {
      const content = await fetchWorkflowFile(repoUrl, branch, filePath);
      const fileName = filePath.split("/").pop() || filePath;
      const targetPath = `${rulesDir}/${fileName}`;

      await writeWorkflowFile(targetPath, content, dryRun, denoFs, console.log);

      // Track file in metadata
      if (!dryRun) {
        addFileToMetadata(metadata, fileName, filePath);
      }

      successCount++;
    } catch (error) {
      console.error(`❌ Error installing ${filePath}: ${error instanceof Error ? error.message : String(error)}`);
      errorCount++;
    }
  }

  // Save metadata
  if (!dryRun && successCount > 0) {
    await saveMetadata(claudeDir, metadata, denoFs);
  }

  console.log(`\n✅ Installation ${dryRun ? "preview" : "complete"}!`);
  console.log(`   Successfully processed: ${successCount}`);
  if (errorCount > 0) {
    console.log(`   Errors: ${errorCount}`);
  }

  mis.outputSuccess({
    message: dryRun ? "Add workflow complete (dry-run)" : "Add workflow complete",
    scope: scope,
    target_directory: rulesDir,
    selected_core: selectedCore,
    selected_stacks: selectedStackNames,
    total_files: allFiles.length,
    successful: successCount,
    errors: errorCount,
    dry_run: dryRun,
  });
} catch (error) {
  mis.outputError(error instanceof Error ? error.message : String(error));
}
