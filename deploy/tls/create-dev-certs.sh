#!/usr/bin/env bash
set -euo pipefail
umask 077

if [[ $# != 2 ]]; then
  printf 'Usage: %s NEW_OUTPUT_DIRECTORY SERVER_DNS_NAME\n' "$0" >&2
  exit 2
fi
certificate_directory=$1
server_name=$2
[[ $server_name =~ ^[A-Za-z0-9]([A-Za-z0-9.-]*[A-Za-z0-9])?$ && ${#server_name} -le 253 ]] || {
  echo 'Use one plain DNS hostname (no scheme, port, wildcard, or whitespace).' >&2
  exit 2
}
command -v openssl >/dev/null || { echo 'OpenSSL 3+ is required.' >&2; exit 1; }
[[ ! -e $certificate_directory ]] || { echo 'Output directory already exists; refusing to overwrite keys.' >&2; exit 1; }
mkdir -m 0700 -- "$certificate_directory"
certificate_directory=$(cd -- "$certificate_directory" && pwd)
mkdir -m 0700 -- "$certificate_directory/ca" "$certificate_directory/server" "$certificate_directory/client"
openssl req -x509 -newkey rsa:3072 -noenc -days 7 \
  -keyout "$certificate_directory/ca/ca.key" -out "$certificate_directory/ca/ca.pem" \
  -subj '/CN=HyperMind development CA' \
  -addext 'basicConstraints=critical,CA:TRUE' -addext 'keyUsage=critical,keyCertSign,cRLSign'
for identity in server client; do
  purpose=serverAuth
  subject="/CN=$server_name"
  extensions=(-addext "subjectAltName=DNS:$server_name")
  if [[ $identity == client ]]; then
    purpose=clientAuth
    subject='/CN=HyperMind development client'
    extensions=()
  fi
  openssl req -new -newkey rsa:3072 -noenc \
    -keyout "$certificate_directory/$identity/$identity.key" \
    -out "$certificate_directory/$identity/$identity.csr" -subj "$subject" \
    -addext 'basicConstraints=critical,CA:FALSE' \
    -addext 'keyUsage=critical,digitalSignature,keyEncipherment' \
    -addext "extendedKeyUsage=$purpose" "${extensions[@]}"
  openssl x509 -req -days 7 -copy_extensions copy \
    -in "$certificate_directory/$identity/$identity.csr" \
    -CA "$certificate_directory/ca/ca.pem" -CAkey "$certificate_directory/ca/ca.key" \
    -CAcreateserial -out "$certificate_directory/$identity/$identity.pem"
done
install -m 0644 "$certificate_directory/ca/ca.pem" "$certificate_directory/server/client-ca.pem"
install -m 0644 "$certificate_directory/ca/ca.pem" "$certificate_directory/client/server-ca.pem"
openssl verify -purpose sslserver -verify_hostname "$server_name" \
  -CAfile "$certificate_directory/ca/ca.pem" "$certificate_directory/server/server.pem"
openssl verify -purpose sslclient -CAfile "$certificate_directory/ca/ca.pem" "$certificate_directory/client/client.pem"
printf 'Development certificates valid for 7 days: %s\n' "$certificate_directory"
echo 'Mount only server/ into the daemon. Keep ca/ private and give clients only client/.'
echo 'Use your managed PKI and a rotation/revocation procedure for production.'
