# Coverage Reports

This directory is intended for generated HTML coverage reports.

From file_vault/, generate the report here:

```bash
cargo llvm-cov clean --workspace
cargo llvm-cov --html --output-dir ../docs/coverage
```

The main report entry point is `html/index.html` after generation.
