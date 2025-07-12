#!/bin/bash

# ===================================================================
# MULTI-THREADED RECURSIVE ONION SITE DOWNLOADER
# ===================================================================
# Author: AI Assistant
# Version: 1.0
# Description: Downloads .onion sites recursively with multi-threading
# ===================================================================

# Configuration - Load from config file if exists
CONFIG_FILE="./download_config.conf"
if [ -f "$CONFIG_FILE" ]; then
    source "$CONFIG_FILE"
    echo "Configuration loaded from $CONFIG_FILE"
else
    echo "Using default configuration"
fi

# Default values (used if not set in config file)
ONION_URL="${ONION_URL:-}"
TOR_PROXY="${TOR_PROXY:-127.0.0.1:9050}"
USER_AGENT="${USER_AGENT:-Mozilla/5.0 (Windows NT 10.0; rv:102.0) Gecko/20100101 Firefox/102.0}"
MAX_CONCURRENT_DOWNLOADS="${MAX_CONCURRENT_DOWNLOADS:-8}"
DOWNLOAD_DIR="${DOWNLOAD_DIR:-./downloaded_site}"
LOG_FILE="${LOG_FILE:-./download.log}"
ERROR_LOG="${ERROR_LOG:-./error.log}"
RETRY_ATTEMPTS="${RETRY_ATTEMPTS:-3}"
DELAY_BETWEEN_REQUESTS="${DELAY_BETWEEN_REQUESTS:-1}"
TIMEOUT="${TIMEOUT:-30}"

# Create directories
mkdir -p "$DOWNLOAD_DIR"
mkdir -p "$(dirname "$LOG_FILE")"

# Semaphore for controlling concurrent downloads
SEMAPHORE_DIR="/tmp/onion_download_semaphore_$$"
mkdir -p "$SEMAPHORE_DIR"

# Initialize semaphore
init_semaphore() {
    for ((i=0; i<MAX_CONCURRENT_DOWNLOADS; i++)); do
        echo > "$SEMAPHORE_DIR/$i"
    done
}

# Acquire semaphore
acquire_semaphore() {
    while true; do
        for ((i=0; i<MAX_CONCURRENT_DOWNLOADS; i++)); do
            if ln "$SEMAPHORE_DIR/$i" "$SEMAPHORE_DIR/lock_$i" 2>/dev/null; then
                echo "$i"
                return 0
            fi
        done
        sleep 0.1
    done
}

# Release semaphore
release_semaphore() {
    local slot="$1"
    rm -f "$SEMAPHORE_DIR/lock_$slot"
}

# Logging functions
log_info() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] INFO: $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: $1" | tee -a "$ERROR_LOG"
}

log_debug() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] DEBUG: $1" >> "$LOG_FILE"
}

# Clean filename for Windows compatibility
clean_filename() {
    local filename="$1"
    # Remove/replace problematic characters
    filename=$(echo "$filename" | sed 's/[<>:"|?*]/_/g')
    filename=$(echo "$filename" | sed 's/\.\./\./g')
    echo "$filename"
}

# URL decode function
url_decode() {
    local url="$1"
    echo "$url" | sed 's/+/ /g; s/%20/ /g; s/%21/!/g; s/%22/"/g; s/%23/#/g; s/%24/$/g; s/%25/%/g; s/%26/\&/g; s/%27/'\''/g; s/%28/(/g; s/%29/)/g; s/%2A/*/g; s/%2B/+/g; s/%2C/,/g; s/%2D/-/g; s/%2E/./g; s/%2F/\//g'
}

# Download file with retry logic
download_file() {
    local url="$1"
    local output_path="$2"
    local attempt=1
    
    while [ $attempt -le $RETRY_ATTEMPTS ]; do
        log_debug "Attempting to download $url (attempt $attempt/$RETRY_ATTEMPTS)"
        
        if curl --socks5-hostname "$TOR_PROXY" \
               --user-agent "$USER_AGENT" \
               --connect-timeout "$TIMEOUT" \
               --max-time $((TIMEOUT * 2)) \
               --retry 2 \
               --retry-delay 2 \
               --create-dirs \
               --silent \
               --show-error \
               --location \
               --output "$output_path" \
               "$url" 2>/dev/null; then
            
            if [ -f "$output_path" ] && [ -s "$output_path" ]; then
                log_info "Downloaded: $url -> $output_path"
                return 0
            else
                log_error "Download failed (empty file): $url"
                rm -f "$output_path"
            fi
        else
            log_error "Download failed (curl error): $url"
        fi
        
        ((attempt++))
        [ $attempt -le $RETRY_ATTEMPTS ] && sleep $((DELAY_BETWEEN_REQUESTS * attempt))
    done
    
    log_error "Failed to download after $RETRY_ATTEMPTS attempts: $url"
    return 1
}

# Parse HTML and extract links
parse_html() {
    local html_file="$1"
    local base_url="$2"
    
    # Extract href attributes from HTML
    grep -oE 'href="[^"]*"' "$html_file" | \
    sed 's/href="//g; s/"//g' | \
    grep -v '^?' | \
    grep -v '^#' | \
    grep -v '^mailto:' | \
    grep -v '^javascript:' | \
    while IFS= read -r link; do
        # Skip parent directory links
        [ "$link" = "../" ] && continue
        
        # Convert relative URLs to absolute
        if [[ "$link" =~ ^https?:// ]]; then
            echo "$link"
        elif [[ "$link" =~ ^/ ]]; then
            echo "${ONION_URL}${link}"
        else
            echo "${base_url}${link}"
        fi
    done
}

# Check if URL is a directory (ends with /)
is_directory() {
    local url="$1"
    [[ "$url" =~ /$ ]]
}

# Get file extension
get_extension() {
    local filename="$1"
    echo "${filename##*.}"
}

# Process directory recursively
process_directory() {
    local url="$1"
    local local_path="$2"
    local depth="${3:-0}"
    
    # Prevent infinite recursion
    if [ $depth -gt 20 ]; then
        log_error "Maximum recursion depth reached for $url"
        return 1
    fi
    
    log_info "Processing directory: $url (depth: $depth)"
    
    # Create local directory
    mkdir -p "$local_path"
    
    # Download directory listing
    local temp_html="${local_path}/index.html.tmp"
    if ! download_file "$url" "$temp_html"; then
        log_error "Failed to download directory listing: $url"
        return 1
    fi
    
    # Parse HTML and process each link
    local links_file="${local_path}/links.tmp"
    parse_html "$temp_html" "$url" > "$links_file"
    
    # Process each link
    while IFS= read -r link; do
        [ -z "$link" ] && continue
        
        # Get the filename/dirname from URL
        local item_name=$(basename "$link")
        item_name=$(url_decode "$item_name")
        item_name=$(clean_filename "$item_name")
        
        local local_item_path="${local_path}/${item_name}"
        
        if is_directory "$link"; then
            # Process subdirectory recursively in background
            {
                local slot=$(acquire_semaphore)
                process_directory "$link" "$local_item_path" $((depth + 1))
                release_semaphore "$slot"
            } &
        else
            # Download file in background
            {
                local slot=$(acquire_semaphore)
                download_file "$link" "$local_item_path"
                release_semaphore "$slot"
                sleep "$DELAY_BETWEEN_REQUESTS"
            } &
        fi
        
    done < "$links_file"
    
    # Clean up temporary files
    rm -f "$temp_html" "$links_file"
    
    # Wait for all background jobs to complete
    wait
    
    log_info "Completed directory: $url"
}

# Main download function
main() {
    log_info "Starting multi-threaded recursive download"
    log_info "Target URL: $ONION_URL"
    log_info "Download directory: $DOWNLOAD_DIR"
    log_info "Max concurrent downloads: $MAX_CONCURRENT_DOWNLOADS"
    log_info "Tor proxy: $TOR_PROXY"
    
    # Initialize semaphore
    init_semaphore
    
    # Check if ONION_URL is set
    if [ -z "$ONION_URL" ]; then
        log_error "No .onion URL configured. Please run the configuration script first:"
        log_error "  ./config.sh"
        exit 1
    fi
    
    # Check if Tor is running
    if ! netstat -an 2>/dev/null | grep -q ":9050"; then
        log_error "Tor is not running on port 9050. Please start Tor first."
        exit 1
    fi
    
    # Test connection
    log_info "Testing connection to $ONION_URL"
    if ! curl --socks5-hostname "$TOR_PROXY" \
             --user-agent "$USER_AGENT" \
             --connect-timeout 10 \
             --max-time 20 \
             --silent \
             --head \
             "$ONION_URL" >/dev/null 2>&1; then
        log_error "Cannot connect to $ONION_URL. Check your Tor connection."
        exit 1
    fi
    
    log_info "Connection test successful"
    
    # Start recursive download
    process_directory "$ONION_URL/" "$DOWNLOAD_DIR"
    
    # Clean up semaphore
    rm -rf "$SEMAPHORE_DIR"
    
    log_info "Download completed!"
    log_info "Files saved to: $DOWNLOAD_DIR"
    log_info "Check $LOG_FILE for detailed logs"
    log_info "Check $ERROR_LOG for errors"
}

# Signal handlers
cleanup() {
    log_info "Cleaning up..."
    # Kill all background jobs
    jobs -p | xargs -r kill 2>/dev/null
    # Clean up semaphore
    rm -rf "$SEMAPHORE_DIR"
    exit 0
}

trap cleanup INT TERM

# Run main function
main "$@" 