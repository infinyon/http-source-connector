---
name: Release
about: Checklist for release
title: "Release "
labels: tracking 
assignees:
---

- [ ] Check that the version in `crates/http-source/Connector.toml` has been incremented. If needed increment it.
- [ ] Tag the intended release and push it to the repository. This will start the [Publish Hub](https://github.com/infinyon/http-source-connector/actions/workflows/publish.yaml) wokflow.
- [ ] Search [fluvio-website repo](https://github.com/infinyon/fluvio-website/) for documentation examples and update version in examples and documentation of features.

