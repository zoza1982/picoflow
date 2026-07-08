# Demo recordings

The GIFs shown in the project [README](../../README.md) are generated with
[VHS](https://github.com/charmbracelet/vhs) from the `.tape` scripts in this directory,
so they are fully reproducible and version-controlled.

| Tape | GIF | Shows |
|------|-----|-------|
| `01-quickstart.tape` | `01-quickstart.gif` | Author a workflow in YAML and `validate` it (DAG + execution order) |
| `02-run.tape` | `02-run.gif` | `run` the workflow — parallel execution by DAG levels |
| `03-inspect.tape` | `03-inspect.gif` | `stats` / `status` observability after a run |

All three drive [`pipeline.yaml`](pipeline.yaml), a small ETL-style DAG
(`fetch` → `clean`/`enrich`/`validate_data` in parallel → `load`).

## Regenerate

Install VHS and its dependencies (`ttyd`, `ffmpeg`), then build picoflow and render:

```bash
# deps (Debian/Ubuntu): apt-get install ffmpeg ttyd; install VHS from its releases
cargo build --release
mkdir -p bin && cp target/release/picoflow bin/     # the tapes expect ./bin/picoflow on PATH
cd docs/demos
vhs 01-quickstart.tape
vhs 02-run.tape
vhs 03-inspect.tape
```

Each tape sets `PATH="$PWD/bin:$PATH"` in a hidden setup step, so it records the local
binary. Adjust `Set Theme`, `Set FontSize`, or `Set Width/Height` in a tape to restyle.
