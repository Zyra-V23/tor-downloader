#!/bin/bash

# ===================================================================
# ONION SITE DOWNLOADER - MASTER SCRIPT
# ===================================================================
# Author: AI Assistant
# Version: 1.0
# Description: Master script to manage all onion downloading operations
# ===================================================================

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
PURPLE='\033[0;35m'
NC='\033[0m'

# Script files
DOWNLOAD_SCRIPT="./download_onion_multithreaded.sh"
CONFIG_SCRIPT="./config.sh"
MONITOR_SCRIPT="./monitor.sh"
CONFIG_FILE="./download_config.conf"

# Functions
print_colored() {
    local color="$1"
    local message="$2"
    echo -e "${color}${message}${NC}"
}

print_banner() {
    clear
    print_colored "$PURPLE" "╔════════════════════════════════════════════════════════════════╗"
    print_colored "$PURPLE" "║                                                                ║"
    print_colored "$PURPLE" "║            🧅 ONION SITE DOWNLOADER SUITE 🧅                  ║"
    print_colored "$PURPLE" "║                    Multi-threaded & Fast                      ║"
    print_colored "$PURPLE" "║                                                                ║"
    print_colored "$PURPLE" "╚════════════════════════════════════════════════════════════════╝"
    echo
}

check_dependencies() {
    print_colored "$BLUE" "Checking dependencies..."
    
    local missing_deps=()
    
    # Check curl
    if ! command -v curl &> /dev/null; then
        missing_deps+=("curl")
    fi
    
    # Check tor
    if ! command -v tor &> /dev/null; then
        missing_deps+=("tor")
    fi
    
    # Check if scripts exist
    if [ ! -f "$DOWNLOAD_SCRIPT" ]; then
        missing_deps+=("download_onion_multithreaded.sh")
    fi
    
    if [ ! -f "$CONFIG_SCRIPT" ]; then
        missing_deps+=("config.sh")
    fi
    
    if [ ! -f "$MONITOR_SCRIPT" ]; then
        missing_deps+=("monitor.sh")
    fi
    
    if [ ${#missing_deps[@]} -eq 0 ]; then
        print_colored "$GREEN" "✓ All dependencies are available"
        return 0
    else
        print_colored "$RED" "✗ Missing dependencies:"
        for dep in "${missing_deps[@]}"; do
            echo "  - $dep"
        done
        return 1
    fi
}

check_tor_status() {
    if netstat -an 2>/dev/null | grep -q ":9050"; then
        print_colored "$GREEN" "✓ Tor is running on port 9050"
        return 0
    else
        print_colored "$RED" "✗ Tor is not running"
        return 1
    fi
}

start_tor() {
    print_colored "$BLUE" "Starting Tor..."
    
    if check_tor_status; then
        print_colored "$YELLOW" "Tor is already running"
        return 0
    fi
    
    print_colored "$BLUE" "Starting Tor daemon..."
    tor --quiet --daemon
    
    # Wait for Tor to start
    local attempts=0
    local max_attempts=10
    
    while [ $attempts -lt $max_attempts ]; do
        if check_tor_status; then
            print_colored "$GREEN" "✓ Tor started successfully"
            return 0
        fi
        
        sleep 2
        ((attempts++))
        print_colored "$YELLOW" "Waiting for Tor to start... ($attempts/$max_attempts)"
    done
    
    print_colored "$RED" "✗ Failed to start Tor"
    return 1
}

stop_tor() {
    print_colored "$BLUE" "Stopping Tor..."
    
    # Kill tor processes
    pkill -f "tor" 2>/dev/null
    
    sleep 2
    
    if ! check_tor_status; then
        print_colored "$GREEN" "✓ Tor stopped"
    else
        print_colored "$YELLOW" "Tor may still be running"
    fi
}

run_configuration() {
    print_colored "$BLUE" "Running configuration..."
    
    if [ -f "$CONFIG_SCRIPT" ]; then
        bash "$CONFIG_SCRIPT"
    else
        print_colored "$RED" "✗ Configuration script not found"
        return 1
    fi
}

start_download() {
    print_colored "$BLUE" "Starting download process..."
    
    # Check if configuration exists
    if [ ! -f "$CONFIG_FILE" ]; then
        print_colored "$YELLOW" "No configuration found. Running setup..."
        run_configuration
    fi
    
    # Check Tor
    if ! check_tor_status; then
        print_colored "$YELLOW" "Tor not running. Starting Tor..."
        start_tor
    fi
    
    # Start download
    if [ -f "$DOWNLOAD_SCRIPT" ]; then
        print_colored "$GREEN" "Starting multi-threaded download..."
        bash "$DOWNLOAD_SCRIPT"
    else
        print_colored "$RED" "✗ Download script not found"
        return 1
    fi
}

monitor_download() {
    print_colored "$BLUE" "Starting download monitor..."
    
    if [ -f "$MONITOR_SCRIPT" ]; then
        bash "$MONITOR_SCRIPT" --realtime
    else
        print_colored "$RED" "✗ Monitor script not found"
        return 1
    fi
}

show_status() {
    print_colored "$BLUE" "Current Status:"
    echo
    
    # Check Tor
    if check_tor_status; then
        print_colored "$GREEN" "🟢 Tor: Running"
    else
        print_colored "$RED" "🔴 Tor: Not running"
    fi
    
    # Check configuration
    if [ -f "$CONFIG_FILE" ]; then
        print_colored "$GREEN" "🟢 Configuration: Found"
    else
        print_colored "$YELLOW" "🟡 Configuration: Not found"
    fi
    
    # Check download directory
    if [ -d "./downloaded_site" ]; then
        local file_count=$(find "./downloaded_site" -type f 2>/dev/null | wc -l)
        print_colored "$GREEN" "🟢 Downloaded Files: $file_count"
    else
        print_colored "$YELLOW" "🟡 Downloaded Files: 0"
    fi
    
    # Check active downloads
    local active_downloads=$(pgrep -f "curl.*socks5-hostname" | wc -l)
    if [ $active_downloads -gt 0 ]; then
        print_colored "$GREEN" "🟢 Active Downloads: $active_downloads"
    else
        print_colored "$YELLOW" "🟡 Active Downloads: 0"
    fi
    
    echo
}

stop_download() {
    print_colored "$BLUE" "Stopping download process..."
    
    # Kill all curl processes
    pkill -f "curl.*socks5-hostname" 2>/dev/null
    
    # Kill download script
    pkill -f "download_onion_multithreaded.sh" 2>/dev/null
    
    print_colored "$GREEN" "✓ Download processes stopped"
}

show_logs() {
    print_colored "$BLUE" "Recent download logs:"
    echo
    
    if [ -f "./download.log" ]; then
        tail -20 "./download.log"
    else
        print_colored "$YELLOW" "No download logs found"
    fi
    
    echo
    print_colored "$RED" "Recent errors:"
    
    if [ -f "./error.log" ]; then
        tail -10 "./error.log"
    else
        print_colored "$YELLOW" "No error logs found"
    fi
}

cleanup() {
    print_colored "$BLUE" "Cleaning up temporary files..."
    
    # Remove temporary files
    rm -f /tmp/onion_download_semaphore_*
    rm -f /tmp/download_start_time
    
    print_colored "$GREEN" "✓ Cleanup completed"
}

quick_start() {
    print_colored "$GREEN" "🚀 QUICK START MODE"
    echo
    
    print_colored "$BLUE" "1. Checking dependencies..."
    if ! check_dependencies; then
        print_colored "$RED" "Please install missing dependencies first"
        return 1
    fi
    
    print_colored "$BLUE" "2. Starting Tor..."
    start_tor
    
    print_colored "$BLUE" "3. Setting up configuration..."
    bash "$CONFIG_SCRIPT" --interactive
    
    print_colored "$BLUE" "4. Starting download..."
    start_download &
    
    print_colored "$BLUE" "5. Starting monitor..."
    sleep 3
    monitor_download
}

show_help() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo
    echo "Commands:"
    echo "  start          Start the download process"
    echo "  stop           Stop the download process"
    echo "  monitor        Monitor download progress"
    echo "  config         Configure download settings"
    echo "  status         Show current status"
    echo "  logs           Show recent logs"
    echo "  tor-start      Start Tor daemon"
    echo "  tor-stop       Stop Tor daemon"
    echo "  tor-status     Check Tor status"
    echo "  cleanup        Clean temporary files"
    echo "  quick-start    Quick start with guided setup"
    echo "  help           Show this help"
    echo
    echo "Examples:"
    echo "  $0 quick-start    # Best for first time users"
    echo "  $0 start          # Start download with existing config"
    echo "  $0 monitor        # Monitor current download"
    echo "  $0 config         # Configure settings"
}

main_menu() {
    while true; do
        print_banner
        show_status
        
        print_colored "$CYAN" "Available Commands:"
        echo "1. 🚀 Quick Start (Recommended for first time)"
        echo "2. ⚙️  Configure Settings"
        echo "3. 🎯 Start Download"
        echo "4. 📊 Monitor Progress"
        echo "5. 🔍 View Logs"
        echo "6. 🧅 Tor Management"
        echo "7. 🛑 Stop Download"
        echo "8. 🧹 Cleanup"
        echo "9. ❓ Help"
        echo "0. 🚪 Exit"
        echo
        
        echo -ne "${BLUE}Select option (0-9): ${NC}"
        read -r choice
        
        case "$choice" in
            1) quick_start ;;
            2) run_configuration ;;
            3) start_download ;;
            4) monitor_download ;;
            5) show_logs ;;
            6) 
                echo "Tor Management:"
                echo "a. Start Tor"
                echo "b. Stop Tor"
                echo "c. Check Status"
                echo -ne "Select (a/b/c): "
                read -r tor_choice
                case "$tor_choice" in
                    a) start_tor ;;
                    b) stop_tor ;;
                    c) check_tor_status ;;
                esac
                ;;
            7) stop_download ;;
            8) cleanup ;;
            9) show_help ;;
            0) 
                print_colored "$GREEN" "Goodbye!"
                exit 0
                ;;
            *) 
                print_colored "$RED" "Invalid option"
                sleep 2
                ;;
        esac
        
        if [ "$choice" != "4" ]; then
            echo
            print_colored "$YELLOW" "Press Enter to continue..."
            read -r
        fi
    done
}

# Main execution
case "$1" in
    start)
        start_download
        ;;
    stop)
        stop_download
        ;;
    monitor)
        monitor_download
        ;;
    config)
        run_configuration
        ;;
    status)
        show_status
        ;;
    logs)
        show_logs
        ;;
    tor-start)
        start_tor
        ;;
    tor-stop)
        stop_tor
        ;;
    tor-status)
        check_tor_status
        ;;
    cleanup)
        cleanup
        ;;
    quick-start)
        quick_start
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        main_menu
        ;;
esac 