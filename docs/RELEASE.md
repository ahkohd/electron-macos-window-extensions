# Release

## Create Release

```bash
yarn run version [major|minor|patch]
git push --follow-tags
```

`napi version` bumps version, updates files, and creates git tag. Push triggers GitHub Actions to build and publish to npm.
