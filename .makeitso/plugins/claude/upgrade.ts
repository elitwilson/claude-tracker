import {
  buildRepoUrl,
  fetchWorkflowFile,
  writeWorkflowFile,
  parseFrontmatter,
  calculateChecksum,
  discoverInstalledWorkflows,
  type FileSystem,
} from "./lib.ts";
import { denoFs, getHomeDir } from "./deno-fs.ts";
import { loadMetadata, getFileSource, getFileChecksum } from "./metadata.ts";

interface UpgradeResult {
  upgraded: string[];
  skipped: Array<{ file: string; reason: string }>;
  errors: Array<{ file: string; error: string }>;
}

/**
 * Compares two semver version strings
 * Returns true if remoteVersion > localVersion
 */
export function isNewerVersion(localVersion: string, remoteVersion: string): boolean {
  const parseVersion = (v: string) => {
    const parts = v.split(".").map(n => parseInt(n, 10));
    return { major: parts[0] || 0, minor: parts[1] || 0, patch: parts[2] || 0 };
  };

  const local = parseVersion(localVersion);
  const remote = parseVersion(remoteVersion);

  if (remote.major !== local.major) return remote.major > local.major;
  if (remote.minor !== local.minor) return remote.minor > local.minor;
  return remote.patch > local.patch;
}

/**
 * Performs the upgrade operation
 */
export async function performUpgrade(
  rulesDir: string,
  claudeDir: string,
  repoUrl: string,
  branch: string,
  force: boolean,
  dryRun: boolean,
  fs: FileSystem = denoFs
): Promise<UpgradeResult> {
  const result: UpgradeResult = {
    upgraded: [],
    skipped: [],
    errors: [],
  };

  // Load metadata to get source paths
  const metadata = await loadMetadata(claudeDir, fs);

  // Discover installed workflow files
  const installedFiles = await discoverInstalledWorkflows(rulesDir, fs);

  if (installedFiles.length === 0) {
    console.log("No workflow files found in .claude/rules/");
    return result;
  }

  console.log(`Found ${installedFiles.length} installed workflow file(s)\n`);

  // Process each installed file
  for (const filePath of installedFiles) {
    const fileName = filePath.split("/").pop()!;

    try {
      // Read local file
      const localContent = await fs.readFile(filePath);
      const localMeta = parseFrontmatter(localContent);

      if (!localMeta) {
        result.skipped.push({
          file: fileName,
          reason: "No version metadata found",
        });
        continue;
      }

      // Get source path from metadata
      const remotePath = getFileSource(metadata, fileName);

      if (!remotePath) {
        result.errors.push({
          file: fileName,
          error: "File not tracked in metadata (was it installed with an older version?)",
        });
        continue;
      }

      // Fetch remote file
      let remoteContent: string;
      try {
        remoteContent = await fetchWorkflowFile(repoUrl, branch, remotePath);
      } catch (error) {
        result.errors.push({
          file: fileName,
          error: error instanceof Error ? error.message : String(error),
        });
        continue;
      }

      const remoteMeta = parseFrontmatter(remoteContent);

      if (!remoteMeta) {
        result.errors.push({
          file: fileName,
          error: "Remote file has no version metadata",
        });
        continue;
      }

      // Check if update is needed
      if (!isNewerVersion(localMeta.version, remoteMeta.version)) {
        result.skipped.push({
          file: fileName,
          reason: `Already up to date (v${localMeta.version})`,
        });
        continue;
      }

      // Check for local modifications using stored checksum from metadata
      let isModified = false;
      const storedChecksum = getFileChecksum(metadata, fileName);
      if (storedChecksum) {
        const localChecksum = await calculateChecksum(localContent);
        isModified = localChecksum !== storedChecksum;
      }

      // Skip modified files unless --force
      if (isModified && !force) {
        result.skipped.push({
          file: fileName,
          reason: `Modified locally (use --force to overwrite)`,
        });
        continue;
      }

      // Perform the upgrade
      if (dryRun) {
        console.log(`Would upgrade: ${fileName} (v${localMeta.version} → v${remoteMeta.version})${isModified ? " [modified]" : ""}`);
      } else {
        await writeWorkflowFile(filePath, remoteContent, false, fs, () => {});
        console.log(`✅ Upgraded: ${fileName} (v${localMeta.version} → v${remoteMeta.version})${isModified ? " [overwrote modifications]" : ""}`);
      }

      result.upgraded.push(fileName);
    } catch (error) {
      result.errors.push({
        file: fileName,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  return result;
}

/**
 * Upgrades workflow files in both global and project scopes independently.
 * Returns separate results for each scope.
 */
export async function upgradeAll(
  projectRoot: string,
  homeDir: string,
  repoUrl: string,
  branch: string,
  force: boolean,
  dryRun: boolean,
  fs: FileSystem = denoFs
): Promise<{ global: UpgradeResult; project: UpgradeResult }> {
  const globalClaudeDir = `${homeDir}/.claude`;
  const globalRulesDir = `${globalClaudeDir}/rules`;

  const projectClaudeDir = `${projectRoot}/.claude`;
  const projectRulesDir = `${projectClaudeDir}/rules`;

  const [global, project] = await Promise.all([
    performUpgrade(globalRulesDir, globalClaudeDir, repoUrl, branch, force, dryRun, fs),
    performUpgrade(projectRulesDir, projectClaudeDir, repoUrl, branch, force, dryRun, fs),
  ]);

  return { global, project };
}

if (import.meta.main) {
  const { mis } = await import("../../mis-plugin-api.ts");

  try {
    const ctx = await mis.loadContext();

    console.log("🔄 Upgrade Claude Workflows\n");

    // Extract repo config
    const owner = mis.getConfig(ctx, "repo_owner", "elitwilson");
    const repo = mis.getConfig(ctx, "repo_name", "claude-workflows");
    const branch = mis.getConfig(ctx, "default_branch", "main");

    // Get flags
    const force = mis.getArg(ctx, "force") === "true" || mis.getArg(ctx, "force") === true;
    const dryRun = ctx.dry_run || false;

    const homeDir = getHomeDir();
    const repoUrl = buildRepoUrl(owner, repo);

    if (dryRun) {
      console.log("🔍 Dry-run mode: No files will be modified\n");
    }

    // Upgrade both global and project scopes
    const { global: globalResult, project: projectResult } = await upgradeAll(
      ctx.project_root,
      homeDir,
      repoUrl,
      branch,
      force,
      dryRun
    );

    // Print global summary
    console.log("\n🌐 Global (~/.claude/) Summary:");
    console.log(`   Upgraded: ${globalResult.upgraded.length}`);
    console.log(`   Skipped: ${globalResult.skipped.length}`);
    console.log(`   Errors: ${globalResult.errors.length}`);

    if (globalResult.skipped.length > 0) {
      console.log("\n⏭️  Skipped:");
      globalResult.skipped.forEach(({ file, reason }) => {
        console.log(`   - ${file}: ${reason}`);
      });
    }

    if (globalResult.errors.length > 0) {
      console.log("\n❌ Errors:");
      globalResult.errors.forEach(({ file, error }) => {
        console.log(`   - ${file}: ${error}`);
      });
    }

    // Print project summary
    console.log("\n📁 Project (.claude/) Summary:");
    console.log(`   Upgraded: ${projectResult.upgraded.length}`);
    console.log(`   Skipped: ${projectResult.skipped.length}`);
    console.log(`   Errors: ${projectResult.errors.length}`);

    if (projectResult.skipped.length > 0) {
      console.log("\n⏭️  Skipped:");
      projectResult.skipped.forEach(({ file, reason }) => {
        console.log(`   - ${file}: ${reason}`);
      });
    }

    if (projectResult.errors.length > 0) {
      console.log("\n❌ Errors:");
      projectResult.errors.forEach(({ file, error }) => {
        console.log(`   - ${file}: ${error}`);
      });
    }

    mis.outputSuccess({
      message: dryRun ? "Upgrade preview complete" : "Upgrade complete",
      global: {
        upgraded: globalResult.upgraded.length,
        skipped: globalResult.skipped.length,
        errors: globalResult.errors.length,
      },
      project: {
        upgraded: projectResult.upgraded.length,
        skipped: projectResult.skipped.length,
        errors: projectResult.errors.length,
      },
      dry_run: dryRun,
    });
  } catch (error) {
    mis.outputError(error instanceof Error ? error.message : String(error));
  }
}
