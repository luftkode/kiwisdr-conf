#!/bin/bash

set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

source /tmp/kiwisdr-conf-main/setup.sh # Load verify_signature()
DIR=/tmp/kiwisdr-conf-main

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Install nginx if not installed
if command_exists nginx; then
    echo "✅ Nginx is already installed: $(nginx -v 2>&1)"
else
    echo "⬜ Installing Nginx..."
    sudo apt update -qq
    sudo apt install -y -qq nginx
    echo "✅ Nginx installed successfully: $(nginx -v 2>&1)"
fi

# Install openssl if not installed
if command_exists openssl; then
    echo "✅ OpenSSL is already installed: $(openssl version)"
else
    echo "⬜ Installing OpenSSL..."
    sudo apt update -qq
    sudo apt install -y -qq openssl
    echo "✅ OpenSSL installed successfully: $(openssl version)"
fi

# Generate self-signed TLS certificate
echo "⬜ Generating self-signed TLS certificate"

verify_signature $DIR/cert/renew-cert.sh && sudo chmod +x $DIR/cert/renew-cert.sh && sudo $DIR/cert/renew-cert.sh

echo "✅ Self-signed TLS certificate created at /etc/ssl/kiwisdr"


echo "⬜ Setting up monthly certificate renewal with systemd..."

# Renewal script
verify_signature $DIR/cert/renew-cert.sh && sudo cp $DIR/cert/renew-cert.sh /usr/local/bin/renew-proxy-cert.sh

# Systemd service with logging
verify_signature $DIR/cert/renew-cert.service && sudo cp $DIR/cert/renew-cert.service /etc/systemd/system/proxy-cert-renew.service

# Systemd timer
verify_signature $DIR/cert/renew-cert.timer && sudo cp $DIR/cert/renew-cert.timer /etc/systemd/system/proxy-cert-renew.timer

# Enable and start the timer
sudo systemctl daemon-reload
sudo systemctl enable proxy-cert-renew.timer
sudo systemctl start proxy-cert-renew.timer

echo "✅ Monthly certificate renewal via systemd is set up."
echo "ℹ️ To view certificate renewal logs: journalctl -u proxy-cert-renew.service"

# Configure web files
echo "⬜ Configuring web files"
verify_signature $DIR/frontend/html/stylesheet.css && sudo cp $DIR/frontend/html/stylesheet.css /var/www/html/stylesheet.css
verify_signature $DIR/frontend/html/502.html && sudo cp $DIR/frontend/html/502.html /var/www/html/502.html
verify_signature $DIR/frontend/html/recorder.html && sudo cp $DIR/frontend/html/recorder.html /var/www/html/recorder.html
verify_signature $DIR/frontend/html/recorder.js && sudo cp $DIR/frontend/html/recorder.js /var/www/html/recorder.js
verify_signature $DIR/frontend/html/help.html && sudo cp $DIR/frontend/html/help.html /var/www/html/help.html
verify_signature $DIR/frontend/html/filebrowser.html && sudo cp $DIR/frontend/html/filebrowser.html /var/www/html/filebrowser.html
verify_signature $DIR/frontend/html/filebrowser.js && sudo cp $DIR/frontend/html/filebrowser.js /var/www/html/filebrowser.js
sudo mkdir -p /var/www/html/media/ && sudo cp -r $DIR/frontend/html/media/* /var/www/html/media/
echo "✅ Web files are configured."

# Configure Nginx
echo "⬜ Configuring Nginx"
verify_signature $DIR/nginx/nginx.conf && sudo cp $DIR/nginx/nginx.conf /etc/nginx/sites-available/kiwisdr

# Disable default site
sudo rm -f /etc/nginx/sites-enabled/default

# Enable the site
sudo ln -sf /etc/nginx/sites-available/kiwisdr /etc/nginx/sites-enabled/kiwisdr

# Test configuration and reload Nginx
sudo nginx -t > /dev/null
sudo systemctl reload nginx

echo "✅ Nginx is configured. Access KiwiSDR at https://kiwisdr.local"