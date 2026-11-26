# Upstream and web exports

Flow Web has two permanent source roles:

- `integration/desktop`: authoritative full Flow Desktop tree plus all Flow Web code and export configuration. Make changes here first, including web-specific changes.
- `main`: generated web distribution. It can be cloned and built without fetching an external submodule.

During the migration, `refactor/web-only` holds the candidate and `feat/web-runtime` remains the old working branch. Do not remove or merge them until the candidate is verified.

## Preparing an upstream update

From a clean full clone (Git and Python 3 required):

```bash
git remote add upstream https://github.com/Flow-Tube/Flow-Desktop.git # once
git fetch origin integration/desktop
git fetch upstream main
git switch -c update/flow-desktop origin/integration/desktop
git merge --no-ff upstream/main
git rev-parse upstream/main > FLOW_DESKTOP_REVISION
git add FLOW_DESKTOP_REVISION
git commit -m "chore: record Flow Desktop revision"
```

For a release, fetch the exact upstream tag into a dedicated remote ref and merge that ref instead of upstream/main. Record the commit actually merged. Resolve conflicts in the integration branch and run the web build, Rust tests and relevant native tests. Review and merge the integration PR before exporting. Never overwrite local changes or use force-push.

## Generate a candidate web branch

```bash
git fetch origin integration/desktop main
source_commit=$(git rev-parse origin/integration/desktop)
git switch --detach "$source_commit"
tree=$(python3 scripts/export-web.py "$source_commit")
candidate=$(git commit-tree "$tree" -p origin/main -p "$source_commit" -m "chore: export Flow Web")
git push origin "$candidate:refs/heads/web-export/$(git rev-parse --short "$source_commit")"
```

Open a PR to main and review the generated changes. The second parent preserves the complete corresponding integration source in Git history, and the export is independently reproducible from its pinned source. The tool does not move refs or modify working files itself. For the initial migration, the existing candidate uses the integration commit as its parent.

Check a web export from its own checkout:

```bash
python3 scripts/export-web.py "$(python3 -c 'import json; print(json.load(open("UPSTREAM.lock"))["integration_commit"])')" --verify HEAD
```

`UPSTREAM.lock` records the exact integration commit and, after an upstream merge has been recorded, the exact upstream commit. The initial export has no independently verified upstream revision yet.

## Export policy

The exporter works directly with Git blobs and preserves executable modes, symbolic links, source history and license. It removes native distribution workflows, signing, packaging and selected packaging scripts. Files in `web-export/overlay/` replace the matching paths in the public tree; update these templates on the integration branch rather than editing generated files.

The shared `src-tauri/` tree and changelog files intentionally remain in this first migration. The headless server imports Rust modules by relative path, migrations are linked, and the UI imports upstream changelogs. Moving or pruning the native tree requires a separate dependency audit and successful builds; it is not done blindly.

Automated upstream release detection, container publication and further pruning are follow-up steps. A successful export proves tree reproducibility, not browser compatibility or release readiness.
