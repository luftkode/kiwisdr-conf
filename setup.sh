#!/bin/bash

set -euo pipefail

PGP_KEY_FINGERPRINT="3CB2F77A8047BEDC"

verify_signature() {
  local file="$1"
  local file_asc="${file}.asc"

  local gpg_output=$(gpg --verify --keyring=- --fingerprint "$file_asc" "$file" 2>&1)
  # Check the verification result. This is a basic check;
  # more robust checks might involve parsing the output of gpg --verify and
  # checking specific error codes.
  if grep -q "gpg: Good signature from " <<< "$gpg_output"; then
    echo "✅ Signature verified successfully for $file."
    return 0 # Success
  else
    echo "❌ Error: Signature verification failed for $file."
    echo "ℹ️ --- Verification Output ---"
    echo "$gpg_output"
    echo "ℹ️ --- End Verification Output ---"
    return 1 # Failure
  fi
}

# Check if the script is being run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  verify_signature nginx/nginx-setup.sh && sudo chmod +x nginx/nginx-setup.sh && sudo ./nginx/nginx-setup.sh
  verify_signature kiwiclient/kiwiclient-setup.sh && sudo chmod +x kiwiclient/kiwiclient-setup.sh && sudo ./kiwiclient/kiwiclient-setup.sh
  verify_signature backend/backend-setup.sh && sudo chmod +x backend/backend-setup.sh && sudo ./backend/backend-setup.sh

  sudo rm -R /tmp/kiwisdr-conf-main

  echo "✅ The KiwiSDR configuration is complete."
  if [ "${1:-}" = "--no-reboot" ]; then
    echo "ℹ️ The --no-reboot flag was provided, so the KiwiSDR will not reboot now."
    exit 0
  else
    echo "ℹ️ The KiwiSDR will now reboot."
    sudo reboot
  fi
fi



