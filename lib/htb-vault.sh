#!/usr/bin/env bash

VAULT_FILE="$CONFIG_DIR/token.vault"
VAULT_MAGIC_HEX="485442484643"
VAULT_VERSION=1
VAULT_KDF_ARGON2ID=1
VAULT_KDF_PBKDF2=2
VAULT_PBKDF2_ITERS=600000
VAULT_MIN_PASSPHRASE=12
VAULT_PREFIX_LEN=20
VAULT_SALT_LEN=8
VAULT_TAG_LEN=32
VAULT_PAD_TO=64

read -r -d '' HTB_VAULT_PY <<'PYEOF' || true
import binascii, hashlib, hmac, os, sys

PAD_TO = 64

def okm(passphrase, salt_hex, iters):
    return hashlib.pbkdf2_hmac(
        "sha512", passphrase, binascii.unhexlify(salt_hex), int(iters), 80
    )

def read_passphrase():
    return sys.stdin.buffer.readline().rstrip(b"\n")

cmd = sys.argv[1]

if cmd == "pad":
    token = sys.stdin.buffer.read()
    if not token or len(token) > 0xFFFF:
        sys.exit(2)
    body = 2 + len(token)
    total = ((body + PAD_TO - 1) // PAD_TO) * PAD_TO
    out = len(token).to_bytes(2, "big") + token + os.urandom(total - body)
    sys.stdout.buffer.write(out)

elif cmd == "unpad":
    plain = sys.stdin.buffer.read()
    if len(plain) < 2:
        sys.exit(2)
    size = int.from_bytes(plain[:2], "big")
    if size == 0 or 2 + size > len(plain):
        sys.exit(2)
    sys.stdout.buffer.write(plain[2 : 2 + size])

elif cmd == "mac":
    _, _, salt_hex, iters, path = sys.argv
    key = okm(read_passphrase(), salt_hex, iters)[48:]
    with open(path, "rb") as fh:
        print(hmac.new(key, fh.read(), hashlib.sha256).hexdigest())

elif cmd == "verify":
    _, _, salt_hex, iters, path, tag_hex = sys.argv
    key = okm(read_passphrase(), salt_hex, iters)[48:]
    with open(path, "rb") as fh:
        tag = hmac.new(key, fh.read(), hashlib.sha256).digest()
    print("OK" if hmac.compare_digest(tag, binascii.unhexlify(tag_hex)) else "BAD")

else:
    sys.exit(2)
PYEOF

htb_vault_deps_missing() {
    local missing=() dep
    for dep in openssl python3 xxd base64; do
        command -v "$dep" >/dev/null 2>&1 || missing+=("$dep")
    done
    printf '%s' "${missing[*]}"
}

htb_vault_exists() { [[ -s "$VAULT_FILE" ]]; }

_vault_be32() { printf '%08x' "$1" | xxd -r -p; }

_vault_hex_to_dec() { printf '%d' "0x${1:-0}"; }

_vault_slice_hex() { xxd -p -c 256 -s "$2" -l "$3" "$1" | tr -d '\n'; }

_vault_tmp() {
    local path="$CONFIG_DIR/.vault.$1.$$"
    printf '%s' "$path"
}

htb_vault_ask_passphrase() {
    REPLY_PASSPHRASE=""
    printf '  %b' "${CYN}${1}${R} > "
    IFS= read -rs REPLY_PASSPHRASE
    echo
    [[ -n "$REPLY_PASSPHRASE" ]]
}

htb_vault_ask_new_passphrase() {
    local first second attempt
    for attempt in 1 2 3; do
        htb_vault_ask_passphrase "$(t vault_new_passphrase)" || return 1
        first="$REPLY_PASSPHRASE"
        if (( ${#first} < VAULT_MIN_PASSPHRASE )); then
            htb_fail "$(t vault_passphrase_short)"
            continue
        fi
        htb_vault_ask_passphrase "$(t vault_repeat_passphrase)" || return 1
        second="$REPLY_PASSPHRASE"
        if [[ "$first" != "$second" ]]; then
            htb_fail "$(t vault_passphrase_mismatch)"
            continue
        fi
        REPLY_PASSPHRASE="$first"
        return 0
    done
    return 1
}

htb_vault_seal() {
    local token="$1" pass="$2"
    local salt_hex ct body tag rc=0
    salt_hex="$(openssl rand -hex "$VAULT_SALT_LEN")" || return 1
    ct="$(_vault_tmp ct)"
    body="$(_vault_tmp body)"

    if ! printf '%s' "$token" \
        | python3 -c "$HTB_VAULT_PY" pad \
        | openssl enc -aes-256-cbc -pbkdf2 -md sha512 -iter "$VAULT_PBKDF2_ITERS" \
              -S "$salt_hex" -pass fd:3 -out "$ct" 3< <(printf '%s\n' "$pass")
    then
        rm -f "$ct" "$body"
        return 1
    fi

    local ct_len; ct_len="$(wc -c < "$ct")"
    {
        printf '%s' "$VAULT_MAGIC_HEX" | xxd -r -p
        printf '%02x%02x' "$VAULT_VERSION" "$VAULT_KDF_PBKDF2" | xxd -r -p
        _vault_be32 "$VAULT_PBKDF2_ITERS"
        _vault_be32 0
        _vault_be32 0
        printf '%s' "$salt_hex" | xxd -r -p
        _vault_be32 "$ct_len"
        cat "$ct"
    } > "$body"

    tag="$(printf '%s\n' "$pass" | python3 -c "$HTB_VAULT_PY" mac "$salt_hex" "$VAULT_PBKDF2_ITERS" "$body")" || rc=1
    if (( rc == 0 )) && [[ -n "$tag" ]]; then
        { cat "$body"; printf '%s' "$tag" | xxd -r -p; } | base64 | tr -d '\n' > "$VAULT_FILE"
        printf '\n' >> "$VAULT_FILE"
        chmod 600 "$VAULT_FILE"
    else
        rc=1
    fi
    rm -f "$ct" "$body"
    return $rc
}

htb_vault_open() {
    local pass="$1"
    REPLY_TOKEN=""
    local blob body rc=1
    blob="$(_vault_tmp blob)"
    body="$(_vault_tmp body)"
    base64 -d < "$VAULT_FILE" > "$blob" 2>/dev/null || { rm -f "$blob"; return 2; }

    local size; size="$(wc -c < "$blob")"
    if [[ "$(_vault_slice_hex "$blob" 0 6)" != "$VAULT_MAGIC_HEX" ]] \
       || (( $(_vault_hex_to_dec "$(_vault_slice_hex "$blob" 6 1)") != VAULT_VERSION )); then
        rm -f "$blob"; return 2
    fi

    local kdf; kdf="$(_vault_hex_to_dec "$(_vault_slice_hex "$blob" 7 1)")"
    if (( kdf != VAULT_KDF_PBKDF2 )); then
        rm -f "$blob"
        VAULT_LAST_ERROR="kdf:$kdf"
        return 2
    fi

    local iters; iters="$(_vault_hex_to_dec "$(_vault_slice_hex "$blob" 8 4)")"
    (( iters >= 100000 )) || { rm -f "$blob"; return 2; }

    local salt_hex; salt_hex="$(_vault_slice_hex "$blob" "$VAULT_PREFIX_LEN" "$VAULT_SALT_LEN")"
    local header_len=$(( VAULT_PREFIX_LEN + VAULT_SALT_LEN + 4 ))
    local ct_len; ct_len="$(_vault_hex_to_dec "$(_vault_slice_hex "$blob" $(( VAULT_PREFIX_LEN + VAULT_SALT_LEN )) 4)")"
    if (( ct_len == 0 || ct_len % 16 != 0 || size != header_len + ct_len + VAULT_TAG_LEN )); then
        rm -f "$blob"; return 2
    fi

    head -c $(( header_len + ct_len )) "$blob" > "$body"
    local tag_hex; tag_hex="$(_vault_slice_hex "$blob" $(( header_len + ct_len )) "$VAULT_TAG_LEN")"

    local verdict
    verdict="$(printf '%s\n' "$pass" | python3 -c "$HTB_VAULT_PY" verify "$salt_hex" "$iters" "$body" "$tag_hex")"
    if [[ "$verdict" != "OK" ]]; then
        rm -f "$blob" "$body"
        return 1
    fi

    REPLY_TOKEN="$(tail -c +$(( header_len + 1 )) "$blob" | head -c "$ct_len" \
        | openssl enc -d -aes-256-cbc -pbkdf2 -md sha512 -iter "$iters" \
              -S "$salt_hex" -pass fd:3 3< <(printf '%s\n' "$pass") 2>/dev/null \
        | python3 -c "$HTB_VAULT_PY" unpad)"
    [[ -n "$REPLY_TOKEN" ]] && rc=0
    rm -f "$blob" "$body"
    return $rc
}

htb_vault_label() {
    htb_vault_exists || { printf '%s' "-"; return; }
    local blob; blob="$(_vault_tmp label)"
    base64 -d < "$VAULT_FILE" > "$blob" 2>/dev/null || { rm -f "$blob"; printf '%s' "-"; return; }
    local kdf iters
    kdf="$(_vault_hex_to_dec "$(_vault_slice_hex "$blob" 7 1)")"
    iters="$(_vault_hex_to_dec "$(_vault_slice_hex "$blob" 8 4)")"
    rm -f "$blob"
    case "$kdf" in
        "$VAULT_KDF_ARGON2ID") printf 'Argon2id m=%sMiB' "$(( iters / 1024 ))" ;;
        "$VAULT_KDF_PBKDF2")   printf 'PBKDF2-HMAC-SHA512 %s %s' "$iters" "$(t vault_rounds)" ;;
        *) printf '%s' "-" ;;
    esac
}

htb_vault_wipe() {
    local path
    for path in "$VAULT_FILE" "$TOKEN_FILE" "$LEGACY_DIR/token"; do
        [[ -f "$path" ]] || continue
        dd if=/dev/zero of="$path" bs=1 count="$(wc -c < "$path")" conv=notrunc status=none 2>/dev/null
        rm -f "$path"
    done
}
