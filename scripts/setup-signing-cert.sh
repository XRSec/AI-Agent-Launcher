#!/bin/zsh
set -euo pipefail

CERT_NAME="${1:-AI Agent Launcher Local Developer}"
OUTPUT_DIR="${2:-$HOME/.aiagentlauncher/certs}"
KEYCHAIN="${KEYCHAIN:-$(security default-keychain | tr -d '\" \t')}"

if [[ -z "$KEYCHAIN" ]]; then
    KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"
fi

echo "==> Setting up persistent Code Signing Certificate: '$CERT_NAME'"
echo "==> Output directory: $OUTPUT_DIR"
echo "==> Target keychain:  $KEYCHAIN"

mkdir -p "$OUTPUT_DIR"
chmod 700 "$OUTPUT_DIR"

CNF_PATH="$OUTPUT_DIR/openssl.cnf"
KEY_PATH="$OUTPUT_DIR/key.pem"
CERT_PATH="$OUTPUT_DIR/cert.pem"
P12_PATH="$OUTPUT_DIR/codesign.p12"
PASS_PATH="$OUTPUT_DIR/p12_password.txt"

# 1. Create OpenSSL configuration for Code Signing
cat > "$CNF_PATH" <<EOF
[ req ]
default_bits = 2048
prompt = no
default_md = sha256
distinguished_name = dn
x509_extensions = codesign_ext

[ dn ]
CN = $CERT_NAME

[ codesign_ext ]
basicConstraints = critical,CA:FALSE
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
EOF

# 2. Generate private key and self-signed certificate if not already generated
if [[ -f "$KEY_PATH" && -f "$CERT_PATH" && -f "$P12_PATH" && -f "$PASS_PATH" ]]; then
    echo "==> Found existing certificate and key in $OUTPUT_DIR, reusing."
    P12_PASS="$(cat "$PASS_PATH")"
else
    echo "==> Generating new private key and self-signed certificate (valid 10 years)..."
    P12_PASS="$(openssl rand -hex 16)"
    echo -n "$P12_PASS" > "$PASS_PATH"
    chmod 600 "$PASS_PATH"

    openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
        -keyout "$KEY_PATH" \
        -out "$CERT_PATH" \
        -config "$CNF_PATH" > /dev/null 2>&1
    chmod 600 "$KEY_PATH"

    echo "==> Exporting PKCS#12 bundle (codesign.p12)..."
    openssl pkcs12 -export -legacy \
        -out "$P12_PATH" \
        -inkey "$KEY_PATH" \
        -in "$CERT_PATH" \
        -passout "pass:$P12_PASS" > /dev/null 2>&1
    chmod 600 "$P12_PATH"
fi

# 3. Clean up any existing duplicate certificate in keychain
if security find-certificate -c "$CERT_NAME" "$KEYCHAIN" > /dev/null 2>&1; then
    echo "==> Removing existing certificate with name '$CERT_NAME' from keychain..."
    security delete-certificate -c "$CERT_NAME" "$KEYCHAIN" > /dev/null 2>&1 || true
fi

# 4. Import PKCS#12 into keychain
echo "==> Importing identity into keychain..."
security import "$P12_PATH" -k "$KEYCHAIN" -P "$P12_PASS" -T /usr/bin/codesign -A

# 5. Add user-domain code signing trust
echo "==> Adding code-signing trust to user domain..."
security add-trusted-cert -d -r trustRoot -p codeSign "$CERT_PATH"

# 6. Verify valid identities
echo "==> Verifying codesigning identity..."
if security find-identity -p codesigning -v | grep -q "$CERT_NAME"; then
    echo "==> SUCCESS! Found valid codesigning identity: '$CERT_NAME'"
    security find-identity -p codesigning -v | grep "$CERT_NAME"
else
    echo "==> Warning: Identity was imported but 'security find-identity' did not list it as valid." >&2
    exit 1
fi

echo ""
echo "=================================================================="
echo " Persistent Code Signing Certificate Ready!"
echo " Name:     $CERT_NAME"
echo " Location: $P12_PATH"
echo " Password: $PASS_PATH"
echo "=================================================================="
