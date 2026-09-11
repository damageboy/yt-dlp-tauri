import assert from "node:assert/strict";
import test from "node:test";
import { rebindMigratedBaseline } from "../scripts/toolchain/migration-baseline.mjs";

const inventory = {repository:"damageboy/yt-dlp-tauri",sourceRepository:"previous/toolchain",releases:[{tag:"toolchain-20260824.1",assets:[{name:"tool.zip",size:42,sha256:"a".repeat(64)}]}]};
const manifest = {targets:[{target:"win-x64",tools:[{name:"deno",sourceUrl:"https://github.com/previous/toolchain/releases/download/toolchain-20260824.1/tool.zip",sourceSize:42,sourceSha256:"a".repeat(64),sha256:"b".repeat(64)}]}]};

test("migration baseline uses only byte-identical inventoried owned archives", () => {
  const result = rebindMigratedBaseline(manifest, inventory);
  assert.equal(result.targets[0].tools[0].sourceUrl,"https://github.com/damageboy/yt-dlp-tauri/releases/download/toolchain-20260824.1/tool.zip");
  assert.equal(result.targets[0].tools[0].sha256,"b".repeat(64));
  assert.equal(manifest.targets[0].tools[0].sourceUrl.includes("previous/toolchain"),true);
  assert.deepEqual(rebindMigratedBaseline(result,inventory),result);
  for (const change of [{sourceSize:43},{sourceSha256:"c".repeat(64)},{sourceUrl:"https://github.com/other/repo/releases/download/toolchain-20260824.1/tool.zip"}]) {
    const broken=structuredClone(manifest); Object.assign(broken.targets[0].tools[0],change);
    assert.throws(() => rebindMigratedBaseline(broken,inventory), /inventoried/u);
  }
});
