const ARCHIVE_REPOSITORY = "damageboy/yt-dlp-tauri";

// Historical comparison preserves executable identity, but reads only our verified mirror.
export function rebindMigratedBaseline(manifest, inventory) {
  if (inventory.repository !== ARCHIVE_REPOSITORY) {
    throw new Error("Baseline migration inventory must describe the owned repository");
  }
  const sources = new Map(inventory.releases.flatMap((release) => release.assets.map((asset) => {
    const suffix = `/releases/download/${release.tag}/${encodeURIComponent(asset.name)}`;
    return [`https://github.com/${inventory.sourceRepository}${suffix}`, {
      ...asset,
      url: `https://github.com/${ARCHIVE_REPOSITORY}${suffix}`,
    }];
  })));
  const baseline = structuredClone(manifest);
  for (const target of baseline.targets) {
    for (const tool of target.tools) {
      if (tool.sourceUrl.startsWith(`https://github.com/${ARCHIVE_REPOSITORY}/releases/download/`)) continue;
      const asset = sources.get(tool.sourceUrl);
      if (!asset || tool.sourceSize !== asset.size || tool.sourceSha256 !== asset.sha256) {
        throw new Error(`Baseline ${target.target}/${tool.name} has no byte-identical inventoried archive`);
      }
      tool.sourceUrl = asset.url;
    }
  }
  return baseline;
}
