# PicoFlow Example Workflows

This directory contains production-ready example workflows demonstrating PicoFlow's capabilities for edge device orchestration.

## Overview

All workflows are designed for resource-constrained edge devices (Raspberry Pi Zero 2 W baseline) and demonstrate real-world use cases for DevOps, IoT, and edge computing scenarios.

## Available Workflows

### 1. Multi-Service Health Monitoring
**File:** `health-check.yaml`
**Schedule:** Every 5 minutes
**Use Case:** Monitor health of multiple services (API, database, cache, queue) with automated notifications

**Features:**
- Parallel health checks for multiple services
- HTTP-based health endpoints
- Conditional success/failure notifications
- Slack and webhook integrations
- Critical vs. non-critical service handling

**Target Audience:** DevOps engineers managing distributed edge infrastructure

**Executors Used:** HTTP, Shell

**Estimated Runtime:** 20-30 seconds

---

### 2. IoT Data Collection Pipeline
**File:** `data-pipeline.yaml`
**Schedule:** Every 10 minutes
**Use Case:** Collect sensor data from IoT devices, process, and upload to central storage

**Features:**
- Parallel data collection from multiple sensors (temperature, humidity, pressure)
- JSON processing and validation with jq
- Data compression for efficient transfer
- SSH-based remote upload
- Automated cleanup of temporary files
- Success notification to monitoring system

**Target Audience:** IoT developers, smart home enthusiasts, industrial edge deployments

**Executors Used:** HTTP, Shell, SSH

**Estimated Runtime:** 2-3 minutes

**Hardware Example:** Raspberry Pi Zero 2 W with I2C sensors (BME280, DHT22)

---

### 3. Comprehensive Backup Pipeline
**File:** `backup-comprehensive.yaml`
**Schedule:** Daily at 2 AM
**Use Case:** Production-grade database backup with health checks, verification, and retention management

**Features:**
- Pre-backup health checks (HTTP)
- Disk space verification
- PostgreSQL database dump via SSH
- Compression (gzip) for space efficiency
- Secure transfer to backup server (SCP)
- Backup integrity verification (gzip test, checksums)
- Restore testing (validation)
- Automated cleanup of old backups (30-day retention)
- Multi-channel notifications (webhook + Slack)
- Comprehensive reporting

**Target Audience:** DevOps/SRE teams, database administrators, backup engineers

**Executors Used:** HTTP, Shell, SSH

**Estimated Runtime:** 20-40 minutes (depends on database size)

**Production Notes:**
- Supports databases up to 100GB+
- Implements 30-day retention policy
- Includes recovery procedures in comments
- Ready for production deployment

---

### 4. Application Deployment Pipeline
**File:** `deployment.yaml`
**Schedule:** On-demand (manual trigger)
**Use Case:** Deploy applications to edge devices with automated testing, health checks, and rollback capability

**Features:**
- Pre-deployment health checks
- Backup of current deployment
- Git-based code deployment
- Automated dependency installation
- Test execution before deployment
- Application build process
- Post-deployment health verification
- Smoke tests
- Rollback procedures (manual trigger)
- Multi-stage notifications
- Deployment audit trail

**Target Audience:** DevOps engineers, platform teams, edge application developers

**Executors Used:** HTTP, Shell, SSH

**Estimated Runtime:** 15-30 minutes (depends on application size)

**Supported Stacks:**
- Rust (Cargo)
- Node.js (npm)
- Python (pip)
- Any systemd-managed service

**Safety Features:**
- Sequential execution for safety
- Automatic backup before deployment
- Health check verification
- Rollback capability
- Smoke tests after deployment

---

## Getting Started

### Prerequisites

1. **PicoFlow Installation:**
   ```bash
   # Install PicoFlow
   cargo install picoflow
   # Or build from source
   cargo build --release
   ```

2. **Environment Variables:**
   Each workflow requires specific environment variables for configuration. See the comments in each YAML file for required variables.

3. **SSH Keys:**
   For workflows using SSH executor:
   ```bash
   # Generate SSH key if needed
   ssh-keygen -t ed25519 -f ~/.ssh/picoflow_deploy_key

   # Copy to target servers
   ssh-copy-id -i ~/.ssh/picoflow_deploy_key.pub user@target-host
   ```

### Running Workflows

#### Validate Workflow
```bash
picoflow validate examples/workflows/health-check.yaml
```

#### Run Once (Manual Execution)
```bash
picoflow run examples/workflows/data-pipeline.yaml
```

#### Run as Scheduled Daemon
```bash
# Start daemon with scheduled workflows
picoflow daemon examples/workflows/health-check.yaml examples/workflows/backup-comprehensive.yaml

# Daemon runs in background, executing workflows per their schedule
```

#### Check Workflow Status
```bash
picoflow status health-check.yaml
```

#### View Execution Logs
```bash
picoflow logs health-check.yaml --tail 50
```

#### View Execution History
```bash
picoflow history health-check.yaml --limit 10
```

---

## Configuration Examples

### Setting Environment Variables

Create a `.env` file or export variables:

```bash
# Health Check Configuration
export API_HOST="api.example.com"
export API_HEALTH_TOKEN="your-token-here"
export DB_PROXY_HOST="db-proxy.example.com"
export WEBHOOK_HOST="monitoring.example.com"
export SLACK_WEBHOOK_URL="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
export DEVICE_ID="rpi-zero-001"

# IoT Pipeline Configuration
export IOT_API_KEY="your-iot-api-key"
export DEVICE_LOCATION="warehouse-1"
export STORAGE_HOST="storage.example.com"
export STORAGE_USER="backup"
export SSH_KEY_PATH="/home/pi/.ssh/id_rsa"

# Backup Configuration
export DB_HOST="postgres.example.com"
export DB_USER="backup_user"
export DB_NAME="production_db"
export BACKUP_HOST="backup-server.example.com"

# Deployment Configuration
export DEPLOY_TARGET_HOST="edge-device-1.example.com"
export DEPLOY_USER="deploy"
export APP_SERVICE_NAME="myapp.service"
export APP_DEPLOY_PATH="/opt/myapp"
export DEPLOY_BRANCH="main"
```

### Using Environment Files

PicoFlow supports `.env` files:

```bash
# Create .env file
cat > .env <<EOF
API_HOST=api.example.com
API_HEALTH_TOKEN=secret-token
EOF

# Run with environment file
picoflow run --env-file .env examples/workflows/health-check.yaml
```

---

## Workflow Patterns

### Parallel Execution
```yaml
config:
  max_parallel: 3  # Run up to 3 tasks simultaneously

tasks:
  - name: task1
    type: http
    # No depends_on - runs immediately

  - name: task2
    type: http
    # No depends_on - runs in parallel with task1

  - name: task3
    type: shell
    depends_on: [task1, task2]  # Waits for both to complete
```

### Error Handling
```yaml
tasks:
  - name: critical_task
    type: ssh
    retry: 3  # Retry up to 3 times on failure
    timeout: 60  # Timeout after 60 seconds

  - name: optional_task
    type: http
    continue_on_failure: true  # Don't fail workflow if this fails
```

### Scheduling
```yaml
# Cron format: minute hour day month weekday
schedule: "*/5 * * * *"  # Every 5 minutes
schedule: "0 2 * * *"    # Daily at 2 AM
schedule: "0 0 * * 0"    # Weekly on Sunday at midnight
schedule: "0 0 1 * *"    # Monthly on 1st at midnight
```

---

## Security Best Practices

### 1. Never Hardcode Secrets
❌ **Bad:**
```yaml
config:
  url: "https://api.example.com"
  headers:
    Authorization: "Bearer my-secret-token"  # DON'T DO THIS
```

✅ **Good:**
```yaml
config:
  url: "https://api.example.com"
  headers:
    Authorization: "Bearer ${API_TOKEN}"  # Use environment variable
```

### 2. Use SSH Keys (Not Passwords)
```yaml
# SSH executor uses key-based authentication by default
- name: remote_task
  type: ssh
  config:
    host: "server.example.com"
    user: "deploy"
    # PicoFlow uses SSH agent or default key locations
    # No password support
```

### 3. Restrict File Permissions
```bash
# Protect SSH keys
chmod 600 ~/.ssh/picoflow_deploy_key

# Protect environment files
chmod 600 .env
```

### 4. Use Dedicated Service Accounts
- Create separate users for deployments (e.g., `deploy`, `backup`)
- Grant minimal required permissions
- Use sudo only when absolutely necessary

### 5. Rotate Credentials Regularly
- API tokens: Rotate every 90 days
- SSH keys: Rotate annually
- Database passwords: Use secrets manager

---

## Performance Optimization

### 1. Tune Parallelism
```yaml
config:
  max_parallel: 2  # Adjust based on device resources
  # Raspberry Pi Zero 2 W: max_parallel: 2-3
  # Raspberry Pi 4: max_parallel: 4-8
```

### 2. Set Appropriate Timeouts
```yaml
tasks:
  - name: quick_check
    timeout: 10  # Fast operations

  - name: database_backup
    timeout: 900  # 15 minutes for large operations
```

### 3. Use Compression
```bash
# Compress large files before transfer
gzip -9 large_file.json
# Reduces network transfer time by 5-10x
```

### 4. Clean Up Temporary Files
```yaml
- name: cleanup
  type: shell
  config:
    command: "rm -f /tmp/temp_*.json"
  continue_on_failure: true
```

---

## Monitoring and Observability

### 1. Workflow Status
```bash
# Check if workflow is running
picoflow status workflow.yaml

# View recent executions
picoflow history workflow.yaml --limit 10
```

### 2. Logs
```bash
# Tail logs in real-time
picoflow logs workflow.yaml --follow

# View logs for specific execution
picoflow logs workflow.yaml --execution-id abc123
```

### 3. Metrics
PicoFlow exposes Prometheus metrics:
```bash
# Scrape metrics endpoint
curl http://localhost:9090/metrics
```

Key metrics:
- `picoflow_workflow_executions_total`
- `picoflow_workflow_duration_seconds`
- `picoflow_task_failures_total`

### 4. Notifications
All example workflows include notification tasks:
- Webhook notifications (generic)
- Slack notifications (chat)
- HTTP callbacks (custom integrations)

---

## Troubleshooting

### Workflow Fails to Start
```bash
# Validate YAML syntax
picoflow validate workflow.yaml

# Check environment variables
env | grep API_

# Verify SSH connectivity
ssh -i ~/.ssh/picoflow_key user@host
```

### Task Timeout
```yaml
# Increase timeout for long-running tasks
- name: slow_task
  timeout: 1800  # 30 minutes
```

### SSH Connection Failures
```bash
# Test SSH manually
ssh -v user@host

# Check SSH key permissions
ls -la ~/.ssh/

# Verify SSH agent
ssh-add -l
```

### Memory Issues
```bash
# Check PicoFlow memory usage
ps aux | grep picoflow

# Monitor system memory
free -h

# Reduce parallelism if needed
max_parallel: 1  # Sequential execution
```

---

## Advanced Topics

### Custom Executors
PicoFlow supports Shell, SSH, and HTTP executors. For other integrations:

```yaml
# Use shell executor to call external tools
- name: custom_integration
  type: shell
  config:
    command: "/usr/local/bin/custom-tool"
    args: ["--option", "value"]
```

### Conditional Execution
Currently, dependencies create conditional execution:

```yaml
- name: deploy
  depends_on: [test]  # Only runs if test succeeds
```

### Task Outputs
Tasks can write to temp files for data passing:

```bash
# Task 1: Generate data
echo "DATA=value" > /tmp/task_output.sh

# Task 2: Read data
source /tmp/task_output.sh
echo "Using $DATA"
```

---

## Contributing

Have a useful workflow example? Contributions are welcome!

1. Create workflow in `examples/workflows/`
2. Add comprehensive comments
3. Test on Raspberry Pi Zero 2 W (or similar)
4. Document environment variables required
5. Submit pull request

---

## Resources

- **PicoFlow Documentation:** [docs/](../../docs/)
- **PRD:** [PRD.md](../../PRD.md)
- **Architecture:** [ARCHITECTURE.md](../../ARCHITECTURE.md)
- **Issues & Roadmap:** [GitHub Issues](https://github.com/your-org/picoflow/issues)

---

## License

PicoFlow is open source software. See [LICENSE](../../LICENSE) for details.

---

## Support

- **GitHub Issues:** Report bugs or request features
- **Discussions:** Ask questions, share workflows
- **Email:** support@picoflow.dev (for commercial support)

---

**Happy Orchestrating! 🚀**
