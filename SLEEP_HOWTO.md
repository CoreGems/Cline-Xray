# When Did I Go To Sleep? Detecting Sleep Patterns from Windows System Data

> **TL;DR**: I wanted to know when I went to sleep last night by checking my PC. The "obvious" answer — Windows Event Logs — completely failed on my always-on Windows 11 machine. The solution? Analyzing browser history, file timestamps, and user activity patterns with Python.

## The Question

Simple question: **"When did I go to sleep last night?"**

My PC is always on. I figured Windows must log when the system sleeps and wakes. Surely there's a PowerShell one-liner for this, right?

## Attempt #1: The "Textbook" Approach — Windows Event Logs

Every blog post and StackOverflow answer says the same thing: check the Windows Event Log for power events.

### The Standard Events Everyone Talks About

| Event ID | Provider | Meaning |
|----------|----------|---------|
| 42 | Kernel-Power | System entering sleep |
| 107 | Kernel-Power | System resumed from sleep |
| 1 | Power-Troubleshooter | Wake with source info (contains sleep/wake times) |
| 109 | Kernel-Power | Shutdown initiated |
| 12 | Kernel-General | OS startup |
| 13 | Kernel-General | OS shutdown |

### The PowerShell Script (That Didn't Work)

Here's what the "correct" approach looks like:

```powershell
# Check for sleep events
Get-WinEvent -FilterHashtable @{
    LogName      = 'System'
    ProviderName = 'Microsoft-Windows-Kernel-Power'
    Id           = 42
    StartTime    = (Get-Date).AddDays(-3)
} -ErrorAction SilentlyContinue

# Check for wake events  
Get-WinEvent -FilterHashtable @{
    LogName      = 'System'
    ProviderName = 'Microsoft-Windows-Kernel-Power'
    Id           = 107
    StartTime    = (Get-Date).AddDays(-3)
} -ErrorAction SilentlyContinue

# Check Power-Troubleshooter (has both sleep and wake times in message)
Get-WinEvent -FilterHashtable @{
    LogName      = 'System'
    ProviderName = 'Microsoft-Windows-Power-Troubleshooter'
    Id           = 1
    StartTime    = (Get-Date).AddDays(-3)
} -ErrorAction SilentlyContinue
```

**Result**: Zero events. Nothing. Nada.

### Why? The Diagnosis

```powershell
# Check system uptime
$os = Get-CimInstance Win32_OperatingSystem
Write-Host "Last Boot: $($os.LastBootUpTime)"
Write-Host "Uptime: $((Get-Date) - $os.LastBootUpTime)"
```

Output:
```
Last Boot: 01/09/2026 00:09:17
Uptime: 36.08:13:51
```

**My PC had been running for 36 days straight.** It never entered traditional S3 sleep. No Kernel-Power sleep events. No Power-Troubleshooter events. No wake history:

```powershell
powercfg /lastwake
# Output: Wake History Count - 0
```

### What About Modern Standby?

Windows 11 uses **Modern Standby** (formerly Connected Standby) on many newer machines. Unlike traditional S3 sleep:

- The CPU doesn't fully shut down
- Network stays partially connected
- The system can wake for background tasks
- **Traditional sleep/wake events (ID 42, 107) are NOT logged**

I tried every alternative event source:

| Source | Result |
|--------|--------|
| `Microsoft-Windows-Kernel-Power` | No events at all in 3 days |
| `Microsoft-Windows-Power-Troubleshooter` | No events |
| `Microsoft-Windows-SleepStudy/Diagnostic` | Empty |
| `Microsoft-Windows-Diagnostics-Performance/Operational` | No events |
| `Microsoft-Windows-UserModePowerService` | No events |
| `Microsoft-Windows-Winlogon` (lock/unlock) | No events |
| Security Log (Event 4800/4801 lock/unlock) | No lock/unlock audit events |
| `Microsoft-Windows-WLAN-AutoConfig` | No events (using Ethernet) |
| `powercfg /sleepstudy` | Requires admin |
| `powercfg /lastwake` | Wake History Count - 0 |

### The "Gap Analysis" Dead End

I tried analyzing gaps in System log events:

```powershell
$allEvents = Get-WinEvent -LogName System -MaxEvents 2000 |
    Where-Object { $_.TimeCreated -gt (Get-Date).AddDays(-3) } |
    Sort-Object TimeCreated

# Find gaps > 30 minutes
```

**Result**: 58 gaps found, but ALL were approximately 1 hour — just the regular interval between Windows scheduled maintenance tasks. The System log has constant background activity 24/7 that masks any real inactivity:

```
Gap: 02/13 06:39 AM -> 02/13 07:38 AM  (1 hours)
Gap: 02/13 01:39 PM -> 02/13 02:38 PM  (1 hours)
Gap: 02/14 01:39 AM -> 02/14 02:38 AM  (1 hours)  # <-- 2 AM, but still just 1 hour
Gap: 02/14 04:39 AM -> 02/14 05:38 AM  (1 hours)
... all ~1 hour, day and night
```

**The System event log is useless for detecting sleep on an always-on PC.**

## Attempt #2: The Solution — User Activity Analysis

If the system doesn't know when I sleep, I need to figure out when **I** was last active. The insight: **sleep = a gap in user-generated activity**.

### What Actually Shows User Activity on Windows?

| Data Source | What It Tells Us | Requires Admin? |
|-------------|-----------------|-----------------|
| Chrome/Edge browser history (SQLite) | Every page visit with timestamps | No |
| Windows Recent folder (`%APPDATA%\Microsoft\Windows\Recent`) | Files you opened | No |
| Downloads/Documents/Desktop file modifications | Files you saved | No |
| Prefetch files (`C:\Windows\Prefetch`) | When apps were launched | Yes |
| `GetLastInputInfo` Win32 API | Current idle time (keyboard/mouse) | No |

### The Key Insight: Browser History Is Gold

Chrome and Edge store their browsing history in SQLite databases with microsecond-precision timestamps. Every page visit, every tab switch — it's all there. This is the richest source of "when was the user active" data.

**Location**: `%LOCALAPPDATA%\Google\Chrome\User Data\{Profile}\History`

**Gotcha #1**: Chrome locks the database file while running. Solution: copy it to a temp file first.

**Gotcha #2**: Chrome timestamps are in **UTC microseconds since January 1, 1601** (the Windows FILETIME epoch). You need to convert:

```python
chrome_epoch = datetime(1601, 1, 1)
visit_time_utc = chrome_epoch + timedelta(microseconds=raw_timestamp)
# Then convert UTC to local time
visit_time_local = visit_time_utc + local_utc_offset
```

**Gotcha #3**: The `visits` table has a `url` column that is a foreign key to `urls.id`, but newer Chrome versions also have a `url` column in `urls`. A `JOIN` with `SELECT url` causes "ambiguous column name" errors. Use `SELECT v.visit_time FROM visits v` instead.

### The Temp Folder Trap

My first version also scanned `AppData\Local\Temp`. **Bad idea.** Background processes create temp files every hour on the hour, producing a constant 1-event-per-hour noise floor that completely masks real inactivity gaps. Remove it from the scan.

### The GetTickCount Overflow

Windows' `GetTickCount()` returns a `DWORD` (32-bit unsigned integer) representing milliseconds since boot. After ~49.7 days of uptime, it wraps around to 0. Since my PC had 36 days of uptime:

```python
# WRONG - can produce negative values after overflow
millis = GetTickCount() - lastInputInfo.dwTime

# CORRECT - handle 32-bit unsigned wraparound
millis = (GetTickCount() - lastInputInfo.dwTime) & 0xFFFFFFFF
```

## The Working Solution

### Full Python Script

```python
"""
Sleep Tracker - Determines when you went to sleep by analyzing user activity patterns.

Works on Windows systems that never fully sleep (Modern Standby / always-on PCs).
Uses multiple heuristics:
1. Win32 GetLastInputInfo (current idle time)
2. Recent files timestamps (Windows Recent folder)
3. Chrome/Edge browser history (SQLite databases)
4. User directory file changes (Downloads, Documents, Desktop)

Usage:
    python sleep_tracker.py          # default: look back 2 days
    python sleep_tracker.py --days 7 # look back 7 days
    python sleep_tracker.py --gap 2  # detect 2+ hour gaps (default: 3)
"""

import os
import sys
import time
import ctypes
import ctypes.wintypes
import sqlite3
import shutil
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from collections import defaultdict
import argparse

# Calculate local UTC offset for timezone conversions
_UTC_OFFSET = datetime.now() - datetime.utcnow()


class C:
    """ANSI color codes for terminal output."""
    CYAN = "\033[96m"
    GREEN = "\033[92m"
    MAGENTA = "\033[95m"
    YELLOW = "\033[93m"
    RED = "\033[91m"
    DIM = "\033[90m"
    BOLD = "\033[1m"
    RESET = "\033[0m"


def header(text):
    print(f"\n{C.CYAN}{'='*50}")
    print(f"  {text}")
    print(f"{'='*50}{C.RESET}\n")


def get_idle_time():
    """Get seconds since last keyboard/mouse input using Win32 API."""
    class LASTINPUTINFO(ctypes.Structure):
        _fields_ = [
            ('cbSize', ctypes.wintypes.UINT),
            ('dwTime', ctypes.wintypes.DWORD),
        ]

    lii = LASTINPUTINFO()
    lii.cbSize = ctypes.sizeof(LASTINPUTINFO)
    ctypes.windll.user32.GetLastInputInfo(ctypes.byref(lii))

    tick = ctypes.windll.kernel32.GetTickCount()
    millis = (tick - lii.dwTime) & 0xFFFFFFFF  # Handle DWORD wraparound
    return millis / 1000.0


def get_recent_files_activity(days=2):
    """Check Windows Recent folder for file access patterns."""
    recent_dir = Path(os.environ["APPDATA"]) / "Microsoft" / "Windows" / "Recent"
    cutoff = datetime.now() - timedelta(days=days)

    timestamps = []
    if recent_dir.exists():
        for f in recent_dir.iterdir():
            try:
                mtime = datetime.fromtimestamp(f.stat().st_mtime)
                if mtime > cutoff:
                    timestamps.append(mtime)
            except (OSError, PermissionError):
                pass

    return sorted(timestamps)


def get_chrome_history(days=2):
    """Read Chrome/Edge browser history for visit timestamps."""
    timestamps = []

    chrome_base = Path(os.environ["LOCALAPPDATA"]) / "Google"
    chrome_dirs = []

    for variant in ["Chrome", "Chrome Canary", "Chrome Beta", "Chrome Dev"]:
        user_data = chrome_base / variant / "User Data"
        if user_data.exists():
            history_file = user_data / "Default" / "History"
            if history_file.exists():
                chrome_dirs.append((variant + " (Default)", history_file))
            for p in user_data.iterdir():
                if p.name.startswith("Profile "):
                    hf = p / "History"
                    if hf.exists():
                        chrome_dirs.append((f"{variant} ({p.name})", hf))

    # Also check Edge (same Chromium SQLite format)
    edge_data = Path(os.environ["LOCALAPPDATA"]) / "Microsoft" / "Edge" / "User Data"
    if edge_data.exists():
        edge_hist = edge_data / "Default" / "History"
        if edge_hist.exists():
            chrome_dirs.append(("Edge (Default)", edge_hist))

    cutoff = datetime.now() - timedelta(days=days)
    chrome_epoch = datetime(1601, 1, 1)
    cutoff_utc = cutoff - _UTC_OFFSET
    cutoff_chrome = int((cutoff_utc - chrome_epoch).total_seconds() * 1_000_000)

    for name, history_file in chrome_dirs:
        try:
            tmp = tempfile.mktemp(suffix=".db")
            shutil.copy2(str(history_file), tmp)

            conn = sqlite3.connect(tmp)
            cursor = conn.cursor()
            cursor.execute(
                "SELECT v.visit_time FROM visits v "
                "WHERE v.visit_time > ? ORDER BY v.visit_time",
                (cutoff_chrome,)
            )

            for row in cursor.fetchall():
                visit_time_utc = chrome_epoch + timedelta(microseconds=row[0])
                visit_time = visit_time_utc + _UTC_OFFSET  # Convert to local
                timestamps.append(visit_time)

            conn.close()
            os.unlink(tmp)
        except Exception as e:
            print(f"  {C.DIM}Could not read {name}: {e}{C.RESET}")

    return sorted(timestamps)


def get_user_dir_activity(days=2):
    """Check key user directories for recent file modifications."""
    cutoff = datetime.now() - timedelta(days=days)
    home = Path(os.environ["USERPROFILE"])

    timestamps = []
    # NOTE: Do NOT include AppData/Local/Temp — background processes create
    # files there every hour, which masks real inactivity gaps
    for d in [home / "Downloads", home / "Documents", home / "Desktop"]:
        if d.exists():
            try:
                for f in d.iterdir():
                    try:
                        mtime = datetime.fromtimestamp(f.stat().st_mtime)
                        if mtime > cutoff:
                            timestamps.append(mtime)
                    except (OSError, PermissionError):
                        pass
            except PermissionError:
                pass

    return sorted(timestamps)


def find_sleep_periods(all_timestamps, min_gap_hours=3):
    """Find gaps in activity that likely represent sleep or away periods."""
    if len(all_timestamps) < 2:
        return []

    periods = []
    for i in range(1, len(all_timestamps)):
        gap = all_timestamps[i] - all_timestamps[i-1]
        gap_hours = gap.total_seconds() / 3600

        if gap_hours >= min_gap_hours:
            sleep_start = all_timestamps[i-1]
            wake_time = all_timestamps[i]
            start_hour = sleep_start.hour
            is_night = start_hour >= 20 or start_hour < 4

            periods.append({
                'sleep_at': sleep_start,
                'wake_at': wake_time,
                'duration_hours': gap_hours,
                'is_likely_sleep': is_night,
            })

    return periods


def print_heatmap(all_timestamps, days=2):
    """Print an hourly activity heatmap."""
    if not all_timestamps:
        return

    hourly = defaultdict(int)
    for ts in all_timestamps:
        hourly[ts.strftime("%m/%d %H:00")] += 1

    now = datetime.now()
    current = (now - timedelta(days=days)).replace(minute=0, second=0, microsecond=0)
    max_count = max(hourly.values()) if hourly else 1

    print(f"  {'Hour':<14} {'Count':>5}  Activity")
    print(f"  {'─'*14} {'─'*5}  {'─'*40}")

    prev_date = None
    while current <= now:
        key = current.strftime("%m/%d %H:00")
        count = hourly.get(key, 0)
        bar_len = int((count / max_count) * 40) if max_count > 0 else 0
        bar = "█" * bar_len
        hour = current.hour
        is_night = hour >= 22 or hour < 7
        color = C.MAGENTA if is_night else C.GREEN
        date_str = current.strftime("%m/%d")
        if date_str != prev_date:
            if prev_date:
                print()
            prev_date = date_str
        if count == 0:
            color = C.DIM
        print(f"  {color}{key:<14} {count:>5}  {bar}{C.RESET}")
        current += timedelta(hours=1)


def main():
    parser = argparse.ArgumentParser(
        description="Detect when you went to sleep based on PC activity"
    )
    parser.add_argument("--days", type=int, default=2,
                        help="Days to look back (default: 2)")
    parser.add_argument("--gap", type=float, default=3.0,
                        help="Min gap hours to count as sleep (default: 3)")
    args = parser.parse_args()

    header("Sleep/Wake Tracker")
    print(f"  Looking back {args.days} day(s), min gap: {args.gap}h")
    print(f"  Current time: {datetime.now().strftime('%A, %I:%M %p')}")

    # Current idle
    idle_secs = get_idle_time()
    idle_mins = idle_secs / 60
    print(f"\n  {C.CYAN}Current idle time:{C.RESET} ", end="")
    if idle_mins < 1:
        print(f"{C.GREEN}{idle_secs:.0f} seconds{C.RESET}")
    elif idle_mins < 60:
        print(f"{C.YELLOW}{idle_mins:.1f} minutes{C.RESET}")
    else:
        print(f"{C.RED}{idle_mins/60:.1f} hours{C.RESET}")

    # Gather all activity
    print(f"\n  {C.DIM}Gathering activity data...{C.RESET}")
    all_timestamps = []

    for label, func in [
        ("Recent files", get_recent_files_activity),
        ("Browser history", get_chrome_history),
        ("User directory changes", get_user_dir_activity),
    ]:
        data = func(args.days)
        print(f"  {C.DIM}{label}: {len(data)} events{C.RESET}")
        all_timestamps.extend(data)

    if not all_timestamps:
        print(f"\n  {C.RED}No activity data found!{C.RESET}")
        return

    all_timestamps.sort()

    # De-duplicate within 1 minute
    deduped = [all_timestamps[0]]
    for ts in all_timestamps[1:]:
        if (ts - deduped[-1]).total_seconds() > 60:
            deduped.append(ts)
    all_timestamps = deduped

    print(f"\n  {C.GREEN}Total unique events: {len(all_timestamps)}{C.RESET}")
    print(f"  Range: {all_timestamps[0].strftime('%m/%d %I:%M %p')} → "
          f"{all_timestamps[-1].strftime('%m/%d %I:%M %p')}")

    header("Activity Heatmap (hourly)")
    print_heatmap(all_timestamps, args.days)

    header("Detected Sleep/Away Periods")
    periods = find_sleep_periods(all_timestamps, min_gap_hours=args.gap)

    if not periods:
        print(f"  {C.YELLOW}No gaps >= {args.gap} hours found.{C.RESET}")
    else:
        for p in periods:
            icon = "💤" if p['is_likely_sleep'] else "🚶"
            label = "SLEEP" if p['is_likely_sleep'] else "AWAY"
            color = C.MAGENTA if p['is_likely_sleep'] else C.YELLOW
            hours = int(p['duration_hours'])
            mins = int((p['duration_hours'] - hours) * 60)
            print(f"  {icon} {color}[{label}]{C.RESET} "
                  f"{p['sleep_at'].strftime('%A %m/%d %I:%M %p')} → "
                  f"{p['wake_at'].strftime('%I:%M %p')}"
                  f"  {C.CYAN}({hours}h {mins}m){C.RESET}")

    header("Last Night Summary")
    night_periods = [p for p in periods if p['is_likely_sleep']]

    if night_periods:
        last = night_periods[-1]
        hours = int(last['duration_hours'])
        mins = int((last['duration_hours'] - hours) * 60)
        print(f"  You stopped using your PC at:  "
              f"{C.MAGENTA}{last['sleep_at'].strftime('%A, %I:%M:%S %p')}{C.RESET}")
        print(f"  You started using it again at: "
              f"{C.GREEN}{last['wake_at'].strftime('%A, %I:%M:%S %p')}{C.RESET}")
        print(f"  PC was idle for:               "
              f"{C.CYAN}{hours}h {mins}m{C.RESET}")
    else:
        print(f"  {C.YELLOW}Could not determine sleep pattern.{C.RESET}")
    print()


if __name__ == "__main__":
    main()
```

## The Results

Running the script on my machine (Windows 11, 36-day uptime, Modern Standby):

```
==================================================
  Sleep/Wake Tracker
==================================================

  Looking back 2 day(s), min gap: 3.0h
  Current time: Saturday, 08:33 AM
  Current idle time: 37 seconds

  Gathering activity data...
  Recent files: 18 events
  Browser history: 106 events
  User directory changes: 12 events

  Total unique activity events: 25
  Range: 02/12 09:24 AM → 02/14 08:26 AM

==================================================
  Activity Heatmap (hourly)
==================================================

  Hour           Count  Activity
  ────────────── ─────  ────────────────────────────────────────
  02/12 09:00        2  ████████████████
  02/12 13:00        2  ████████████████
  02/12 16:00        1  ████████
  02/12 17:00        1  ████████
  02/12 19:00        2  ████████████████
  02/12 20:00        3  ████████████████████████
  (midnight - dead silence)
  02/13 09:00        1  ████████
  02/13 10:00        2  ████████████████
  02/13 21:00        1  ████████
  (midnight - dead silence again)
  02/14 05:00        1  ████████
  02/14 07:00        4  ████████████████████████████████
  02/14 08:00        5  ████████████████████████████████████████

==================================================
  Detected Sleep/Away Periods
==================================================

  💤 [SLEEP] Thursday 02/12 08:29 PM → 09:25 AM  (12h 56m)
  💤 [SLEEP] Friday 02/13 09:59 PM → 05:46 AM   (7h 47m)

==================================================
  Last Night Summary
==================================================

  You stopped using your PC at:  Friday, 09:59:33 PM
  You started using it again at: Saturday, 05:46:37 AM
  PC was idle for:               7h 47m
```

**Answer: I went to sleep around 10 PM and woke up at 5:46 AM. 7 hours 47 minutes.**

## Lessons Learned

### 1. Modern Standby Broke All the Old Tricks
Every tutorial about Windows sleep detection assumes traditional S3 sleep. If your PC uses Modern Standby (most Windows 11 laptops and desktops with newer chipsets), none of the standard Kernel-Power events exist. `powercfg /lastwake` returns nothing. The system never truly "sleeps" from the OS perspective.

### 2. Background System Events Create a Noise Floor
The Windows System event log has scheduled tasks running every hour, 24/7. This creates uniform 1-hour gaps that look identical at 2 PM and 2 AM. You cannot distinguish sleep from wakefulness by looking at system event gaps alone.

### 3. The Temp Folder Is a Trap
`AppData\Local\Temp` gets files created by background processes (Windows Update, antivirus, telemetry) every hour. Including it in your scan masks all real inactivity gaps.

### 4. Browser History Is the Best Signal
Chrome's SQLite database has microsecond-precision timestamps for every page visit. It's the richest source of user activity data that doesn't require admin privileges. But watch out for:
- **File locking**: Chrome keeps the DB locked while running → copy to temp first
- **UTC timestamps**: Chrome stores everything in UTC microseconds since 1601-01-01
- **Schema changes**: Modern Chrome has ambiguous column names if you JOIN visits+urls naively

### 5. GetTickCount Overflows After 49.7 Days
If your PC has been up for more than ~49.7 days, `GetTickCount()` wraps around. The subtraction for idle time calculation can produce garbage values unless you mask with `& 0xFFFFFFFF`.

### 6. Multiple Data Sources > Single Source
No single source gives a complete picture:
- **Browser history** misses offline activity
- **Recent files** misses browser-only sessions
- **File timestamps** miss read-only browsing
- Combining all three gives a reliable activity timeline

## Requirements

- Python 3.6+ (uses f-strings)
- Windows 10/11
- No admin required
- No pip packages needed (all stdlib: `ctypes`, `sqlite3`, `pathlib`, `argparse`)
- Works with Chrome, Chrome Canary, Chrome Dev, Chrome Beta, and Edge

## Usage

```bash
# Basic usage - last 2 days, 3-hour gap threshold
python sleep_tracker.py

# Look back a full week
python sleep_tracker.py --days 7

# Detect shorter absences (2+ hours)
python sleep_tracker.py --gap 2
```
