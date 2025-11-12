# PicoFlow Troubleshooting Guide

**Version:** 1.0.0
**Last Updated:** November 12, 2025

---

## Table of Contents

1. [General Troubleshooting](#general-troubleshooting)
2. [Workflow Validation Errors](#workflow-validation-errors)
3. [Execution Errors](#execution-errors)
4. [SSH Executor Issues](#ssh-executor-issues)
5. [HTTP Executor Issues](#http-executor-issues)
6. [Daemon Mode Problems](#daemon-mode-problems)
7. [Performance Issues](#performance-issues)
8. [Memory Issues](#memory-issues)
9. [Database Issues](#database-issues)
10. [Network Connectivity](#network-connectivity)
11. [Log Analysis](#log-analysis)
12. [Recovery Procedures](#recovery-procedures)

---

## General Troubleshooting

### Enable Debug Logging

The first step in troubleshooting is to enable detailed logging:

```bash
# Run with debug logging and pretty format
picoflow --log-level debug --log-format pretty run workflow.yaml

# Or set environment variable
export PICOFLOW_LOG_LEVEL=debug
picoflow run workflow.yaml

# For daemon mode
picoflow --log-level debug daemon start workflow.yaml
```

### Check System Requirements

```bash
# Check available memory
free -h

# Check disk space
df -h /var/lib/picoflow

# Check PicoFlow version
picoflow --version

# Verify binary integrity
sha256sum $(which picoflow)
```

### Validate Workflow Before Running

```bash
# Always validate first
picoflow validate workflow.yaml

# Check for common issues
picoflow validate --verbose workflow.yaml 2>&1 | grep -i error
```

### Check Daemon Status

```bash
# Check if daemon is running
picoflow daemon status

# Check process
ps aux | grep picoflow

# Check PID file
cat picoflow.pid

# With systemd
systemctl status picoflow
```

---

## Workflow Validation Errors

### Error: "Workflow has cycles"

**Symptom:**
```
Error: Workflow validation failed: DAG contains cycle
Tasks involved: task_a -> task_b -> task_c -> task_a
```

**Cause:** Circular dependencies in task graph.

**Solution:**

1. Review dependency chain:
```yaml
# BAD: Circular dependency
tasks:
  - name: task_a
    depends_on: [task_c]  # Points back to task_c
  - name: task_b
    depends_on: [task_a]
  - name: task_c
    depends_on: [task_b]  # Creates cycle
```

2. Fix by removing circular reference:
```yaml
# GOOD: Linear dependency
tasks:
  - name: task_a
    # No dependencies
  - name: task_b
    depends_on: [task_a]
  - name: task_c
    depends_on: [task_b]
```

3. Visualize your DAG on paper to identify cycles.

### Error: "Task not found in depends_on"

**Symptom:**
```
Error: Task 'task_b' depends on 'task_x' which does not exist
```

**Cause:** Referenced task name doesn't match any defined task.

**Solution:**

1. Check for typos in task names:
```yaml
# BAD: Typo in dependency
- name: backup_database
  depends_on: [health_chek]  # Should be health_check
```

2. Verify task exists:
```bash
# Extract all task names
grep "name:" workflow.yaml | grep -v "^name:"
```

3. Fix the reference:
```yaml
# GOOD: Correct reference
- name: backup_database
  depends_on: [health_check]  # Matches existing task
```

### Error: "Duplicate task name"

**Symptom:**
```
Error: Duplicate task name: 'backup'
```

**Cause:** Two or more tasks have the same name.

**Solution:**

1. Find duplicates:
```bash
grep -n "name:" workflow.yaml | grep -v "^[0-9]*:name:" | sort
```

2. Rename to unique identifiers:
```yaml
# BAD: Duplicate names
- name: backup
  type: shell
- name: backup
  type: ssh

# GOOD: Unique names
- name: backup_local
  type: shell
- name: backup_remote
  type: ssh
```

### Error: "Invalid YAML syntax"

**Symptom:**
```
Error: Failed to parse YAML: invalid type at line 15
```

**Cause:** YAML formatting error.

**Solution:**

1. Validate YAML syntax:
```bash
# Use yamllint or Python
python3 -c "import yaml; yaml.safe_load(open('workflow.yaml'))"
```

2. Common YAML mistakes:
```yaml
# BAD: Incorrect indentation
tasks:
- name: task1
  config:
  url: "http://example.com"  # Should be indented under config

# GOOD: Correct indentation
tasks:
  - name: task1
    config:
      url: "http://example.com"

# BAD: Unquoted special characters
command: echo $PATH  # $ needs quoting

# GOOD: Quoted strings
command: "echo $PATH"
```

3. Check for:
   - Consistent indentation (2 or 4 spaces, not tabs)
   - Quoted strings with special characters
   - Proper list syntax (`-` for items)
   - Matching brackets/braces

### Error: "Unknown executor type"

**Symptom:**
```
Error: Unknown executor type: 'docker'
```

**Cause:** Using unsupported executor type.

**Solution:**

Supported executors in v1.0:
- `shell`
- `ssh`
- `http`

```yaml
# BAD: Unsupported executor
type: docker  # Not available in v1.0

# GOOD: Use supported executor
type: shell
config:
  command: "docker"
  args: ["run", "ubuntu", "echo", "Hello"]
```

---

## Execution Errors

### Error: "Task timeout"

**Symptom:**
```
Task 'long_running_task' failed: Timeout after 300 seconds
```

**Cause:** Task exceeded configured timeout.

**Solution:**

1. Increase timeout for specific task:
```yaml
- name: long_running_task
  timeout: 1800  # 30 minutes
```

2. Or globally:
```yaml
config:
  timeout_default: 900  # 15 minutes
```

3. For very long tasks, disable timeout:
```yaml
- name: extremely_long_task
  timeout: 0  # No timeout (use with caution!)
```

4. Monitor task progress:
```bash
# Follow logs in real-time
picoflow logs --workflow myworkflow --task long_running_task --follow
```

### Error: "Max retries exceeded"

**Symptom:**
```
Task 'flaky_task' failed after 3 retry attempts
Last error: Connection refused
```

**Cause:** Task failed repeatedly despite retries.

**Solution:**

1. Investigate the root cause:
```bash
# Check task logs
picoflow logs --workflow myworkflow --task flaky_task

# Check if service is running
ssh user@host "systemctl status myservice"
```

2. Increase retry count:
```yaml
- name: flaky_task
  retry: 10  # More retries for unstable connections
```

3. Add delay between retries (handled automatically via exponential backoff)

4. Fix the underlying issue (network, permissions, service availability)

### Error: "Command not found"

**Symptom:**
```
Task 'backup' failed: Command not found: pg_dump
```

**Cause:** Command not in PATH or not installed.

**Solution:**

1. Use absolute path:
```yaml
# BAD: Relies on PATH
command: "pg_dump"

# GOOD: Absolute path
command: "/usr/bin/pg_dump"

# Find absolute path
# which pg_dump
```

2. Verify command exists:
```bash
# Local shell task
which pg_dump

# Remote SSH task
ssh user@host "which pg_dump"
```

3. Install missing package:
```bash
# Ubuntu/Debian
sudo apt install postgresql-client

# RHEL/CentOS
sudo yum install postgresql
```

### Error: "Permission denied"

**Symptom:**
```
Task 'write_file' failed: Permission denied
```

**Cause:** Insufficient permissions to execute command or access file.

**Solution:**

1. Check file permissions:
```bash
ls -la /path/to/file
```

2. Run PicoFlow as appropriate user:
```bash
# Run as specific user
sudo -u backupuser picoflow run workflow.yaml

# Or fix file permissions
chmod +x /path/to/script.sh
```

3. For SSH tasks, verify remote permissions:
```bash
ssh user@host "ls -la /path/to/file"
```

4. Use sudo in task (if configured):
```yaml
- name: privileged_task
  type: ssh
  config:
    host: "server.example.com"
    user: "deploy"
    command: "sudo systemctl restart myservice"
```

5. Add user to required group:
```bash
sudo usermod -aG docker picoflow  # For Docker access
sudo usermod -aG backup picoflow  # For backup files
```

---

## SSH Executor Issues

### Error: "SSH connection failed"

**Symptom:**
```
Task 'remote_backup' failed: SSH connection failed: Connection refused
```

**Cause:** Cannot establish SSH connection to remote host.

**Solutions:**

1. **Verify SSH service is running:**
```bash
# Check if host is reachable
ping -c 3 remote-host

# Check if SSH port is open
nc -zv remote-host 22

# On remote host
sudo systemctl status sshd
```

2. **Test SSH connection manually:**
```bash
ssh -i ~/.ssh/picoflow_key user@remote-host "echo Success"
```

3. **Check firewall:**
```bash
# On remote host
sudo ufw status
sudo iptables -L | grep 22
```

### Error: "SSH authentication failed"

**Symptom:**
```
Task 'remote_task' failed: Authentication failed
```

**Cause:** SSH key authentication issue.

**Solutions:**

1. **Verify key permissions:**
```bash
# Private key must be 600
ls -la ~/.ssh/picoflow_key
chmod 600 ~/.ssh/picoflow_key

# Public key should be 644
chmod 644 ~/.ssh/picoflow_key.pub
```

2. **Check authorized_keys on remote host:**
```bash
ssh user@remote-host "cat ~/.ssh/authorized_keys"
# Should contain your public key
```

3. **Test key authentication:**
```bash
ssh -i ~/.ssh/picoflow_key -v user@remote-host
# -v for verbose output showing auth attempts
```

4. **Check SSH agent:**
```bash
# Add key to agent
ssh-add ~/.ssh/picoflow_key

# List loaded keys
ssh-add -l
```

5. **Fix authorized_keys permissions:**
```bash
# On remote host
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys
```

### Error: "Host key verification failed"

**Symptom:**
```
Task 'ssh_task' failed: Host key verification failed
```

**Cause:** Host not in known_hosts file.

**Solutions:**

1. **Add host to known_hosts:**
```bash
ssh-keyscan -H remote-host >> ~/.ssh/known_hosts
```

2. **Or connect manually first:**
```bash
ssh user@remote-host
# Type 'yes' when prompted
```

3. **Verify known_hosts:**
```bash
ssh-keygen -F remote-host
```

### Error: "SSH command timeout"

**Symptom:**
```
Task 'long_ssh_command' failed: SSH command timeout
```

**Cause:** Remote command takes longer than timeout.

**Solutions:**

1. **Increase timeout:**
```yaml
- name: long_ssh_command
  type: ssh
  config:
    host: "server.example.com"
    user: "admin"
    command: "long_running_script.sh"
  timeout: 3600  # 1 hour
```

2. **Run command in background on remote host:**
```yaml
config:
  command: "nohup long_script.sh > /tmp/output.log 2>&1 &"
```

3. **Check command status in separate task:**
```yaml
- name: start_long_task
  type: ssh
  config:
    command: "nohup process.sh > /tmp/process.log 2>&1 & echo $!"

- name: check_completion
  type: ssh
  depends_on: [start_long_task]
  config:
    command: "wait $(cat /tmp/process.pid)"
```

---

## HTTP Executor Issues

### Error: "HTTP request failed: Connection timeout"

**Symptom:**
```
Task 'api_call' failed: Connection timeout after 30s
```

**Cause:** API endpoint not responding.

**Solutions:**

1. **Test endpoint manually:**
```bash
curl -v https://api.example.com/health
```

2. **Check DNS resolution:**
```bash
nslookup api.example.com
dig api.example.com
```

3. **Verify network connectivity:**
```bash
ping api.example.com
traceroute api.example.com
```

4. **Increase timeout:**
```yaml
- name: slow_api
  type: http
  config:
    url: "https://api.example.com/slow-endpoint"
    timeout: 120  # 2 minutes
```

5. **Add retries:**
```yaml
- name: api_call
  type: http
  config:
    url: "https://api.example.com/endpoint"
  retry: 5
```

### Error: "HTTP 401 Unauthorized"

**Symptom:**
```
Task 'api_call' failed: HTTP 401 Unauthorized
```

**Cause:** Missing or invalid authentication.

**Solutions:**

1. **Add authentication header:**
```yaml
- name: authenticated_api
  type: http
  config:
    url: "https://api.example.com/data"
    method: GET
    headers:
      Authorization: "Bearer ${API_TOKEN}"
```

2. **Set environment variable:**
```bash
export API_TOKEN="your-secret-token"
picoflow run workflow.yaml
```

3. **Test authentication manually:**
```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
     https://api.example.com/data
```

### Error: "HTTP 404 Not Found"

**Symptom:**
```
Task 'api_call' failed: HTTP 404 Not Found
```

**Cause:** Incorrect URL or endpoint doesn't exist.

**Solutions:**

1. **Verify URL:**
```bash
curl -I https://api.example.com/endpoint
```

2. **Check API documentation** for correct endpoint path

3. **Enable debug logging** to see full request:
```bash
picoflow --log-level debug run workflow.yaml
```

### Error: "HTTP 500 Internal Server Error"

**Symptom:**
```
Task 'api_call' failed: HTTP 500 Internal Server Error
```

**Cause:** Server-side error.

**Solutions:**

1. **Add retry logic** (server might be temporarily unavailable):
```yaml
- name: api_call
  type: http
  config:
    url: "https://api.example.com/endpoint"
  retry: 5
```

2. **Check API status page** for known issues

3. **Review request payload:**
```yaml
- name: api_post
  type: http
  config:
    url: "https://api.example.com/resource"
    method: POST
    body:
      field: "value"  # Ensure this matches API schema
```

4. **Test with curl:**
```bash
curl -X POST https://api.example.com/resource \
     -H "Content-Type: application/json" \
     -d '{"field":"value"}'
```

### Error: "SSL certificate verification failed"

**Symptom:**
```
Task 'https_call' failed: SSL certificate verify failed
```

**Cause:** Invalid or self-signed SSL certificate.

**Solutions:**

1. **For production:** Fix the SSL certificate on the server (never disable verification in production)

2. **For development/testing only:**
```yaml
# NOT RECOMMENDED FOR PRODUCTION
- name: dev_api_call
  type: http
  config:
    url: "https://dev-api.example.com"
    insecure_skip_verify: true  # Development only!
```

3. **Add custom CA certificate:**
```bash
# Add CA cert to system trust store
sudo cp custom-ca.crt /usr/local/share/ca-certificates/
sudo update-ca-certificates
```

---

## Daemon Mode Problems

### Error: "Daemon already running"

**Symptom:**
```
Error: Daemon already running (PID 12345)
```

**Cause:** PicoFlow daemon is already started.

**Solutions:**

1. **Check daemon status:**
```bash
picoflow daemon status
```

2. **Stop existing daemon:**
```bash
picoflow daemon stop
```

3. **Force kill if not responding:**
```bash
# Find PID
cat picoflow.pid

# Kill process
kill -9 $(cat picoflow.pid)

# Remove stale PID file
rm picoflow.pid
```

### Error: "Cannot write PID file"

**Symptom:**
```
Error: Cannot write PID file: Permission denied
```

**Cause:** No write permission in current directory.

**Solutions:**

1. **Change to writable directory:**
```bash
cd /var/lib/picoflow
picoflow daemon start workflows/*.yaml
```

2. **Fix permissions:**
```bash
sudo chown picoflow:picoflow /var/lib/picoflow
chmod 755 /var/lib/picoflow
```

3. **Run as appropriate user:**
```bash
sudo -u picoflow picoflow daemon start workflows/*.yaml
```

### Error: "Daemon crashed unexpectedly"

**Symptom:**
```
Daemon stopped running without clean shutdown
```

**Cause:** Process terminated unexpectedly.

**Solutions:**

1. **Check system logs:**
```bash
# With systemd
journalctl -u picoflow -n 100

# System messages
dmesg | grep picoflow

# Out of memory killer
grep -i "out of memory" /var/log/syslog
```

2. **Check for core dumps:**
```bash
ls -la /var/crash/
coredumpctl list
```

3. **Enable automatic restart with systemd:**
```ini
[Service]
Restart=on-failure
RestartSec=10s
```

4. **Increase memory limits:**
```ini
[Service]
MemoryLimit=256M  # Adjust as needed
```

---

## Performance Issues

### Issue: "Slow workflow execution"

**Symptoms:**
- Workflows taking much longer than expected
- High CPU usage
- Tasks queueing instead of running in parallel

**Diagnostics:**

1. **Check execution history:**
```bash
picoflow history --workflow slow-workflow
```

2. **Identify bottleneck tasks:**
```bash
picoflow stats --workflow slow-workflow
```

3. **Monitor system resources:**
```bash
# CPU and memory
top -p $(pgrep picoflow)

# I/O wait
iostat -x 5
```

**Solutions:**

1. **Increase parallelism:**
```yaml
config:
  max_parallel: 8  # Increase from default
```

2. **Optimize task dependencies:**
```yaml
# BAD: Unnecessary sequential execution
- name: task_b
  depends_on: [task_a]  # Not actually needed

# GOOD: Remove unnecessary dependency
- name: task_b
  # No dependency = runs in parallel
```

3. **Reduce task startup overhead:**
```yaml
# Combine small tasks
- name: multiple_checks
  type: shell
  config:
    command: "/bin/sh"
    args: ["-c", "check1 && check2 && check3"]
```

### Issue: "High memory usage"

**Symptoms:**
- Memory usage exceeding 50MB with 10 tasks
- System slowdown
- Out of memory errors

**Diagnostics:**

1. **Check memory usage:**
```bash
ps aux | grep picoflow | awk '{print $6/1024 " MB"}'

# Monitor over time
watch -n 5 'ps aux | grep picoflow'
```

2. **Check task logs size:**
```bash
du -sh logs/
find logs/ -type f -size +10M
```

**Solutions:**

1. **Reduce parallelism:**
```yaml
config:
  max_parallel: 2  # Reduce concurrent tasks
```

2. **Limit log retention:**
```yaml
[logs]
retention_days = 7
max_size_mb = 100
```

3. **Clean up old data:**
```bash
# Clean old logs
find logs/ -mtime +7 -delete

# Vacuum database
sqlite3 picoflow.db "VACUUM;"
```

4. **Redirect large output:**
```yaml
# Don't capture large stdout/stderr
- name: large_output_task
  type: shell
  config:
    command: "/opt/script.sh > /dev/null 2>&1"
```

### Issue: "Slow DAG parsing"

**Symptoms:**
- Long delay before workflow starts
- High CPU during validation

**Diagnostics:**

1. **Time the validation:**
```bash
time picoflow validate large-workflow.yaml
```

2. **Check workflow complexity:**
```bash
# Count tasks
grep "^  - name:" workflow.yaml | wc -l

# Count dependencies
grep "depends_on:" workflow.yaml | wc -l
```

**Solutions:**

1. **Split large workflows:**
```yaml
# Split into multiple smaller workflows
# workflow-part1.yaml (50 tasks)
# workflow-part2.yaml (50 tasks)
```

2. **Reduce dependency complexity:**
```yaml
# Avoid deeply nested dependencies
# Keep DAG levels under 10 if possible
```

---

## Memory Issues

### Error: "Out of memory"

**Symptom:**
```
Error: Cannot allocate memory
Killed (OOM killer)
```

**Cause:** System ran out of available memory.

**Solutions:**

1. **Check available memory:**
```bash
free -h
vmstat 5
```

2. **Identify memory consumers:**
```bash
ps aux --sort=-%mem | head -20
```

3. **Reduce parallel tasks:**
```yaml
config:
  max_parallel: 1  # Most conservative
```

4. **Add swap space:**
```bash
# Create 1GB swap file
sudo dd if=/dev/zero of=/swapfile bs=1M count=1024
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

5. **Stop other services:**
```bash
# Free up memory
sudo systemctl stop unnecessary-service
```

6. **Upgrade device** (if consistently hitting limits)

### Issue: "Memory leak"

**Symptoms:**
- Memory usage grows over time
- Daemon eventually crashes
- Need to restart daemon frequently

**Diagnostics:**

1. **Monitor memory over time:**
```bash
# Record memory every minute
while true; do
  date >> memory.log
  ps aux | grep picoflow >> memory.log
  sleep 60
done
```

2. **Check execution history size:**
```bash
ls -lh picoflow.db
sqlite3 picoflow.db "SELECT COUNT(*) FROM workflow_executions;"
```

**Solutions:**

1. **Clean old execution history:**
```bash
sqlite3 picoflow.db "
  DELETE FROM task_executions WHERE workflow_execution_id IN (
    SELECT id FROM workflow_executions
    WHERE started_at < datetime('now', '-30 days')
  );
  DELETE FROM workflow_executions
  WHERE started_at < datetime('now', '-30 days');
  VACUUM;
"
```

2. **Restart daemon periodically:**
```bash
# Add to crontab for weekly restart
0 3 * * 0 systemctl restart picoflow
```

3. **Report issue** with reproduction steps at https://github.com/zoza1982/picoflow/issues

---

## Database Issues

### Error: "Database locked"

**Symptom:**
```
Error: database is locked
```

**Cause:** Another process has locked the SQLite database.

**Solutions:**

1. **Check for multiple PicoFlow instances:**
```bash
ps aux | grep picoflow
# Kill extra instances
```

2. **Wait and retry:**
```yaml
# SQLite will retry automatically with timeout
# Usually resolves itself
```

3. **Check for stale locks:**
```bash
# Remove journal file if PicoFlow is not running
rm picoflow.db-journal
```

### Error: "Database corrupted"

**Symptom:**
```
Error: database disk image is malformed
```

**Cause:** Database file corrupted (power loss, disk failure).

**Solutions:**

1. **Backup existing database:**
```bash
cp picoflow.db picoflow.db.corrupted
```

2. **Try to recover:**
```bash
sqlite3 picoflow.db "PRAGMA integrity_check;"

# If recoverable
sqlite3 picoflow.db ".recover" | sqlite3 picoflow_recovered.db
mv picoflow_recovered.db picoflow.db
```

3. **Start fresh (loses history):**
```bash
rm picoflow.db
# PicoFlow will create new database on next run
```

### Issue: "Database growing too large"

**Symptoms:**
- Database file size in GB range
- Slow queries
- Running out of disk space

**Diagnostics:**

1. **Check database size:**
```bash
ls -lh picoflow.db
du -h picoflow.db
```

2. **Check record counts:**
```bash
sqlite3 picoflow.db "
  SELECT
    'workflow_executions' as table_name,
    COUNT(*) as count
  FROM workflow_executions
  UNION ALL
  SELECT
    'task_executions',
    COUNT(*)
  FROM task_executions;
"
```

**Solutions:**

1. **Clean old records:**
```bash
# Keep only last 30 days
sqlite3 picoflow.db "
  DELETE FROM task_executions WHERE workflow_execution_id IN (
    SELECT id FROM workflow_executions
    WHERE started_at < datetime('now', '-30 days')
  );
  DELETE FROM workflow_executions
  WHERE started_at < datetime('now', '-30 days');
"
```

2. **Vacuum database:**
```bash
sqlite3 picoflow.db "VACUUM;"
```

3. **Archive old data:**
```bash
# Export to CSV
sqlite3 -header -csv picoflow.db "
  SELECT * FROM workflow_executions
  WHERE started_at < datetime('now', '-90 days');
" > archive.csv

# Then delete from database
```

4. **Configure retention:**
```toml
[database]
retention_days = 30
auto_vacuum = true
```

---

## Network Connectivity

### Issue: "Intermittent connection failures"

**Symptoms:**
- SSH or HTTP tasks fail randomly
- Works sometimes, fails other times
- Higher failure rate at certain times

**Diagnostics:**

1. **Test connectivity:**
```bash
# Ping test
ping -c 100 remote-host | tail -5

# Connection stability
for i in {1..20}; do
  ssh user@host "date" || echo "Failed at $i"
  sleep 5
done
```

2. **Check network quality:**
```bash
# Packet loss and latency
mtr remote-host

# Network interface statistics
ifconfig -a
netstat -i
```

**Solutions:**

1. **Increase retries:**
```yaml
config:
  retry_default: 10  # More retries for unreliable networks

tasks:
  - name: remote_task
    type: ssh
    retry: 15  # Even more for critical tasks
```

2. **Add exponential backoff:**
```yaml
# Already built-in, but can configure
[retry]
backoff_multiplier = 2.0
max_backoff_seconds = 300
```

3. **Use mobile network fallback:**
```bash
# Configure network bonding or failover
# OS-level solution
```

### Issue: "DNS resolution failures"

**Symptoms:**
```
Task failed: Name or service not known
```

**Diagnostics:**

1. **Test DNS:**
```bash
nslookup api.example.com
dig api.example.com
host api.example.com
```

2. **Check DNS servers:**
```bash
cat /etc/resolv.conf
```

**Solutions:**

1. **Use IP address instead:**
```yaml
# If DNS is problematic
config:
  host: "192.168.1.100"  # Instead of hostname
```

2. **Add to /etc/hosts:**
```bash
echo "192.168.1.100 api.example.com" | sudo tee -a /etc/hosts
```

3. **Configure reliable DNS:**
```bash
# Use Google DNS or Cloudflare
echo "nameserver 8.8.8.8" | sudo tee /etc/resolv.conf
echo "nameserver 1.1.1.1" | sudo tee -a /etc/resolv.conf
```

---

## Log Analysis

### Finding Errors in Logs

**JSON logs (default):**

```bash
# Find all errors
grep '"level":"error"' logs/*/*.log

# Find specific workflow errors
grep '"workflow":"backup-workflow"' logs/*/*.log | grep error

# Parse with jq
cat picoflow.log | jq 'select(.level == "error")'
```

**Pretty logs:**

```bash
# Find ERROR lines
grep "ERROR" picoflow.log

# Find specific task
grep "task=backup_database" picoflow.log | grep ERROR
```

### Common Log Patterns

**Successful execution:**
```json
{"level":"info","workflow":"backup","task":"backup_db","status":"running"}
{"level":"info","workflow":"backup","task":"backup_db","status":"success","duration_ms":4523}
```

**Failed execution:**
```json
{"level":"error","workflow":"backup","task":"backup_db","status":"failed","error":"Connection timeout"}
```

**Retry attempts:**
```json
{"level":"warn","workflow":"backup","task":"backup_db","attempt":2,"error":"Temporary failure"}
{"level":"info","workflow":"backup","task":"backup_db","attempt":2,"status":"success"}
```

### Analyzing Performance from Logs

```bash
# Extract task durations
grep "duration_ms" picoflow.log | \
  jq -r '[.task, .duration_ms] | @csv' | \
  sort -t, -k2 -n -r

# Average duration by task
grep "duration_ms" picoflow.log | \
  jq -r '[.task, .duration_ms] | @tsv' | \
  awk '{sum[$1]+=$2; count[$1]++} END {for (task in sum) print task, sum[task]/count[task]}'
```

---

## Recovery Procedures

### Recovering from Workflow Failure

1. **Identify failed task:**
```bash
picoflow status --workflow myworkflow
picoflow logs --workflow myworkflow --task failed_task
```

2. **Fix the issue** (see error-specific solutions above)

3. **Resume workflow:**
```bash
# Currently: Re-run entire workflow
picoflow run workflow.yaml

# Future: Task-level resume (planned)
```

### Recovering from Daemon Crash

1. **Check crash reason:**
```bash
journalctl -u picoflow -n 100
dmesg | grep picoflow
```

2. **Fix underlying issue** (OOM, disk full, etc.)

3. **Clean up state:**
```bash
# Check for stale locks
rm -f picoflow.pid
rm -f picoflow.db-journal
```

4. **Restart daemon:**
```bash
systemctl start picoflow
# Or
picoflow daemon start workflows/*.yaml
```

5. **Verify workflows resumed:**
```bash
picoflow daemon status
picoflow workflow list
```

### Recovering from Disk Full

1. **Identify large files:**
```bash
du -h logs/ | sort -h | tail -20
ls -lhS logs/*/* | head -20
```

2. **Clean up logs:**
```bash
# Remove old logs
find logs/ -mtime +7 -delete

# Remove large logs
find logs/ -size +100M -delete
```

3. **Clean database:**
```bash
sqlite3 picoflow.db "
  DELETE FROM workflow_executions
  WHERE started_at < datetime('now', '-30 days');
  VACUUM;
"
```

4. **Add more storage** or move to larger partition:
```bash
# Move data directory
sudo mv /var/lib/picoflow /mnt/external/picoflow
sudo ln -s /mnt/external/picoflow /var/lib/picoflow
```

### Recovering from Corrupted Configuration

1. **Validate configuration:**
```bash
picoflow validate workflow.yaml
```

2. **Check YAML syntax:**
```bash
python3 -c "import yaml; yaml.safe_load(open('workflow.yaml'))"
```

3. **Restore from backup:**
```bash
# If you have version control
git checkout workflow.yaml

# Or from backup
cp workflow.yaml.backup workflow.yaml
```

4. **Rebuild from scratch** if necessary, using examples as reference

---

## Getting Help

### Before Asking for Help

1. **Check this troubleshooting guide**
2. **Search existing issues:** https://github.com/zoza1982/picoflow/issues
3. **Enable debug logging** and capture output
4. **Collect relevant information:**
   - PicoFlow version: `picoflow --version`
   - OS and architecture: `uname -a`
   - Workflow YAML (sanitized)
   - Error messages and logs
   - Steps to reproduce

### Reporting Bugs

Include in your bug report:

1. **Environment:**
   ```bash
   picoflow --version
   uname -a
   free -h
   ```

2. **Workflow (sanitized):**
   ```yaml
   # Remove any secrets/credentials
   ```

3. **Error output:**
   ```bash
   picoflow --log-level debug run workflow.yaml 2>&1 | tee debug.log
   ```

4. **Steps to reproduce:**
   - Step 1: ...
   - Step 2: ...
   - Expected: ...
   - Actual: ...

5. **File an issue:** https://github.com/zoza1982/picoflow/issues/new

### Community Support

- **GitHub Discussions:** https://github.com/zoza1982/picoflow/discussions
- **Documentation:** https://github.com/zoza1982/picoflow/tree/main/docs
- **Examples:** https://github.com/zoza1982/picoflow/tree/main/examples

---

**Document Version:** 1.0.0
**Last Updated:** November 12, 2025
**Feedback:** Report issues at https://github.com/zoza1982/picoflow/issues
