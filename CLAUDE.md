# Release Process

1. Review changes since the last tag to build the changelog:
   ```
   git log $(git describe --tags --abbrev=0)..HEAD --oneline
   git diff <last-tag>..<first-new-commit>
   ```

2. Bump the version:
   ```
   cargo bump minor   # or major / patch
   git add Cargo.toml Cargo.lock
   git commit -m "Bump version to x.y.z"
   ```

3. Tag with the changelog as the message:
   ```
   git tag -a vx.y.z -m "..."
   ```

4. Push:
   ```
   git push origin master && git push origin vx.y.z
   ```
