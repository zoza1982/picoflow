# PicoFlow FAQ (Frequently Asked Questions)

**Version:** 1.0.0
**Last Updated:** November 12, 2025

---

## Table of Contents

1. [General Questions](#general-questions)
2. [Comparison with Other Tools](#comparison-with-other-tools)
3. [Memory and Performance](#memory-and-performance)
4. [Edge Device Compatibility](#edge-device-compatibility)
5. [Security](#security)
6. [Workflow Design](#workflow-design)
7. [Scheduling](#scheduling)
8. [Features and Limitations](#features-and-limitations)
9. [Roadmap and Future Features](#roadmap-and-future-features)
10. [Contributing](#contributing)

---

## General Questions

### What is PicoFlow?

PicoFlow is a lightweight DAG (Directed Acyclic Graph) workflow orchestrator written in Rust, designed specifically for resource-constrained edge devices like the Raspberry Pi Zero 2 W. It provides enterprise-grade workflow orchestration with minimal memory footprint (<20MB idle).

### Why was PicoFlow created?

Traditional workflow orchestrators like Apache Airflow require 2GB+ RAM and are designed for datacenter environments. There was no good option for running complex DAG workflows on edge devices with limited resources. PicoFlow fills this gap with a 100x smaller footprint while maintaining production-grade features.

### Who should use PicoFlow?

- **Edge infrastructure engineers** managing IoT fleets
- **Homelab enthusiasts** running automation on Raspberry Pis
- **Embedded systems developers** building commercial IoT products
- **DevOps engineers** needing lightweight automation for minimal VMs
- **Anyone** needing workflow orchestration with minimal resource usage

### Is PicoFlow production-ready?

Yes! PicoFlow v1.0 includes:
- Comprehensive test coverage (>80%)
- Production-grade error handling and retry logic
- Persistent state management
- Graceful shutdown and crash recovery
- Security best practices
- Real-world testing on Raspberry Pi Zero 2 W

### Is PicoFlow open source?

Yes! PicoFlow is open source under the MIT License. You're free to use, modify, and distribute it. Contributions are welcome!

### What platforms does PicoFlow support?

- Linux ARM32 (Raspberry Pi Zero 2 W, Pi 3)
- Linux ARM64 (Raspberry Pi 4/5, modern SBCs)
- Linux x86_64 (standard servers, VMs)
- macOS (for development)

### What programming language is PicoFlow written in?

Rust! This enables:
- Memory safety without garbage collection
- Zero-cost abstractions for performance
- Small binary size
- Excellent concurrency support
- No runtime dependencies

---

## Comparison with Other Tools

### How does PicoFlow compare to Apache Airflow?

| Feature | PicoFlow | Airflow |
|---------|----------|---------|
| **Memory (idle)** | <20MB | 2GB+ |
| **Binary size** | 3MB | N/A (Python) |
| **Installation** | Single binary | Complex (Python, DB, web server) |
| **DAG support** | Yes | Yes |
| **Edge devices** | Yes (designed for) | No |
| **Web UI** | Planned (v1.1) | Yes |
| **Python integration** | Planned (v2.1) | Native |
| **Dynamic DAGs** | Planned (v1.2) | Yes |
| **Learning curve** | Low (YAML) | Medium-High (Python) |

**Use PicoFlow when:** Running on edge devices, minimizing resource usage, simple YAML workflows.

**Use Airflow when:** Complex Python-based workflows, large-scale data pipelines, need rich web UI.

### How does PicoFlow compare to Luigi?

| Feature | PicoFlow | Luigi |
|---------|----------|-------|
| **Memory** | <20MB | ~200MB |
| **Language** | Rust | Python |
| **DAG support** | Yes | Yes |
| **Scheduling** | Cron + daemon | External cron |
| **Edge devices** | Yes | Limited |
| **Retry logic** | Built-in | Built-in |

**Use PicoFlow when:** Memory is constrained, want integrated scheduling, prefer YAML over Python.

**Use Luigi when:** Complex Python workflows, existing Luigi infrastructure.

### How does PicoFlow compare to cron?

| Feature | PicoFlow | cron |
|---------|----------|------|
| **DAG support** | Yes | No |
| **Dependencies** | Yes | No (workarounds needed) |
| **Retry logic** | Yes | No |
| **Execution history** | Yes (SQLite) | No (manual logging) |
| **Parallel execution** | Yes | Limited |
| **Monitoring** | Prometheus metrics | Manual |

**Use PicoFlow when:** Need task dependencies, retry logic, execution history, monitoring.

**Use cron when:** Very simple scheduled tasks, no dependencies, minimal requirements.

### How does PicoFlow compare to systemd timers?

Similar to cron comparison. PicoFlow adds:
- DAG-based dependencies (vs systemd unit dependencies)
- Retry logic with exponential backoff
- Execution history and monitoring
- Cross-platform (systemd is Linux-only)

**Use PicoFlow when:** Need complex workflows, execution history, cross-platform support.

**Use systemd when:** Simple service automation, system-level tasks, tight OS integration.

### Can I migrate from Airflow/Luigi to PicoFlow?

Yes, but workflows need to be rewritten in YAML format. Benefits:
- Simpler configuration (YAML vs Python)
- 100x less memory usage
- Single binary deployment
- Built-in scheduling

Migration guide planned for v1.1.

---

## Memory and Performance

### How much memory does PicoFlow actually use?

Real-world measurements on Raspberry Pi Zero 2 W:

- **Idle (daemon mode):** 15-18MB RSS
- **Running 1 task:** 20-25MB RSS
- **Running 10 parallel tasks:** 40-50MB RSS
- **Peak (complex workflow):** ~50MB RSS

This is 100x less than Airflow (2GB+) and 10x less than Luigi (~200MB).

### What's the binary size?

Current v1.0: **3.0MB** (stripped release binary)

This is 70% under the 10MB target. You can copy it to any device and run immediately—no dependencies!

### Can PicoFlow handle large workflows?

Yes! Performance targets:

- **DAG parsing:** <50ms for 100 tasks
- **Task startup:** <100ms per task
- **Support:** 1000+ tasks per DAG

Tested with workflows containing 100+ tasks on Raspberry Pi Zero 2 W.

### How fast are workflows executed?

Execution speed depends on your tasks, not PicoFlow overhead. PicoFlow adds <100ms per task startup. Actual execution time depends on what your tasks do (shell commands, SSH operations, API calls).

Example: A 10-task workflow with 5-second tasks completes in ~50 seconds (vs 50 seconds for tasks alone = minimal overhead).

### Will PicoFlow work on devices with 256MB RAM?

Possibly! PicoFlow idles at <20MB, but you need memory for:
- Operating system (~50-100MB)
- Your tasks' memory usage
- Some headroom

**Recommendation:** 512MB minimum (tested baseline).

### Can I run PicoFlow on Raspberry Pi Zero (original, 512MB)?

The original Pi Zero (ARMv6) is not officially supported because Rust's official toolchain targets ARMv7+. However:
- Pi Zero 2 W (ARMv7, 512MB) is fully supported and tested
- You might be able to cross-compile for ARMv6 using custom toolchains (community effort)

---

## Edge Device Compatibility

### What Raspberry Pi models are supported?

| Model | Architecture | RAM | Status |
|-------|-------------|-----|--------|
| **Pi Zero 2 W** | ARMv7 (32-bit) | 512MB | ✅ Fully tested (baseline) |
| **Pi 3** | ARMv7/ARMv8 | 1GB | ✅ Supported |
| **Pi 4** | ARMv8 (64-bit) | 2-8GB | ✅ Supported |
| **Pi 5** | ARMv8 (64-bit) | 4-8GB | ✅ Supported |
| **Pi Zero** (original) | ARMv6 | 512MB | ⚠️ Not officially supported |

### What other single-board computers work?

Any ARM or x86_64 Linux SBC should work:

- **Orange Pi** (various models)
- **Rock Pi**
- **Odroid**
- **BeagleBone**
- **NVIDIA Jetson** (Nano, Xavier, etc.)

Requirements:
- ARM32 (ARMv7), ARM64, or x86_64 architecture
- Linux kernel 3.10+
- 512MB+ RAM (recommended)

### Can I run PicoFlow on ESP32 or microcontrollers?

No. PicoFlow requires:
- Operating system (Linux/macOS)
- Filesystem (for SQLite database and logs)
- Network stack (for SSH/HTTP)
- At least ~20MB RAM

ESP32 has only ~500KB RAM. Consider using PicoFlow on a Pi Zero to orchestrate ESP32 devices via HTTP/MQTT.

### Does PicoFlow work on cloud VMs?

Absolutely! PicoFlow excels on:
- **Minimal VMs** (512MB RAM instances)
- **Free tier instances** (AWS t2.micro, GCP f1-micro)
- **Cost-optimized deployments**

Example: Run 10+ workflow orchestrators for the cost of 1 Airflow instance.

### Can I use PicoFlow in Docker containers?

Yes! Official Docker images available:

```bash
docker pull ghcr.io/zoza1982/picoflow:latest

# Run workflow
docker run -v $(pwd)/workflows:/workflows \
  ghcr.io/zoza1982/picoflow:latest \
  run /workflows/backup.yaml
```

Container image size: ~10MB (Alpine-based).

---

## Security

### Is PicoFlow secure?

Yes! Security features:

- **SSH:** Key-based authentication only (no passwords)
- **Secrets:** Environment variables (not stored in YAML)
- **Input validation:** All user inputs sanitized
- **No code injection:** Command arguments passed as arrays, not shell strings
- **Minimal dependencies:** Rust's safety guarantees

Security audit completed as part of v1.0 release.

### Can I use password-based SSH?

**No.** PicoFlow only supports SSH key authentication for security reasons. Password auth is inherently less secure and not recommended for automation.

Setup SSH keys:
```bash
ssh-keygen -t ed25519 -f ~/.ssh/picoflow_key
ssh-copy-id -i ~/.ssh/picoflow_key.pub user@remote-host
```

### How do I handle secrets?

**Best practices:**

1. **Environment variables:**
```bash
export DB_PASSWORD="secret"
picoflow run workflow.yaml
```

```yaml
- name: backup
  type: ssh
  config:
    command: "pg_dump -h ${DB_HOST} -U ${DB_USER}"
```

2. **File references:**
```yaml
- name: api_call
  type: http
  config:
    headers:
      Authorization: "Bearer $(cat /run/secrets/api_token)"
```

3. **External secret managers** (HashiCorp Vault, AWS Secrets Manager):
```yaml
- name: fetch_secret
  type: shell
  config:
    command: "vault"
    args: ["kv", "get", "-field=password", "secret/db"]
```

**Never store secrets in workflow YAML files!**

### Can I run PicoFlow as root?

**Not recommended!** Security best practices:

1. Create dedicated user:
```bash
sudo useradd -r -s /bin/false picoflow
```

2. Run as that user:
```bash
sudo -u picoflow picoflow daemon start workflows/*.yaml
```

3. Use sudo in tasks only when necessary:
```yaml
- name: privileged_task
  type: shell
  config:
    command: "sudo"
    args: ["systemctl", "restart", "myservice"]
```

4. Configure sudoers for specific commands:
```bash
# /etc/sudoers.d/picoflow
picoflow ALL=(ALL) NOPASSWD: /bin/systemctl restart myservice
```

### Is the web UI secure?

Web UI (planned for v1.1) will:
- Bind to localhost by default (127.0.0.1)
- Be read-only (no workflow execution from UI)
- Not include authentication (use reverse proxy + auth if needed)

**Security recommendation:** Use SSH tunnel or reverse proxy with authentication for remote access.

### How do I audit what PicoFlow does?

1. **Enable audit logging:**
```toml
[logging]
audit_log = true
audit_file = "/var/log/picoflow/audit.log"
```

2. **Review execution history:**
```bash
picoflow history --format json > audit.json
```

3. **Database queries:**
```sql
sqlite3 picoflow.db "
  SELECT workflow_name, task_name, started_at, exit_code
  FROM task_executions
  ORDER BY started_at DESC;
"
```

---

## Workflow Design

### What's the difference between a workflow and a task?

- **Workflow:** Collection of tasks with dependencies (defined in YAML)
- **Task:** Single unit of work (shell command, SSH command, HTTP request)

```yaml
name: my-workflow  # This is the workflow

tasks:             # These are tasks
  - name: task1
  - name: task2
```

### How do I handle task dependencies?

Use the `depends_on` field:

```yaml
tasks:
  - name: task_a
    # Runs first (no dependencies)

  - name: task_b
    depends_on: [task_a]  # Runs after task_a completes

  - name: task_c
    depends_on: [task_a, task_b]  # Runs after both complete
```

### Can tasks run in parallel?

Yes! Tasks with no dependencies run in parallel (up to `max_parallel` limit):

```yaml
config:
  max_parallel: 4

tasks:
  - name: parallel_1
  - name: parallel_2
  - name: parallel_3
  # All three start simultaneously
```

### How do I pass data between tasks?

**v1.0 options:**

1. **Files:**
```yaml
- name: generate_data
  type: shell
  config:
    command: "/opt/generate.sh > /tmp/data.json"

- name: process_data
  depends_on: [generate_data]
  type: shell
  config:
    command: "/opt/process.sh /tmp/data.json"
```

2. **Environment variables:**
```yaml
- name: use_data
  type: shell
  config:
    command: "process.sh"
    env:
      INPUT_FILE: "/tmp/data.json"
```

**v1.2 (planned):** Rich data passing with JSON outputs and templating.

### What's the maximum workflow size?

**Tested limits:**
- **1000+ tasks** per workflow
- **Parsing time:** <50ms for 100 tasks
- **No hard limits** in code

**Practical recommendations:**
- Keep workflows under 100 tasks for maintainability
- Split large workflows into smaller, composable ones
- Use `continue_on_failure` for optional cleanup tasks

### Can I have dynamic workflows?

**v1.0:** No. Workflows are static (defined in YAML).

**v1.2 (planned):** Conditional execution and loops:
```yaml
- name: conditional_task
  on_success: [previous_task]  # Only if previous succeeded

- name: loop_task
  iterate_over: [item1, item2, item3]
```

### How do I handle failures?

Multiple strategies:

1. **Retry logic:**
```yaml
- name: flaky_task
  retry: 5  # Retry up to 5 times
```

2. **Continue on failure:**
```yaml
- name: optional_cleanup
  continue_on_failure: true  # Workflow continues even if this fails
```

3. **Dependent task only runs on success:**
```yaml
- name: send_success_notification
  depends_on: [critical_task]  # Only runs if critical_task succeeds
```

### Can I have multiple workflows scheduled?

Yes! Run daemon with multiple workflow files:

```bash
picoflow daemon start workflow1.yaml workflow2.yaml workflow3.yaml
```

Each workflow's schedule is independent.

---

## Scheduling

### What cron format does PicoFlow use?

**6-field format** (includes seconds):

```
┌─────────── second (0-59)
│ ┌───────── minute (0-59)
│ │ ┌─────── hour (0-23)
│ │ │ ┌───── day of month (1-31)
│ │ │ │ ┌─── month (1-12)
│ │ │ │ │ ┌─ day of week (0-6, Sunday = 0)
│ │ │ │ │ │
* * * * * *
```

Examples:
```yaml
# Daily at 2 AM
schedule: "0 0 2 * * *"

# Every 5 minutes
schedule: "0 */5 * * * *"

# Every Monday at 9 AM
schedule: "0 0 9 * * 1"
```

See [User Guide - Scheduling](user-guide.md#scheduling-with-cron) for more patterns.

### Can I run a workflow manually even if it's scheduled?

Yes! Two modes:

1. **Daemon mode:** Scheduled execution
```bash
picoflow daemon start workflow.yaml
```

2. **Manual execution:** Run once immediately
```bash
picoflow run workflow.yaml
```

Both modes work with the same YAML file.

### How do I prevent overlapping workflow runs?

**v1.0:** Not yet implemented. Workflows can overlap if previous run hasn't finished.

**Workaround:** Use file locks in your workflow:
```yaml
- name: acquire_lock
  type: shell
  config:
    command: "flock -n /tmp/workflow.lock -c 'sleep 0.1' || exit 1"

- name: main_task
  depends_on: [acquire_lock]
```

**v1.1 (planned):** Built-in `allow_concurrent: false` option.

### Can I schedule workflows at different times?

Yes! Each workflow has its own schedule:

```yaml
# backup.yaml
schedule: "0 0 2 * * *"  # 2 AM daily

# monitoring.yaml
schedule: "0 */5 * * * *"  # Every 5 minutes

# weekly-report.yaml
schedule: "0 0 9 * * 1"  # Monday 9 AM
```

### What timezone does PicoFlow use?

Uses the **system timezone** where PicoFlow is running.

Check your timezone:
```bash
timedatectl
date +%Z
```

Change timezone:
```bash
sudo timedatectl set-timezone America/New_York
```

**v1.1 (planned):** Per-workflow timezone configuration.

---

## Features and Limitations

### Does PicoFlow have a web UI?

**v1.0:** No web UI. Use CLI commands for monitoring:
```bash
picoflow status
picoflow history
picoflow logs --workflow myworkflow --follow
```

**v1.1 (planned):** Read-only web UI for:
- DAG visualization
- Execution history
- Real-time status
- Log viewer

### Can I trigger workflows via API?

**v1.0:** No HTTP API for triggering.

**Workarounds:**
1. Use cron-based scheduling
2. Trigger via SSH: `ssh pi@device "picoflow run workflow.yaml"`
3. Use systemd timers

**v1.1 (planned):** REST API for workflow triggering.

### Does PicoFlow support Docker executor?

**v1.0:** No. Use shell executor to run Docker commands:

```yaml
- name: run_container
  type: shell
  config:
    command: "docker"
    args: ["run", "--rm", "ubuntu", "echo", "Hello"]
```

**v1.1 (planned):** Native Docker executor (feature-gated, not recommended for Pi Zero).

### Can I use PicoFlow with Kubernetes?

**v1.0:** Not directly. But you can:
- Deploy PicoFlow as K8s Job/CronJob
- Use HTTP executor to call K8s API
- Use shell executor with kubectl

**v2.0 (planned):** K8s executor for running tasks as K8s Jobs.

### Does PicoFlow support distributed execution?

**v1.0:** No. Single-node execution only.

**v2.0 (planned):** Multi-node distributed execution with:
- Leader/worker architecture
- Task distribution
- Fault tolerance

For now, run multiple PicoFlow instances (one per device) with different workflows.

### Can I write tasks in Python or JavaScript?

**v1.0:** No. Only shell/SSH/HTTP executors.

**Workaround:** Use shell executor:
```yaml
- name: python_task
  type: shell
  config:
    command: "/usr/bin/python3"
    args: ["/opt/scripts/my_script.py", "--arg", "value"]
```

**v2.1 (planned):** Plugin system for Python/JS task definitions.

### What happens if PicoFlow crashes during execution?

PicoFlow persists state to SQLite database. On restart:
1. **Completed tasks:** Marked as complete (not re-run)
2. **Running tasks:** Marked as failed (assume crashed)
3. **Pending tasks:** Not started

**Current behavior:** Re-run the entire workflow.

**v1.1 (planned):** Resume from last checkpoint.

### How do I monitor PicoFlow?

Multiple options:

1. **CLI:**
```bash
picoflow status
picoflow history
picoflow logs --follow
```

2. **Prometheus metrics:**
```bash
curl http://localhost:9090/metrics
```

3. **Database queries:**
```sql
sqlite3 picoflow.db "SELECT * FROM workflow_executions ORDER BY started_at DESC LIMIT 10;"
```

4. **Systemd (if using):**
```bash
journalctl -u picoflow -f
```

---

## Roadmap and Future Features

### What's the current status?

**v1.0 (Current):** Production-ready release
- Shell, SSH, HTTP executors
- Cron scheduling
- Parallel execution
- Retry logic
- Prometheus metrics
- CLI tools
- 3MB binary, <20MB RAM

### What's coming in v1.1?

**Planned for Q2 2026:**
- Docker executor (feature-gated)
- Read-only Web UI
- REST API for triggering workflows
- Task output capture
- Resume from checkpoint
- Concurrent execution control

### What's coming in v1.2?

**Planned for Q3 2026:**
- Conditional task execution
- Loop constructs
- Enhanced templating
- Output artifacts
- Rich data passing between tasks

### Will there be a distributed version?

Yes! **v2.0 (planned for Q4 2026):**
- Multi-node execution
- Leader/worker architecture
- High availability
- Distributed task queue

Note: v2.0 targets clusters (3+ nodes), not individual edge devices.

### Will there be a Python API?

**v2.1 (planned for 2027):**
- Python task definitions
- JavaScript/Node.js tasks
- Plugin system
- Custom executors

For now, use shell executor to run Python scripts.

### Can I request features?

Yes! Feature requests welcome:
1. Check existing requests: https://github.com/zoza1982/picoflow/issues
2. File new request: https://github.com/zoza1982/picoflow/issues/new
3. Discuss in community: https://github.com/zoza1982/picoflow/discussions

Popular requests get prioritized!

---

## Contributing

### How can I contribute?

Many ways to help:

1. **Report bugs:** https://github.com/zoza1982/picoflow/issues
2. **Suggest features:** GitHub Discussions
3. **Improve docs:** Fix typos, add examples
4. **Write code:** See [CONTRIBUTING.md](../CONTRIBUTING.md)
5. **Share workflows:** Submit example workflows
6. **Spread the word:** Blog posts, social media

### I found a bug. What should I do?

1. Check if already reported: https://github.com/zoza1982/picoflow/issues
2. Enable debug logging: `picoflow --log-level debug run workflow.yaml`
3. File detailed bug report with:
   - PicoFlow version
   - Operating system
   - Steps to reproduce
   - Expected vs actual behavior
   - Logs and error messages

### I want to add a feature. How do I start?

1. **Discuss first:** Open GitHub Discussion or Issue
2. **Read contributing guide:** [CONTRIBUTING.md](../CONTRIBUTING.md)
3. **Check roadmap:** Align with project direction
4. **Start small:** Fix bugs, improve docs
5. **Submit PR:** Follow PR template

### Do you accept Pull Requests?

Yes! Requirements:
- Tests for new features
- Documentation updates
- Clippy and rustfmt compliance
- No breaking changes (without discussion)

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.

### How is PicoFlow maintained?

- **Core team:** Active maintainers review PRs and issues
- **Community:** Contributors help with code, docs, support
- **Releases:** Regular releases (monthly minor, quarterly major)
- **Support:** Best-effort community support via GitHub

### Can I sponsor PicoFlow development?

Not yet, but planned! Options being considered:
- GitHub Sponsors
- OpenCollective
- Patreon

For now, contributions (code, docs, feedback) are the best way to help.

### Is commercial support available?

**v1.0:** No commercial support yet.

**Future:** Considering offering:
- Priority support
- Custom features
- Consulting services
- Training

Contact via GitHub Discussions if interested.

---

## Still Have Questions?

- **Documentation:** https://github.com/zoza1982/picoflow/tree/main/docs
- **GitHub Issues:** https://github.com/zoza1982/picoflow/issues
- **Discussions:** https://github.com/zoza1982/picoflow/discussions
- **Examples:** https://github.com/zoza1982/picoflow/tree/main/examples

---

**Document Version:** 1.0.0
**Last Updated:** November 12, 2025
**Feedback:** Report issues at https://github.com/zoza1982/picoflow/issues
