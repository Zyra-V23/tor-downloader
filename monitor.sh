#!/bin/bash

# ===================================================================
# DOWNLOAD MONITOR SCRIPT
# ===================================================================
# Author: AI Assistant
# Version: 1.0
# Description: Monitor download progress and statistics
# ===================================================================

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
LOG_FILE="./download.log"
ERROR_LOG="./error.log"
DOWNLOAD_DIR="./downloaded_site"
REFRESH_INTERVAL=5

# Function to print colored output
print_colored() {
    local color="$1"
    local message="$2"
    echo -e "${color}${message}${NC}"
}

# Function to get human readable size
human_readable_size() {
    local size="$1"
    if [ "$size" -lt 1024 ]; then
        echo "${size}B"
    elif [ "$size" -lt 1048576 ]; then
        echo "$((size / 1024))KB"
    elif [ "$size" -lt 1073741824 ]; then
        echo "$((size / 1048576))MB"
    else
        echo "$((size / 1073741824))GB"
    fi
}

# Function to count files recursively
count_files() {
    local dir="$1"
    if [ -d "$dir" ]; then
        find "$dir" -type f 2>/dev/null | wc -l
    else
        echo "0"
    fi
}

# Function to get directory size
get_dir_size() {
    local dir="$1"
    if [ -d "$dir" ]; then
        du -sb "$dir" 2>/dev/null | cut -f1
    else
        echo "0"
    fi
}

# Function to get download statistics
get_download_stats() {
    local total_files=$(count_files "$DOWNLOAD_DIR")
    local total_size=$(get_dir_size "$DOWNLOAD_DIR")
    local success_count=0
    local error_count=0
    local current_downloads=0
    
    # Count successful downloads
    if [ -f "$LOG_FILE" ]; then
        success_count=$(grep -c "Downloaded:" "$LOG_FILE" 2>/dev/null || echo "0")
    fi
    
    # Count errors
    if [ -f "$ERROR_LOG" ]; then
        error_count=$(grep -c "ERROR:" "$ERROR_LOG" 2>/dev/null || echo "0")
    fi
    
    # Count current active downloads (curl processes)
    current_downloads=$(pgrep -f "curl.*socks5-hostname" | wc -l)
    
    echo "$total_files|$total_size|$success_count|$error_count|$current_downloads"
}

# Function to display real-time statistics
display_stats() {
    local stats=$(get_download_stats)
    local total_files=$(echo "$stats" | cut -d'|' -f1)
    local total_size=$(echo "$stats" | cut -d'|' -f2)
    local success_count=$(echo "$stats" | cut -d'|' -f3)
    local error_count=$(echo "$stats" | cut -d'|' -f4)
    local current_downloads=$(echo "$stats" | cut -d'|' -f5)
    
    local human_size=$(human_readable_size "$total_size")
    
    clear
    print_colored "$CYAN" "╔════════════════════════════════════════════════════════════════╗"
    print_colored "$CYAN" "║                    DOWNLOAD MONITOR                           ║"
    print_colored "$CYAN" "╠════════════════════════════════════════════════════════════════╣"
    print_colored "$BLUE" "║ Files Downloaded: $total_files"
    print_colored "$BLUE" "║ Total Size: $human_size"
    print_colored "$GREEN" "║ Successful Downloads: $success_count"
    print_colored "$RED" "║ Errors: $error_count"
    print_colored "$YELLOW" "║ Active Downloads: $current_downloads"
    print_colored "$CYAN" "╠════════════════════════════════════════════════════════════════╣"
    print_colored "$BLUE" "║ Download Directory: $DOWNLOAD_DIR"
    print_colored "$BLUE" "║ Log File: $LOG_FILE"
    print_colored "$BLUE" "║ Error Log: $ERROR_LOG"
    print_colored "$CYAN" "╚════════════════════════════════════════════════════════════════╝"
    
    echo
    print_colored "$YELLOW" "Press Ctrl+C to exit monitor"
    echo
}

# Function to show recent activity
show_recent_activity() {
    print_colored "$BLUE" "Recent Downloads:"
    if [ -f "$LOG_FILE" ]; then
        tail -10 "$LOG_FILE" | grep "Downloaded:" | while read -r line; do
            echo "  $line"
        done
    else
        echo "  No activity yet"
    fi
    
    echo
    print_colored "$RED" "Recent Errors:"
    if [ -f "$ERROR_LOG" ]; then
        tail -5 "$ERROR_LOG" | while read -r line; do
            echo "  $line"
        done
    else
        echo "  No errors yet"
    fi
}

# Function to show top-level directory structure
show_directory_structure() {
    print_colored "$BLUE" "Directory Structure:"
    if [ -d "$DOWNLOAD_DIR" ]; then
        ls -la "$DOWNLOAD_DIR" | head -20
    else
        echo "  Download directory not found"
    fi
}

# Function to show download speed estimation
estimate_speed() {
    local start_time_file="/tmp/download_start_time"
    local current_time=$(date +%s)
    local start_time
    
    if [ -f "$start_time_file" ]; then
        start_time=$(cat "$start_time_file")
    else
        echo "$current_time" > "$start_time_file"
        start_time=$current_time
    fi
    
    local elapsed_time=$((current_time - start_time))
    
    if [ $elapsed_time -gt 0 ]; then
        local stats=$(get_download_stats)
        local total_size=$(echo "$stats" | cut -d'|' -f2)
        local speed_bps=$((total_size / elapsed_time))
        local speed_human=$(human_readable_size "$speed_bps")
        
        print_colored "$GREEN" "Download Speed: ${speed_human}/s"
        print_colored "$GREEN" "Elapsed Time: ${elapsed_time}s"
    else
        print_colored "$YELLOW" "Calculating speed..."
    fi
}

# Function to monitor in real-time
monitor_realtime() {
    print_colored "$GREEN" "Starting real-time monitor..."
    print_colored "$YELLOW" "Refresh interval: ${REFRESH_INTERVAL}s"
    
    while true; do
        display_stats
        show_recent_activity
        estimate_speed
        
        sleep "$REFRESH_INTERVAL"
    done
}

# Function to generate detailed report
generate_report() {
    local report_file="download_report_$(date +%Y%m%d_%H%M%S).txt"
    
    print_colored "$BLUE" "Generating detailed report..."
    
    {
        echo "ONION SITE DOWNLOAD REPORT"
        echo "Generated: $(date)"
        echo "=============================="
        echo
        
        echo "SUMMARY:"
        local stats=$(get_download_stats)
        local total_files=$(echo "$stats" | cut -d'|' -f1)
        local total_size=$(echo "$stats" | cut -d'|' -f2)
        local success_count=$(echo "$stats" | cut -d'|' -f3)
        local error_count=$(echo "$stats" | cut -d'|' -f4)
        
        echo "Total Files: $total_files"
        echo "Total Size: $(human_readable_size "$total_size")"
        echo "Successful Downloads: $success_count"
        echo "Errors: $error_count"
        echo
        
        echo "DIRECTORY STRUCTURE:"
        if [ -d "$DOWNLOAD_DIR" ]; then
            find "$DOWNLOAD_DIR" -type f | head -100
        fi
        echo
        
        echo "LARGEST FILES:"
        if [ -d "$DOWNLOAD_DIR" ]; then
            find "$DOWNLOAD_DIR" -type f -exec ls -la {} \; | sort -k5 -nr | head -10
        fi
        echo
        
        echo "DOWNLOAD LOG (last 50 entries):"
        if [ -f "$LOG_FILE" ]; then
            tail -50 "$LOG_FILE"
        fi
        echo
        
        echo "ERROR LOG:"
        if [ -f "$ERROR_LOG" ]; then
            cat "$ERROR_LOG"
        fi
        
    } > "$report_file"
    
    print_colored "$GREEN" "Report generated: $report_file"
}

# Function to show help
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo "Options:"
    echo "  -r, --realtime      Real-time monitoring"
    echo "  -s, --stats         Show current statistics"
    echo "  -a, --activity      Show recent activity"
    echo "  -d, --directory     Show directory structure"
    echo "  -R, --report        Generate detailed report"
    echo "  -h, --help          Show this help"
    echo "  (no options)        Show current statistics"
}

# Main script logic
case "$1" in
    -r|--realtime)
        monitor_realtime
        ;;
    -s|--stats)
        display_stats
        ;;
    -a|--activity)
        show_recent_activity
        ;;
    -d|--directory)
        show_directory_structure
        ;;
    -R|--report)
        generate_report
        ;;
    -h|--help)
        show_help
        ;;
    *)
        display_stats
        show_recent_activity
        ;;
esac 