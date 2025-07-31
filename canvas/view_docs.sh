#!/bin/bash

# Enhanced documentation viewer for your canvas library
echo "=========================================="
echo "CANVAS LIBRARY DOCUMENTATION"
echo "=========================================="

# Function to display module docs with colors
show_module() {
    local module=$1
    local title=$2
    
    echo -e "\n\033[1;34m=== $title ===\033[0m"
    echo -e "\033[33mFiles in $module:\033[0m"
    find src/$module -name "*.rs" 2>/dev/null | sort
    echo
    
    # Show doc comments for this module
    find src/$module -name "*.rs" 2>/dev/null | while read file; do
        if grep -q "///" "$file"; then
            echo -e "\033[32m--- $file ---\033[0m"
            grep -n "^\s*///" "$file" | sed 's/^\([0-9]*:\)\s*\/\/\/ /\1 /' | head -10
            echo
        fi
    done
}

# Main modules
show_module "canvas" "CANVAS SYSTEM"
show_module "autocomplete" "AUTOCOMPLETE SYSTEM" 
show_module "config" "CONFIGURATION SYSTEM"

# Show lib.rs and other root files
echo -e "\n\033[1;34m=== ROOT DOCUMENTATION ===\033[0m"
if [ -f "src/lib.rs" ]; then
    echo -e "\033[32m--- src/lib.rs ---\033[0m"
    grep -n "^\s*///" src/lib.rs | sed 's/^\([0-9]*:\)\s*\/\/\/ /\1 /' 2>/dev/null
fi

if [ -f "src/dispatcher.rs" ]; then
    echo -e "\033[32m--- src/dispatcher.rs ---\033[0m"
    grep -n "^\s*///" src/dispatcher.rs | sed 's/^\([0-9]*:\)\s*\/\/\/ /\1 /' 2>/dev/null
fi

echo -e "\n\033[1;36m=========================================="
echo "To view specific module documentation:"
echo "  ./view_canvas_docs.sh canvas"
echo "  ./view_canvas_docs.sh autocomplete" 
echo "  ./view_canvas_docs.sh config"
echo "==========================================\033[0m"

# If specific module requested
if [ $# -eq 1 ]; then
    show_module "$1" "$(echo $1 | tr '[:lower:]' '[:upper:]') MODULE DETAILS"
fi
