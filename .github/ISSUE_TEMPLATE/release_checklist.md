---
name: Release
about: Checklist for release
title: "Release vX.Y.Z"
labels: tracking
assignees:
---

- [ ] Check that the version in `crates/http-source/Connector.toml` has been incremented. If needed increment it.
- [ ] Tag the intended release with the pattern `vX.Y.Z` matching the version `git tag vX.Y.Z -m "release"` and push it to the repository `git push --tags`. This will start the [Publish Hub](https://github.com/infinyon/http-source-connector/actions/workflows/publish.yaml) wokflow.
- [ ] Search [fluvio-website repo](https://github.com/infinyon/fluvio-website/) for documentation examples and update version in examples and documentation of features.
    - https://github.com/infinyon/fluvio-website/blob/master/embeds/connectors/inbound/http.md
    - https://github.com/infinyon/fluvio-website/blob/master/embeds/connectors/http-source-with-secrets.yaml
- [ ] Check that connector has been published `fluvio hub connector list`
