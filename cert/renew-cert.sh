#!/bin/bash
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

SSL_DIR="/etc/ssl/kiwisdr"
CA_DIR="$SSL_DIR/ca"
TS=$(date +%F-%H%M%S)
HOST="kiwisdr.local"
INTERFACE="eth0"

ipv4() {
    ipv4_address=$(
        ip addr show dev $INTERFACE 2>/dev/null |
        grep -w inet |
        awk '{print $2}' |
        cut -d '/' -f 1 |
        tr -d ' '
    )

    if [ -z "$ipv4_address" ]; then
        echo "Error: Could not find an IPv4 address for interface '$INTERFACE'."
        echo "Check if the interface exists or if it has an IP assigned."
        exit 1
    else
        echo "$ipv4_address"
    fi

    exit 0
}

IPV4=$(ipv4)

mkdir -p "$SSL_DIR" "$CA_DIR"

# ----------------------------------------------------------------------
# 1. Create KiwiCA (EC P-256) if not present
# ----------------------------------------------------------------------
if [[ ! -f "$CA_DIR/KiwiCA.key" || ! -f "$CA_DIR/KiwiCA.pem" ]]; then
  echo "Creating new EC (P-256) local CA: KiwiCA"
  openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:prime256v1 -out "$CA_DIR/KiwiCA.key"
  chmod 600 "$CA_DIR/KiwiCA.key"

  # Self-sign CA certificate (10 years)
  openssl req -x509 -new -key "$CA_DIR/KiwiCA.key" \
    -sha256 -days 3650 \
    -subj "/C=DK/ST=Aarhus/L=Skyby/O=SkyTEM Surveys ApS/OU=Local Development CA/CN=KiwiCA" \
    -out "$CA_DIR/KiwiCA.pem"
  chmod 644 "$CA_DIR/KiwiCA.pem"
else
  echo "Using existing KiwiCA"
fi

CONF_FILE=$(mktemp)
cat > "$CONF_FILE" <<EOF
[req]
default_bits       = 2048
prompt             = no
default_md         = sha256
distinguished_name = dn
req_extensions     = req_ext

[dn]
C = DK
ST = Aarhus
L = Skyby
O = SkyTEM Surveys ApS
OU = SkyTEM Surveys ApS
CN = ${HOST}

[req_ext]
subjectAltName = @alt_names

[server_cert]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer

[alt_names]
DNS.1 = ${HOST}
IP.1  = ${IPV4}
EOF

# ----------------------------------------------------------------------
# 3. Generate server EC key and CSR (P-256)
# ----------------------------------------------------------------------
openssl req -new -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
  -nodes -keyout "$SSL_DIR/kiwisdr.key" -out "$SSL_DIR/kiwisdr.csr" \
  -config "$CONF_FILE" -extensions req_ext

chmod 600 "$SSL_DIR/kiwisdr.key"

# ----------------------------------------------------------------------
# 4. Sign CSR with KiwiCA (produce server cert)
# ----------------------------------------------------------------------
openssl x509 -req -in "$SSL_DIR/kiwisdr.csr" \
  -CA "$CA_DIR/KiwiCA.pem" -CAkey "$CA_DIR/KiwiCA.key" \
  -CAcreateserial -out "$SSL_DIR/kiwisdr.crt" \
  -days 90 -sha256 -extfile "$CONF_FILE" -extensions server_cert

# tighten permissions
chmod 644 "$SSL_DIR/kiwisdr.crt"

# Cleanup
rm -f "$SSL_DIR/kiwisdr.csr" "$CONF_FILE"

# ----------------------------------------------------------------------
# 5. Reload nginx to apply new cert (best-effort)
# ----------------------------------------------------------------------
systemctl reload nginx || echo "⚠️  Warning: Failed to reload nginx. Please reload it manually."

echo "✅ New server certificate installed at: $SSL_DIR/kiwisdr.crt"
echo "✅ Local CA is: $CA_DIR/KiwiCA.pem"
echo
echo "ℹ️ To trust the chain in browsers/OS, import $CA_DIR/KiwiCA.pem into your OS/browser trust store."
