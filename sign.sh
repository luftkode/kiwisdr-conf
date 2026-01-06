#!/bin/bash
set -e

# Check if --force was passed to the script
FORCE=false
if [[ "$1" == "--force" ]]; then
    FORCE=true
fi

sign() {
    local file="$1"
    local sig="${file}.asc"

    # If signature doesn't exist or file is newer, or the --force argument is passed, re-sign
    if [[ ! -f "$sig" || "$file" -nt "$sig" || "$FORCE" == true ]]; then
        gpg --batch --yes --armor --detach-sign --output "$sig" "$file"
        echo "✅ Signed $file"
    else
        echo "⏩ Skipped $file (no changes)"
    fi
}

sign cert/renew-cert.service
sign cert/renew-cert.timer
sign cert/renew-cert.sh

sign frontend/html/502.html
sign frontend/html/recorder.html
sign frontend/html/recorder.js
sign frontend/html/filebrowser.html
sign frontend/html/filebrowser.js
sign frontend/html/help.html
sign frontend/html/stylesheet.css

sign nginx/nginx-setup.sh
sign nginx/nginx.conf

sign kiwiclient/kiwiclient-setup.sh

sign backend/backend-setup.sh
sign backend/backend.service
sign backend/build/backend.armv7
sign backend/build/backend.aarch64
sign backend/build/backend.x86_64

sign setup.sh

sleep 3