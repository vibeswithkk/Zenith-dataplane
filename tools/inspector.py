#!/usr/bin/env python3
"""
Zenith Inspector - Debug and inspect running engine
"""
import requests
import json
import sys
import time
from datetime import datetime

API_URL = "http://localhost:8080"

def get_status():
    """Get engine status"""
    try:
        resp = requests.get(f"{API_URL}/status", timeout=2)
        if resp.status_code == 200:
            return resp.json()
    except:
        return None

def get_plugins():
    """Get loaded plugins"""
    try:
        resp = requests.get(f"{API_URL}/plugins", timeout=2)
        if resp.status_code == 200:
            return resp.json()
    except:
        return []

def print_status(status):
    """Print formatted status"""
    if not status:
        print("[FAIL] Engine not responding")
        return
    
    print("=" * 60)
    print(f"Zenith Engine Status - {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 60)
    print(f"Status:       {status.get('status', 'unknown').upper()}")
    print(f"Buffer Size:  {status.get('buffer_len', 0):,}")
    print(f"Plugins:      {status.get('plugin_count', 0)}")
    print("=" * 60)

def print_plugins(plugins):
    """Print plugin list"""
    if not plugins:
        print("No plugins loaded")
        return
    
    print("\nLoaded Plugins:")
    print("-" * 60)
    for plugin in plugins:
        print(f"  [{plugin.get('id')}] {plugin.get('status')}")

def watch_mode():
    """Continuous monitoring"""
    print("Watching engine (Ctrl+C to exit)...\n")
    try:
        while True:
            status = get_status()
            plugins = get_plugins()
            
            # Clear screen (simple)
            print("\033[2J\033[H", end='')
            
            print_status(status)
            print_plugins(plugins)
            
            time.sleep(2)
    except KeyboardInterrupt:
        print("\n\nStopped monitoring")

def main():
    if len(sys.argv) > 1 and sys.argv[1] == "watch":
        watch_mode()
    else:
        status = get_status()
        plugins = get_plugins()
        print_status(status)
        print_plugins(plugins)

if __name__ == "__main__":
    main()
