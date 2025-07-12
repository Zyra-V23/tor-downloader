#!/bin/bash

# ===================================================================
# CONFIGURATION SCRIPT FOR ONION SITE DOWNLOADER
# ===================================================================
# Author: AI Assistant
# Version: 1.0
# Description: Configure parameters for the multi-threaded downloader
# ===================================================================

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration file
CONFIG_FILE="./download_config.conf"

# Default values
DEFAULT_ONION_URL=""
DEFAULT_TOR_PROXY="127.0.0.1:9050"
DEFAULT_USER_AGENT="Mozilla/5.0 (Windows NT 10.0; rv:102.0) Gecko/20100101 Firefox/102.0"
DEFAULT_MAX_CONCURRENT=8
DEFAULT_DOWNLOAD_DIR="./downloaded_site"
DEFAULT_RETRY_ATTEMPTS=3
DEFAULT_DELAY=1
DEFAULT_TIMEOUT=30

# Function to print colored output
print_colored() {
    local color="$1"
    local message="$2"
    echo -e "${color}${message}${NC}"
}

# Function to read user input with default value
read_with_default() {
    local prompt="$1"
    local default="$2"
    local result
    
    echo -ne "${BLUE}${prompt}${NC} [${YELLOW}${default}${NC}]: "
    read -r result
    
    if [ -z "$result" ]; then
        echo "$default"
    else
        echo "$result"
    fi
}

# Function to validate number input
validate_number() {
    local input="$1"
    local min="$2"
    local max="$3"
    
    if ! [[ "$input" =~ ^[0-9]+$ ]]; then
        return 1
    fi
    
    if [ "$input" -lt "$min" ] || [ "$input" -gt "$max" ]; then
        return 1
    fi
    
    return 0
}

# Function to validate URL
validate_url() {
    local url="$1"
    
    if [[ "$url" =~ ^https?://.*\.onion/?.*$ ]]; then
        return 0
    else
        return 1
    fi
}

# Function to validate proxy format
validate_proxy() {
    local proxy="$1"
    
    if [[ "$proxy" =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}:[0-9]+$ ]]; then
        return 0
    else
        return 1
    fi
}

# Function to load existing configuration
load_config() {
    if [ -f "$CONFIG_FILE" ]; then
        source "$CONFIG_FILE"
        print_colored "$GREEN" "✓ Configuration loaded from $CONFIG_FILE"
    else
        print_colored "$YELLOW" "⚠ No existing configuration found. Using defaults."
    fi
}

# Function to save configuration
save_config() {
    cat > "$CONFIG_FILE" << EOF
# Onion Site Downloader Configuration
# Generated on $(date)

# Target .onion URL
ONION_URL="$ONION_URL"

# Tor proxy settings
TOR_PROXY="$TOR_PROXY"

# User agent string
USER_AGENT="$USER_AGENT"

# Download settings
MAX_CONCURRENT_DOWNLOADS=$MAX_CONCURRENT_DOWNLOADS
DOWNLOAD_DIR="$DOWNLOAD_DIR"
RETRY_ATTEMPTS=$RETRY_ATTEMPTS
DELAY_BETWEEN_REQUESTS=$DELAY_BETWEEN_REQUESTS
TIMEOUT=$TIMEOUT

# Logging settings
LOG_FILE="$LOG_FILE"
ERROR_LOG="$ERROR_LOG"
EOF
    
    print_colored "$GREEN" "✓ Configuration saved to $CONFIG_FILE"
}

# Function to display current configuration
show_current_config() {
    print_colored "$BLUE" "\n=== CURRENT CONFIGURATION ==="
    echo "Target URL: $ONION_URL"
    echo "Tor Proxy: $TOR_PROXY"
    echo "Max Concurrent Downloads: $MAX_CONCURRENT_DOWNLOADS"
    echo "Download Directory: $DOWNLOAD_DIR"
    echo "Retry Attempts: $RETRY_ATTEMPTS"
    echo "Delay Between Requests: ${DELAY_BETWEEN_REQUESTS}s"
    echo "Request Timeout: ${TIMEOUT}s"
    echo "User Agent: $USER_AGENT"
    echo "Log File: $LOG_FILE"
    echo "Error Log: $ERROR_LOG"
    print_colored "$BLUE" "==============================\n"
}

# Function to test Tor connection
test_tor_connection() {
    print_colored "$BLUE" "Testing Tor connection..."
    
    # Check if Tor is running
    if ! netstat -an 2>/dev/null | grep -q ":9050"; then
        print_colored "$RED" "✗ Tor is not running on port 9050"
        print_colored "$YELLOW" "Please start Tor first: tor --quiet &"
        return 1
    fi
    
    # Test connection to a .onion site
    if curl --socks5-hostname "$TOR_PROXY" \
           --user-agent "$USER_AGENT" \
           --connect-timeout 10 \
           --max-time 20 \
           --silent \
           --head \
           "$ONION_URL" >/dev/null 2>&1; then
        print_colored "$GREEN" "✓ Tor connection successful"
        return 0
    else
        print_colored "$RED" "✗ Cannot connect to $ONION_URL"
        print_colored "$YELLOW" "Check your Tor connection and the .onion URL"
        return 1
    fi
}

# Function to estimate download size
estimate_download() {
    print_colored "$BLUE" "Estimating download size..."
    
    # This is a basic estimation - in reality, you'd need to crawl the site
    print_colored "$YELLOW" "Note: This is a rough estimation based on visible files"
    
    # Download the main page and count links
    local temp_file="/tmp/onion_estimate.html"
    if curl --socks5-hostname "$TOR_PROXY" \
           --user-agent "$USER_AGENT" \
           --connect-timeout 10 \
           --max-time 20 \
           --silent \
           --output "$temp_file" \
           "$ONION_URL" 2>/dev/null; then
        
        local file_count=$(grep -c 'href="[^"]*"' "$temp_file" | head -1)
        local dir_count=$(grep -c 'folder.gif' "$temp_file" | head -1)
        
        echo "Estimated files in root: $file_count"
        echo "Estimated directories: $dir_count"
        echo "Total estimated items: $((file_count + dir_count))"
        
        rm -f "$temp_file"
    else
        print_colored "$RED" "✗ Failed to estimate download size"
    fi
}

# Interactive configuration menu
configure_interactive() {
    print_colored "$GREEN" "\n=== INTERACTIVE CONFIGURATION ==="
    
    # Load existing config
    load_config
    
    # Set default values if not loaded
    ONION_URL="${ONION_URL:-$DEFAULT_ONION_URL}"
    TOR_PROXY="${TOR_PROXY:-$DEFAULT_TOR_PROXY}"
    USER_AGENT="${USER_AGENT:-$DEFAULT_USER_AGENT}"
    MAX_CONCURRENT_DOWNLOADS="${MAX_CONCURRENT_DOWNLOADS:-$DEFAULT_MAX_CONCURRENT}"
    DOWNLOAD_DIR="${DOWNLOAD_DIR:-$DEFAULT_DOWNLOAD_DIR}"
    RETRY_ATTEMPTS="${RETRY_ATTEMPTS:-$DEFAULT_RETRY_ATTEMPTS}"
    DELAY_BETWEEN_REQUESTS="${DELAY_BETWEEN_REQUESTS:-$DEFAULT_DELAY}"
    TIMEOUT="${TIMEOUT:-$DEFAULT_TIMEOUT}"
    LOG_FILE="${LOG_FILE:-./download.log}"
    ERROR_LOG="${ERROR_LOG:-./error.log}"
    
    echo
    
    # Configure URL
    while true; do
        if [ -z "$ONION_URL" ]; then
            echo -ne "${BLUE}Enter .onion URL${NC}: "
            read -r ONION_URL
        else
            ONION_URL=$(read_with_default "Enter .onion URL" "$ONION_URL")
        fi
        
        if [ -z "$ONION_URL" ]; then
            print_colored "$RED" "✗ .onion URL is required"
            continue
        fi
        
        if validate_url "$ONION_URL"; then
            break
        else
            print_colored "$RED" "✗ Invalid .onion URL format (must be http://something.onion)"
        fi
    done
    
    # Configure Tor proxy
    while true; do
        TOR_PROXY=$(read_with_default "Enter Tor proxy (IP:PORT)" "$TOR_PROXY")
        if validate_proxy "$TOR_PROXY"; then
            break
        else
            print_colored "$RED" "✗ Invalid proxy format (use IP:PORT)"
        fi
    done
    
    # Configure concurrent downloads
    while true; do
        MAX_CONCURRENT_DOWNLOADS=$(read_with_default "Max concurrent downloads (1-20)" "$MAX_CONCURRENT_DOWNLOADS")
        if validate_number "$MAX_CONCURRENT_DOWNLOADS" 1 20; then
            break
        else
            print_colored "$RED" "✗ Must be a number between 1 and 20"
        fi
    done
    
    # Configure download directory
    DOWNLOAD_DIR=$(read_with_default "Download directory" "$DOWNLOAD_DIR")
    
    # Configure retry attempts
    while true; do
        RETRY_ATTEMPTS=$(read_with_default "Retry attempts (1-10)" "$RETRY_ATTEMPTS")
        if validate_number "$RETRY_ATTEMPTS" 1 10; then
            break
        else
            print_colored "$RED" "✗ Must be a number between 1 and 10"
        fi
    done
    
    # Configure delay
    while true; do
        DELAY_BETWEEN_REQUESTS=$(read_with_default "Delay between requests in seconds (0-10)" "$DELAY_BETWEEN_REQUESTS")
        if validate_number "$DELAY_BETWEEN_REQUESTS" 0 10; then
            break
        else
            print_colored "$RED" "✗ Must be a number between 0 and 10"
        fi
    done
    
    # Configure timeout
    while true; do
        TIMEOUT=$(read_with_default "Request timeout in seconds (10-300)" "$TIMEOUT")
        if validate_number "$TIMEOUT" 10 300; then
            break
        else
            print_colored "$RED" "✗ Must be a number between 10 and 300"
        fi
    done
    
    # Configure User Agent
    USER_AGENT=$(read_with_default "User Agent string" "$USER_AGENT")
    
    # Configure log files
    LOG_FILE=$(read_with_default "Log file path" "$LOG_FILE")
    ERROR_LOG=$(read_with_default "Error log file path" "$ERROR_LOG")
    
    echo
    show_current_config
    
    # Ask for confirmation
    echo -ne "${BLUE}Save this configuration? (y/N): ${NC}"
    read -r confirm
    
    if [[ "$confirm" =~ ^[Yy]$ ]]; then
        save_config
        print_colored "$GREEN" "✓ Configuration saved successfully"
    else
        print_colored "$YELLOW" "Configuration not saved"
    fi
}

# Main menu
main_menu() {
    while true; do
        print_colored "$BLUE" "\n=== ONION SITE DOWNLOADER CONFIGURATOR ==="
        echo "1. Interactive Configuration"
        echo "2. Show Current Configuration"
        echo "3. Test Tor Connection"
        echo "4. Estimate Download Size"
        echo "5. Quick Start (Use Defaults)"
        echo "6. Advanced Settings"
        echo "7. Exit"
        echo -ne "${BLUE}Select option (1-7): ${NC}"
        
        read -r choice
        
        case "$choice" in
            1)
                configure_interactive
                ;;
            2)
                load_config
                show_current_config
                ;;
            3)
                load_config
                test_tor_connection
                ;;
            4)
                load_config
                estimate_download
                ;;
            5)
                # Quick start with defaults
                ONION_URL="$DEFAULT_ONION_URL"
                TOR_PROXY="$DEFAULT_TOR_PROXY"
                USER_AGENT="$DEFAULT_USER_AGENT"
                MAX_CONCURRENT_DOWNLOADS="$DEFAULT_MAX_CONCURRENT"
                DOWNLOAD_DIR="$DEFAULT_DOWNLOAD_DIR"
                RETRY_ATTEMPTS="$DEFAULT_RETRY_ATTEMPTS"
                DELAY_BETWEEN_REQUESTS="$DEFAULT_DELAY"
                TIMEOUT="$DEFAULT_TIMEOUT"
                LOG_FILE="./download.log"
                ERROR_LOG="./error.log"
                
                save_config
                print_colored "$GREEN" "✓ Quick start configuration saved"
                ;;
            6)
                print_colored "$YELLOW" "Advanced settings:"
                echo "- Edit $CONFIG_FILE manually"
                echo "- Modify download_onion_multithreaded.sh directly"
                echo "- Use environment variables"
                ;;
            7)
                print_colored "$GREEN" "Goodbye!"
                exit 0
                ;;
            *)
                print_colored "$RED" "Invalid option. Please select 1-7."
                ;;
        esac
    done
}

# Check if running interactively
if [ "$1" = "--interactive" ] || [ "$1" = "-i" ]; then
    main_menu
elif [ "$1" = "--test" ] || [ "$1" = "-t" ]; then
    load_config
    test_tor_connection
elif [ "$1" = "--show" ] || [ "$1" = "-s" ]; then
    load_config
    show_current_config
elif [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Usage: $0 [OPTIONS]"
    echo "Options:"
    echo "  -i, --interactive    Interactive configuration"
    echo "  -t, --test          Test Tor connection"
    echo "  -s, --show          Show current configuration"
    echo "  -h, --help          Show this help"
    echo "  (no options)        Run interactive menu"
else
    main_menu
fi 