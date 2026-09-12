import { randomUUID } from "node:crypto";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function normalizeVersion(version) {
  return version.trim().replace(/^v/i, "");
}

export function extractReleaseNotes(changelog, version) {
  const normalizedVersion = normalizeVersion(version);
  const lines = changelog.replace(/\r\n/g, "\n").split("\n");
  const headingPattern = new RegExp(`^##\\s+${escapeRegExp(normalizedVersion)}(?:\\s+-\\s+.*)?\\s*$`);
  const startIndex = lines.findIndex((line) => headingPattern.test(line.trim()));

  if (startIndex === -1) {
    throw new Error(`CHANGELOG.md does not contain a section for ${normalizedVersion}.`);
  }

  const nextSectionIndex = lines.findIndex((line, index) => index > startIndex && /^##\s+\S/.test(line.trim()));
  const endIndex = nextSectionIndex === -1 ? lines.length : nextSectionIndex;
  const section = trimBlankLines(lines.slice(startIndex, endIndex));
  const body = section.join("\n").trim();

  if (!body || section.length <= 1) {
    throw new Error(`CHANGELOG.md section for ${normalizedVersion} is empty.`);
  }

  return `${body}\n`;
}

function trimBlankLines(lines) {
  let start = 0;
  let end = lines.length;

  while (start < end && lines[start].trim() === "") {
    start += 1;
  }
  while (end > start && lines[end - 1].trim() === "") {
    end -= 1;
  }

  return lines.slice(start, end);
}

function readPackageVersion(packageJsonPath) {
  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
  if (typeof packageJson.version !== "string" || packageJson.version.trim() === "") {
    throw new Error(`${packageJsonPath} does not contain a string version.`);
  }
  return packageJson.version;
}

function parseArgs(argv) {
  const args = {
    changelog: "CHANGELOG.md",
    packageJson: "package.json",
    output: "release-notes.md",
    version: "",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (!flag.startsWith("--") || value === undefined) {
      throw new Error(`Invalid argument: ${flag}`);
    }
    index += 1;

    if (flag === "--changelog") {
      args.changelog = value;
    } else if (flag === "--package-json") {
      args.packageJson = value;
    } else if (flag === "--output") {
      args.output = value;
    } else if (flag === "--version") {
      args.version = value;
    } else {
      throw new Error(`Unknown argument: ${flag}`);
    }
  }

  return args;
}

function writeGitHubOutput(name, value) {
  if (!process.env.GITHUB_OUTPUT) {
    return;
  }

  const delimiter = `release_notes_${randomUUID().replace(/-/g, "")}`;
  appendFileSync(process.env.GITHUB_OUTPUT, `${name}<<${delimiter}\n${value}${delimiter}\n`);
}

export function main(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const version = args.version || readPackageVersion(args.packageJson);
  const releaseNotes = extractReleaseNotes(readFileSync(args.changelog, "utf8"), version);

  writeFileSync(args.output, releaseNotes);
  writeGitHubOutput("release_body", releaseNotes);
  process.stdout.write(releaseNotes);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
