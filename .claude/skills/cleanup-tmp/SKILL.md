---
name: cleanup-tmp
description: Final cleanup step for close-out-thread. Removes all artifacts from current session (all /tmp/rustdv-<uid>* directories). Reports only freed space and current usage.
---

# Clean up /tmp session artifacts

Final step. Removes all `/tmp/rustdv-$(id -u)*` directories (main session plus
any suffixed subdirs like `-framework`, `-target`).

**Output only: space freed and final usage.**

```bash
BEFORE=$(df /tmp | awk 'NR==2 {print $4}')
rm -rf /tmp/rustdv-$(id -u)/ /tmp/rustdv-$(id -u)-* 2>/dev/null || true
AFTER=$(df /tmp | awk 'NR==2 {print $4}')
FREED=$(( (AFTER - BEFORE) / 1024 ))
echo "Cleaned. Freed: ${FREED}MB | /tmp: $(df -h /tmp | awk 'NR==2 {print $5}') used"
```
