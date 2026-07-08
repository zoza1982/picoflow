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

Install VHS and its dependencies (`ttyd`, `ffmpeg`), then build picoflow and run the
render script:

```bash
# deps (Debian/Ubuntu): apt-get install ffmpeg ttyd gifsicle; install VHS from its releases
cargo build --release
docs/demos/render.sh
```

`render.sh` stages the freshly built binary at `docs/demos/bin/picoflow` (the tapes put it
on `PATH` via a hidden setup step), renders each tape with VHS, then uses `ffmpeg`'s `tpad`
filter to **freeze the final frame for a few seconds** — VHS trims trailing idle frames, so
the closing "digest" pause is added in post. Adjust the `HOLD` values in `render.sh`, or
`Set TypingSpeed` / `Sleep` in a tape, to retune pacing.
