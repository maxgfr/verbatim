// Stamp a release version into the Cargo workspace before the CLI is built,
// so `verbatim --version` answers the tag's version (Homebrew's test checks it).
const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+/.test(version)) {
  console.error("usage: bun scripts/set-version.ts <version>");
  process.exit(1);
}
const path = new URL("../Cargo.toml", import.meta.url).pathname;
const toml = await Bun.file(path).text();
const next = toml.replace(/(\[workspace\.package\]\nversion = )"[^"]*"/, `$1"${version}"`);
if (next === toml && !toml.includes(`version = "${version}"`)) {
  console.error("no [workspace.package] version found in Cargo.toml");
  process.exit(1);
}
await Bun.write(path, next);
console.log(`Cargo workspace version ${version}`);
