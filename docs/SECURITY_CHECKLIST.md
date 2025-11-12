# PicoFlow Security Checklist

Quick reference security checklist for developers and operators.

---

## Development Checklist

### When Adding New Executors

- [ ] **Input Validation:** Validate all user inputs (URLs, paths, commands, arguments)
- [ ] **Output Limits:** Enforce maximum output size (10MB default)
- [ ] **Timeout Enforcement:** All I/O operations must have timeouts
- [ ] **Error Handling:** Never leak sensitive information in error messages
- [ ] **Authentication:** Use key-based authentication (never passwords)
- [ ] **Injection Prevention:** Use parameterized APIs (command+args, not shell strings)

### When Modifying Parsers/Validators

- [ ] **Size Limits:** Enforce maximum input sizes (YAML: 1MB, commands: 4KB)
- [ ] **Character Validation:** Whitelist allowed characters for names/identifiers
- [ ] **Path Validation:** Check for path traversal (block `..`, require absolute paths)
- [ ] **Count Limits:** Enforce maximum counts (tasks: 1000, args: 256)
- [ ] **DoS Prevention:** Prevent resource exhaustion via deeply nested structures

### When Writing Example Workflows

- [ ] **No Hardcoded Secrets:** Use `${ENVIRONMENT_VARIABLE}` for all secrets
- [ ] **Secure Defaults:** Never use insecure options (e.g., `StrictHostKeyChecking=no`)
- [ ] **Documentation:** Document all required environment variables
- [ ] **Security Notes:** Include security best practices in comments
- [ ] **File Permissions:** Document required permissions (SSH keys: 600)

---

## Deployment Checklist

### Pre-Deployment

- [ ] **System User:** Create dedicated `picoflow` user (no shell, no login)
  ```bash
  useradd -r -s /bin/false picoflow
  ```

- [ ] **File Permissions:**
  ```bash
  # Database (owner read/write only)
  chmod 600 /var/lib/picoflow/picoflow.db
  chown picoflow:picoflow /var/lib/picoflow/picoflow.db

  # SSH keys (owner read only)
  chmod 600 ~/.ssh/id_rsa
  chmod 644 ~/.ssh/id_rsa.pub

  # Workflow configs (owner read/write, group read)
  chmod 640 /etc/picoflow/workflows/*.yaml
  chown picoflow:picoflow /etc/picoflow/workflows/

  # PID file directory
  chmod 755 /var/run/picoflow/
  chown picoflow:picoflow /var/run/picoflow/
  ```

- [ ] **SSH Configuration:**
  ```bash
  # Add all remote hosts to known_hosts
  ssh-keyscan -H hostname1.example.com >> ~/.ssh/known_hosts
  ssh-keyscan -H hostname2.example.com >> ~/.ssh/known_hosts

  # Set proper permissions
  chmod 644 ~/.ssh/known_hosts
  ```

- [ ] **Environment Variables:** Set all required secrets in environment
  ```bash
  # Use systemd EnvironmentFile or similar
  export DB_PASSWORD="..."
  export API_TOKEN="..."
  ```

- [ ] **Network Security:**
  - [ ] Configure firewall rules (allow only necessary outbound connections)
  - [ ] Consider network namespace isolation
  - [ ] Use HTTPS for all HTTP executor calls

### systemd Hardening (Recommended)

Add to `/etc/systemd/system/picoflow.service`:

```ini
[Service]
# Run as dedicated user
User=picoflow
Group=picoflow

# Security hardening
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
NoNewPrivileges=true
ReadWritePaths=/var/lib/picoflow /var/run/picoflow

# Resource limits
MemoryMax=512M
TasksMax=100
LimitNOFILE=1024

# Restart policy
Restart=on-failure
RestartSec=10s
```

---

## Secrets Management Checklist

### SSH Keys

- [ ] Generate dedicated deployment keys (not personal keys)
  ```bash
  ssh-keygen -t ed25519 -f ~/.ssh/picoflow_deploy -C "picoflow@hostname"
  ```

- [ ] Set restrictive permissions
  ```bash
  chmod 600 ~/.ssh/picoflow_deploy
  chmod 644 ~/.ssh/picoflow_deploy.pub
  ```

- [ ] Add public key to target hosts
  ```bash
  ssh-copy-id -i ~/.ssh/picoflow_deploy.pub user@target-host
  ```

- [ ] Rotate keys regularly (quarterly recommended)

### API Tokens / Passwords

- [ ] Store in environment variables (never in YAML files)
- [ ] Use systemd `EnvironmentFile` for production
  ```ini
  [Service]
  EnvironmentFile=/etc/picoflow/secrets.env
  ```

- [ ] Set proper permissions on secrets file
  ```bash
  chmod 600 /etc/picoflow/secrets.env
  chown picoflow:picoflow /etc/picoflow/secrets.env
  ```

- [ ] Rotate credentials regularly

### Future: Secrets Manager Integration

For production deployments, consider:
- HashiCorp Vault
- AWS Secrets Manager
- Azure Key Vault
- GCP Secret Manager

---

## Workflow Security Checklist

### Before Running a Workflow

- [ ] **Review YAML:** No hardcoded secrets
- [ ] **Validate Paths:** All paths are absolute and trusted
- [ ] **Check Commands:** All commands use absolute paths
- [ ] **Environment Variables:** All required variables are set
- [ ] **SSH Hosts:** All hosts are in `~/.ssh/known_hosts`
- [ ] **Test First:** Run in test environment before production

### Workflow Best Practices

#### ✅ DO

```yaml
# Use environment variables for secrets
config:
  host: "${DB_HOST}"
headers:
  Authorization: "Bearer ${API_TOKEN}"

# Use absolute paths for commands
tasks:
  - name: backup
    type: shell
    config:
      command: "/usr/bin/tar"
      args: ["-czf", "/backup/data.tar.gz", "/data"]

# Set appropriate timeouts
timeout: 300  # 5 minutes

# Use retry logic for flaky operations
retry: 3
```

#### ❌ DON'T

```yaml
# DON'T: Hardcode secrets
headers:
  Authorization: "Bearer sk-1234567890abcdef"

# DON'T: Use relative paths
config:
  command: "tar"  # Could execute malicious binary in PATH

# DON'T: Disable security features
args:
  - "-o"
  - "StrictHostKeyChecking=no"  # Vulnerable to MITM

# DON'T: Use shell string interpolation
config:
  command: "/bin/sh"
  args: ["-c", "rm -rf ${USER_INPUT}"]  # Command injection risk
```

---

## Monitoring Checklist

### Security Events to Monitor

- [ ] **Failed Authentication Attempts**
  ```bash
  # Monitor logs for SSH auth failures
  journalctl -u picoflow | grep "Authentication failed"
  ```

- [ ] **Unusual Network Activity**
  - Connections to unexpected hosts
  - Large data transfers
  - Requests to metadata services (169.254.169.254)

- [ ] **Failed Workflow Executions**
  ```bash
  # Check for repeated failures
  picoflow history workflow-name --status failed --limit 10
  ```

- [ ] **Resource Exhaustion**
  - Memory usage approaching limits
  - Disk space on database partition
  - High task failure rates

### Audit Logging

Enable audit logging for:
- All executor actions (shell, SSH, HTTP)
- Authentication events
- Workflow start/stop
- Configuration changes

Example configuration (future feature):
```yaml
logging:
  audit:
    enabled: true
    file: /var/log/picoflow/audit.log
    events:
      - executor_command
      - ssh_connection
      - http_request
      - auth_failure
```

---

## Incident Response Checklist

### If You Suspect a Compromise

1. **Immediate Actions:**
   - [ ] Stop the daemon: `systemctl stop picoflow`
   - [ ] Isolate the system from network
   - [ ] Preserve logs: `cp -r /var/log/picoflow /forensics/`

2. **Investigation:**
   - [ ] Review workflow execution history
   - [ ] Check for unauthorized workflows
   - [ ] Review database for suspicious activity
   - [ ] Check for modified files: `debsums -c` (Debian) or `rpm -Va` (RHEL)

3. **Recovery:**
   - [ ] Rotate all SSH keys
   - [ ] Rotate all API tokens/credentials
   - [ ] Review and update all workflow configurations
   - [ ] Update PicoFlow to latest version
   - [ ] Re-deploy from clean backup

4. **Post-Incident:**
   - [ ] Document findings
   - [ ] Update security controls
   - [ ] Review access logs
   - [ ] Conduct security training

---

## Regular Maintenance Checklist

### Weekly

- [ ] Review failed workflow executions
- [ ] Check disk space on database partition
- [ ] Monitor memory usage trends
- [ ] Review security logs for anomalies

### Monthly

- [ ] Run dependency audit: `cargo audit`
- [ ] Review and rotate credentials
- [ ] Test backup/restore procedures
- [ ] Review user access and permissions
- [ ] Update documentation

### Quarterly

- [ ] Rotate SSH keys
- [ ] Conduct security review
- [ ] Update dependencies: `cargo update`
- [ ] Review and update workflows
- [ ] Penetration testing (if applicable)

### Annually

- [ ] Full security audit
- [ ] Disaster recovery drill
- [ ] Review security policies
- [ ] Update threat model
- [ ] Training/awareness update

---

## Security Testing Checklist

### Before Each Release

- [ ] **Dependency Audit:** `cargo audit` passes with no vulnerabilities
- [ ] **Static Analysis:** `cargo clippy -- -D warnings` passes
- [ ] **Security Tests:** All security-focused tests pass
- [ ] **Example Review:** All examples follow security best practices
- [ ] **Documentation:** Security documentation is up-to-date

### Security Test Cases

```bash
# Command injection tests
cargo test test_command_injection

# Path traversal tests
cargo test test_path_traversal

# Input validation tests
cargo test test_input_limits

# SSRF tests (after HTTP-01 fix)
cargo test test_ssrf_prevention

# Integration tests with actual SSH/HTTP
cargo test --test '*' --features integration
```

---

## Quick Reference: Common Security Mistakes

### Mistake 1: Using Relative Paths
```yaml
# ❌ WRONG
command: "ls"  # Could execute malicious binary

# ✅ CORRECT
command: "/usr/bin/ls"
```

### Mistake 2: Hardcoding Secrets
```yaml
# ❌ WRONG
headers:
  Authorization: "Bearer my-secret-token"

# ✅ CORRECT
headers:
  Authorization: "Bearer ${API_TOKEN}"
```

### Mistake 3: Disabling SSH Host Key Checking
```bash
# ❌ WRONG
scp -o StrictHostKeyChecking=no file.txt host:/path/

# ✅ CORRECT
# First: ssh-keyscan -H hostname >> ~/.ssh/known_hosts
scp file.txt host:/path/
```

### Mistake 4: Excessive Permissions
```bash
# ❌ WRONG
chmod 777 picoflow.db
chmod 644 ~/.ssh/id_rsa

# ✅ CORRECT
chmod 600 picoflow.db
chmod 600 ~/.ssh/id_rsa
```

### Mistake 5: No Timeout on Long Operations
```yaml
# ❌ WRONG (could hang forever)
timeout: 0

# ✅ CORRECT
timeout: 300  # 5 minutes
```

---

## Emergency Contacts

**Security Issues:**
- Report via: [GitHub Security Advisory](https://github.com/yourusername/picoflow/security/advisories)
- Email: security@yourdomain.com

**Critical Bugs:**
- GitHub Issues: https://github.com/yourusername/picoflow/issues

---

## Additional Resources

- **Full Security Audit:** See `SECURITY_AUDIT_REPORT.md`
- **Applied Fixes:** See `SECURITY_FIXES_SUMMARY.md`
- **Best Practices:** See `docs/SECURITY.md` (to be created)
- **Production Guide:** See `docs/PRODUCTION_DEPLOYMENT.md` (to be created)

---

**Last Updated:** November 12, 2025
**Document Version:** 1.0
