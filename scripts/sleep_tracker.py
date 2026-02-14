"""
Sleep Tracker - Determines when you went to sleep by analyzing user activity patterns.

Works on Windows systems that never fully sleep (Modern Standby / always-on PCs).
Uses multiple heuristics:
1. Win32 GetLastInputInfo (current idle time)
2. Recent files timestamps (Windows Recent folder)
3. Chrome browser history (last visited pages)
4. Prefetch file timestamps (last app launches)

Usage:
    python sleep_tracker.py          # default: look back 2 days
    python sleep_tracker.py --days 7 # look back 7 days
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


# ─── Colors for terminal output ───────────────────────────────────
class C:
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


# ─── 1. Current idle time via Win32 API ──────────────────────────
def get_idle_time():
    """Get seconds since last keyboard/mouse input."""
    class LASTINPUTINFO(ctypes.Structure):
        _fields_ = [
            ('cbSize', ctypes.wintypes.UINT),
            ('dwTime', ctypes.wintypes.DWORD),
        ]
    
    lii = LASTINPUTINFO()
    lii.cbSize = ctypes.sizeof(LASTINPUTINFO)
    ctypes.windll.user32.GetLastInputInfo(ctypes.byref(lii))
    
    tick = ctypes.windll.kernel32.GetTickCount()
    # Handle DWORD wraparound (32-bit unsigned)
    millis = (tick - lii.dwTime) & 0xFFFFFFFF
    return millis / 1000.0


# ─── 2. Recent files analysis ────────────────────────────────────
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


# ─── 3. Chrome browser history ───────────────────────────────────
def get_chrome_history(days=2):
    """Read Chrome browser history for visit timestamps."""
    timestamps = []
    
    # Check multiple Chrome profiles
    chrome_base = Path(os.environ["LOCALAPPDATA"]) / "Google"
    chrome_dirs = []
    
    for variant in ["Chrome", "Chrome Canary", "Chrome Beta", "Chrome Dev"]:
        user_data = chrome_base / variant / "User Data"
        if user_data.exists():
            # Default profile
            history_file = user_data / "Default" / "History"
            if history_file.exists():
                chrome_dirs.append((variant + " (Default)", history_file))
            # Numbered profiles
            for p in user_data.iterdir():
                if p.name.startswith("Profile "):
                    hf = p / "History"
                    if hf.exists():
                        chrome_dirs.append((f"{variant} ({p.name})", hf))
    
    # Also check Edge
    edge_data = Path(os.environ["LOCALAPPDATA"]) / "Microsoft" / "Edge" / "User Data"
    if edge_data.exists():
        edge_hist = edge_data / "Default" / "History"
        if edge_hist.exists():
            chrome_dirs.append(("Edge (Default)", edge_hist))
    
    cutoff = datetime.now() - timedelta(days=days)
    # Chrome epoch: Jan 1, 1601 UTC (microseconds)
    chrome_epoch = datetime(1601, 1, 1)
    # Chrome stores UTC, so adjust cutoff to UTC for the query
    cutoff_utc = cutoff - _UTC_OFFSET
    cutoff_chrome = int((cutoff_utc - chrome_epoch).total_seconds() * 1_000_000)
    
    for name, history_file in chrome_dirs:
        try:
            # Copy to temp file since Chrome locks the DB
            tmp = tempfile.mktemp(suffix=".db")
            shutil.copy2(str(history_file), tmp)
            
            conn = sqlite3.connect(tmp)
            cursor = conn.cursor()
            cursor.execute(
                "SELECT v.visit_time FROM visits v "
                "WHERE v.visit_time > ? "
                "ORDER BY v.visit_time",
                (cutoff_chrome,)
            )
            
            for row in cursor.fetchall():
                # Convert from Chrome UTC to local time
                visit_time_utc = chrome_epoch + timedelta(microseconds=row[0])
                visit_time = visit_time_utc + _UTC_OFFSET
                timestamps.append(visit_time)
            
            conn.close()
            os.unlink(tmp)
        except Exception as e:
            print(f"  {C.DIM}Could not read {name}: {e}{C.RESET}")
    
    return sorted(timestamps)


# ─── 4. Prefetch files ───────────────────────────────────────────
def get_prefetch_activity(days=2):
    """Check Prefetch folder for application launch timestamps."""
    prefetch_dir = Path("C:/Windows/Prefetch")
    cutoff = datetime.now() - timedelta(days=days)
    
    timestamps = []
    if prefetch_dir.exists():
        try:
            for f in prefetch_dir.iterdir():
                try:
                    mtime = datetime.fromtimestamp(f.stat().st_mtime)
                    if mtime > cutoff:
                        timestamps.append(mtime)
                except (OSError, PermissionError):
                    pass
        except PermissionError:
            pass
    
    return sorted(timestamps)


# ─── 5. User profile directory changes ───────────────────────────
def get_user_dir_activity(days=2):
    """Check key user directories for recent file modifications."""
    cutoff = datetime.now() - timedelta(days=days)
    home = Path(os.environ["USERPROFILE"])
    
    timestamps = []
    check_dirs = [
        home / "Downloads",
        home / "Documents",
        home / "Desktop",
    ]
    
    for d in check_dirs:
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


# ─── Analysis: Find sleep/wake periods ───────────────────────────
def find_sleep_periods(all_timestamps, min_gap_hours=3):
    """
    Given a sorted list of activity timestamps, find gaps that likely represent sleep.
    A gap > min_gap_hours during nighttime (10 PM - 10 AM) is likely sleep.
    """
    if len(all_timestamps) < 2:
        return []
    
    periods = []
    for i in range(1, len(all_timestamps)):
        gap = all_timestamps[i] - all_timestamps[i-1]
        gap_hours = gap.total_seconds() / 3600
        
        if gap_hours >= min_gap_hours:
            sleep_start = all_timestamps[i-1]
            wake_time = all_timestamps[i]
            
            # Classify: nighttime sleep vs daytime away
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
        key = ts.strftime("%m/%d %H:00")
        hourly[key] += 1
    
    # Generate all hours in range
    now = datetime.now()
    start = now - timedelta(days=days)
    current = start.replace(minute=0, second=0, microsecond=0)
    
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
        
        # Add date separator
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
    parser = argparse.ArgumentParser(description="Detect when you went to sleep based on PC activity")
    parser.add_argument("--days", type=int, default=2, help="Days to look back (default: 2)")
    parser.add_argument("--gap", type=float, default=3.0, help="Min gap hours to count as sleep (default: 3)")
    args = parser.parse_args()
    
    header("Sleep/Wake Tracker")
    print(f"  Looking back {args.days} day(s), min gap: {args.gap}h")
    print(f"  Current time: {datetime.now().strftime('%A, %I:%M %p')}")
    
    # Current idle time
    idle_secs = get_idle_time()
    idle_mins = idle_secs / 60
    print(f"\n  {C.CYAN}Current idle time:{C.RESET} ", end="")
    if idle_mins < 1:
        print(f"{C.GREEN}{idle_secs:.0f} seconds{C.RESET}")
    elif idle_mins < 60:
        print(f"{C.YELLOW}{idle_mins:.1f} minutes{C.RESET}")
    else:
        print(f"{C.RED}{idle_mins/60:.1f} hours{C.RESET}")
    
    # Gather all activity timestamps
    print(f"\n  {C.DIM}Gathering activity data...{C.RESET}")
    
    all_timestamps = []
    
    # Recent files
    recent = get_recent_files_activity(args.days)
    print(f"  {C.DIM}Recent files: {len(recent)} events{C.RESET}")
    all_timestamps.extend(recent)
    
    # Chrome history
    chrome = get_chrome_history(args.days)
    print(f"  {C.DIM}Browser history: {len(chrome)} events{C.RESET}")
    all_timestamps.extend(chrome)
    
    # Prefetch
    prefetch = get_prefetch_activity(args.days)
    print(f"  {C.DIM}Prefetch (app launches): {len(prefetch)} events{C.RESET}")
    all_timestamps.extend(prefetch)
    
    # User dirs
    user_files = get_user_dir_activity(args.days)
    print(f"  {C.DIM}User directory changes: {len(user_files)} events{C.RESET}")
    all_timestamps.extend(user_files)
    
    if not all_timestamps:
        print(f"\n  {C.RED}No activity data found!{C.RESET}")
        return
    
    all_timestamps.sort()
    
    # De-duplicate timestamps within 1 minute
    deduped = [all_timestamps[0]]
    for ts in all_timestamps[1:]:
        if (ts - deduped[-1]).total_seconds() > 60:
            deduped.append(ts)
    all_timestamps = deduped
    
    print(f"\n  {C.GREEN}Total unique activity events: {len(all_timestamps)}{C.RESET}")
    print(f"  Range: {all_timestamps[0].strftime('%m/%d %I:%M %p')} → {all_timestamps[-1].strftime('%m/%d %I:%M %p')}")
    
    # Activity heatmap
    header("Activity Heatmap (hourly)")
    print_heatmap(all_timestamps, args.days)
    
    # Find sleep periods
    header("Detected Sleep/Away Periods")
    periods = find_sleep_periods(all_timestamps, min_gap_hours=args.gap)
    
    if not periods:
        print(f"  {C.YELLOW}No gaps >= {args.gap} hours found.{C.RESET}")
        print(f"  {C.DIM}Try lowering --gap (e.g., --gap 2){C.RESET}")
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
    
    # Last night summary
    header("Last Night Summary")
    
    # Find the most recent sleep period (prefer nighttime ones)
    night_periods = [p for p in periods if p['is_likely_sleep']]
    
    if night_periods:
        last_sleep = night_periods[-1]
        hours = int(last_sleep['duration_hours'])
        mins = int((last_sleep['duration_hours'] - hours) * 60)
        
        print(f"  You stopped using your PC at:  {C.MAGENTA}{last_sleep['sleep_at'].strftime('%A, %I:%M:%S %p')}{C.RESET}")
        print(f"  You started using it again at: {C.GREEN}{last_sleep['wake_at'].strftime('%A, %I:%M:%S %p')}{C.RESET}")
        print(f"  PC was idle for:               {C.CYAN}{hours}h {mins}m{C.RESET}")
        print()
        print(f"  {C.DIM}(This approximates your sleep schedule based on PC usage patterns){C.RESET}")
    elif periods:
        last = periods[-1]
        hours = int(last['duration_hours'])
        mins = int((last['duration_hours'] - hours) * 60)
        print(f"  Last significant idle period:")
        print(f"  Stopped at: {C.MAGENTA}{last['sleep_at'].strftime('%A, %I:%M:%S %p')}{C.RESET}")
        print(f"  Resumed at: {C.GREEN}{last['wake_at'].strftime('%A, %I:%M:%S %p')}{C.RESET}")
        print(f"  Duration:   {C.CYAN}{hours}h {mins}m{C.RESET}")
    else:
        print(f"  {C.YELLOW}Could not determine sleep pattern.{C.RESET}")
        print(f"  {C.DIM}Your PC may have been in constant use, or activity data is sparse.{C.RESET}")
    
    print()


if __name__ == "__main__":
    main()
