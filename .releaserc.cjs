// semantic-release: version from Conventional Commits and a GitHub release
// with notes. No commit is pushed back: the version lives in the tag. The
// release workflow then builds the CLI per OS at that tag and attaches it;
// maxgfr/homebrew-tap picks the binaries up from the release.
module.exports = {
  branches: ["main"],
  plugins: [
    ["@semantic-release/commit-analyzer", { preset: "conventionalcommits" }],
    ["@semantic-release/release-notes-generator", { preset: "conventionalcommits" }],
    ["@semantic-release/exec", { successCmd: 'echo "published=true" >> "$GITHUB_OUTPUT" && echo "version=${nextRelease.version}" >> "$GITHUB_OUTPUT"' }],
    // A failed release shows in the Actions run; no tracking issue needed.
    ["@semantic-release/github", { failComment: false, failTitle: false, successComment: false }],
  ],
};
