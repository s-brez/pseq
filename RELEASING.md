# Releasing pseq

Releases are tag-driven. Merges to `master` run CI, but package publication only starts from an annotated `v<version>` tag whose version matches the source tree.

## npm trusted publishing

Configure the npm package `@s-brez/pseq` with a trusted publisher:

- Publisher: GitHub Actions
- Organization or user: `s-brez`
- Repository: `pseq`
- Workflow filename: `release.yml`
- Environment name: leave blank
- Allowed action: `npm publish`

The release workflow uses GitHub Actions OIDC, so no `NPM_TOKEN` secret is required.

## GitHub setup

No npm secret is required in GitHub.

In repository settings, check:

- Actions are enabled for the repository.
- The repository or organization policy allows the actions used by the workflows.
- `GITHUB_TOKEN` is allowed to create releases. The workflow requests `contents: write` only in the final publish job.

Restrict `v*` tag creation to maintainers if tag rulesets are available for the repository. A pushed annotated `v*` tag is the release boundary.

## Dry runs

CI runs on branch pushes and pull requests.

The release workflow has a pull-request dry run for release/package changes. It builds all platform binaries, stages the npm package, packs the tarball, smoke-tests the packed package, prepares release archives, and uploads those archives as workflow artifacts. It does not publish to npm or create a GitHub Release unless the workflow is running from a pushed annotated `v*` tag.

After the workflow exists on `master`, it can also be manually dry-run from the GitHub Actions UI with the `workflow_dispatch` trigger. Manual runs are always dry runs.

## Prepare a release

```bash
./scripts/prepare-release 0.0.3
rtk cargo test --locked
rtk npm --prefix npm/pseq run verify:metadata
rtk git diff
rtk git commit -am "release 0.0.3"
rtk git tag -a v0.0.3 -m "Release 0.0.3"
rtk git push origin master v0.0.3
```

The tag workflow validates the tag and source versions, builds all platform binaries, stages and verifies the npm package, packs a tarball, smoke-tests the packed package, publishes that tarball to npm, and then creates the GitHub Release with archives and checksums.
