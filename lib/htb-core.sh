#!/usr/bin/env bash

umask 077

htb_clean_path() {
    local entry clean=""
    while IFS= read -r entry; do
        [[ -z "$entry" || "$entry" != /* ]] && continue
        [[ -d "$entry" ]] || continue
        local perms
        perms="$(stat -c '%A' "$entry" 2>/dev/null)" || perms="$(stat -f '%Sp' "$entry" 2>/dev/null)" || continue
        [[ "${perms:5:1}" == "w" || "${perms:8:1}" == "w" ]] && continue
        clean+="${clean:+:}$entry"
    done < <(printf '%s\n' "${PATH//:/$'\n'}")
    [[ -n "$clean" ]] && PATH="$clean"
    export PATH
}
htb_clean_path

htb_refuse_root() {
    [[ "${EUID:-$(id -u)}" -eq 0 ]] || return 0
    if [[ "${HTB_ALLOW_ROOT:-}" == "1" ]]; then
        printf '%b\n' "  \033[1;33mEjecutando como root por HTB_ALLOW_ROOT=1.\033[0m" >&2
        return 0
    fi
    printf '%b\n' "  \033[1;31mNo me ejecuto como root.\033[0m" >&2
    printf '%b\n' "  La suite solo necesita tu usuario. Si de verdad lo quieres:" >&2
    printf '%b\n' "  HTB_ALLOW_ROOT=1 ./htb.sh" >&2
    exit 1
}
htb_refuse_root

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/htb-hfc"
LEGACY_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/htb-suite"
TOKEN_FILE="$CONFIG_DIR/token"
THEME_FILE="$CONFIG_DIR/theme"
LANG_FILE="$CONFIG_DIR/lang"
VPN_DIR="${HTB_VPN_DIR:-$CONFIG_DIR/vpn}"
DL_DIR="${HTB_DL_DIR:-$HOME/HTB}"
mkdir -p "$CONFIG_DIR" "$VPN_DIR" "$DL_DIR" 2>/dev/null

htb_secure_config_dir() {
    if [[ -L "$CONFIG_DIR" ]]; then
        printf '%b\n' "  \033[1;31m${CONFIG_DIR} es un enlace simbolico. No guardo secretos ahi.\033[0m" >&2
        exit 1
    fi
    if [[ ! -O "$CONFIG_DIR" ]]; then
        printf '%b\n' "  \033[1;31m${CONFIG_DIR} pertenece a otro usuario. No guardo secretos ahi.\033[0m" >&2
        exit 1
    fi
    chmod 700 "$CONFIG_DIR" 2>/dev/null
}
htb_secure_config_dir

BASE_V4="https://labs.hackthebox.com/api/v4"
BASE_V5="https://labs.hackthebox.com/api/v5"
STORE_URL="https://www.hackthebox.com/hacker/subscriptions"
TOKEN_URL="https://app.hackthebox.com/profile/settings"

R=$'\033[0m'; BLD=$'\033[1m'

htb_load_theme() {
    HTB_THEME="dark"
    [[ -s "$THEME_FILE" ]] && HTB_THEME="$(tr -d '\r\n' < "$THEME_FILE")"
    [[ "$HTB_THEME" != "light" ]] && HTB_THEME="dark"
    if [[ "$HTB_THEME" == "light" ]]; then
        GRN=$'\033[1;38;2;27;69;58m'
        RED=$'\033[1;38;2;140;51;46m'
        CYN=$'\033[1;38;2;47;111;140m'
        YLW=$'\033[1;38;2;150;115;26m'
        GRY=$'\033[0;38;2;41;94;73m'
        WHT=$'\033[1;38;2;12;26;22m'
        PRP=$'\033[1;38;2;107;74;140m'
        ORG=$'\033[0;38;2;138;85;39m'
        DIM=$'\033[2;38;2;91;122;112m'
    else
        GRN=$'\033[1;38;2;71;131;107m'
        RED=$'\033[1;38;2;176;69;63m'
        CYN=$'\033[1;38;2;79;158;196m'
        YLW=$'\033[1;38;2;201;162;39m'
        GRY=$'\033[0;38;2;122;154;144m'
        WHT=$'\033[1;38;2;201;214;210m'
        PRP=$'\033[1;38;2;166;126;196m'
        ORG=$'\033[0;38;2;193;122;63m'
        DIM=$'\033[2;38;2;61;94;84m'
    fi
}

htb_set_theme() {
    printf '%s' "$1" > "$THEME_FILE"
    htb_load_theme
}

declare -gA MSG

htb_load_lang() {
    HTB_LANG="es"
    [[ -s "$LANG_FILE" ]] && HTB_LANG="$(tr -d '\r\n' < "$LANG_FILE")"
    case "$HTB_LANG" in
        en) _catalog_en ;;
        ru) _catalog_ru ;;
        *)  HTB_LANG="es"; _catalog_es ;;
    esac
}

htb_set_lang() {
    printf '%s' "$1" > "$LANG_FILE"
    htb_load_lang
}

t() { printf '%s' "${MSG[$1]:-$1}"; }

htb_check_deps() {
    local missing=()
    local dep
    for dep in curl jq openssl python3 xxd base64; do
        command -v "$dep" &>/dev/null || missing+=("$dep")
    done
    (( ${#missing[@]} == 0 )) && return 0
    printf '%b\n' "${YLW}$(t deps_missing): ${missing[*]}${R}"
    printf '%b\n' "${GRY}$(t deps_manual): ${missing[*]}${R}"
    exit 1
}

htb_load_token() {
    if [[ -n "${HTB_TOKEN:-}" ]]; then
        API_TOKEN="$HTB_TOKEN"
        htb_note "$(t token_env_warning)"
        return 0
    fi

    if htb_vault_exists; then
        htb_unlock_vault
        return 0
    fi

    local legacy=""
    [[ -s "$TOKEN_FILE" ]] && legacy="$(tr -d '\r\n ' < "$TOKEN_FILE")"
    [[ -z "$legacy" && -s "$LEGACY_DIR/token" ]] && legacy="$(tr -d '\r\n ' < "$LEGACY_DIR/token")"
    if [[ -n "$legacy" ]]; then
        clear; htb_show_mark; echo
        htb_note "$(t vault_migrating)"
        echo
        if htb_vault_ask_new_passphrase && htb_vault_seal "$legacy" "$REPLY_PASSPHRASE"; then
            API_TOKEN="$legacy"
            REPLY_PASSPHRASE=""
            rm -f "$TOKEN_FILE" "$LEGACY_DIR/token"
            htb_ok "$(t vault_migrated)"
            sleep 1
            return 0
        fi
        REPLY_PASSPHRASE=""
        htb_fail "$(t vault_migration_failed)"
        exit 1
    fi

    htb_first_token
}

htb_first_token() {
    clear
    htb_show_mark
    echo
    printf '%b\n'  "  ${CYN}$(t token_ask)${R}"
    printf '%b\n'  "  ${GRY}$(t token_where): ${TOKEN_URL}${R}"
    printf '%b\n\n' "  ${GRY}$(t token_stored): ${VAULT_FILE}${R}"
    printf '  %b' "${GRN}token > ${R}"
    IFS= read -rs API_TOKEN
    echo
    if [[ -z "$API_TOKEN" ]]; then
        printf '%b\n' "  ${RED}$(t token_empty)${R}"
        exit 1
    fi
    echo
    htb_note "$(t vault_choose_passphrase)"
    if ! htb_vault_ask_new_passphrase; then
        htb_fail "$(t vault_cancelled)"
        exit 1
    fi
    if ! htb_vault_seal "$API_TOKEN" "$REPLY_PASSPHRASE"; then
        REPLY_PASSPHRASE=""
        htb_fail "$(t vault_write_failed)"
        exit 1
    fi
    REPLY_PASSPHRASE=""
    htb_ok "$(t vault_sealed)"
    sleep 1
}

htb_unlock_vault() {
    clear
    htb_show_mark
    echo
    htb_field "$(t vault_cipher)" "$(htb_vault_label)"
    echo
    local attempt
    for attempt in 1 2 3; do
        if ! htb_vault_ask_passphrase "$(t vault_passphrase)"; then
            htb_fail "$(t vault_cancelled)"
            exit 1
        fi
        htb_vault_open "$REPLY_PASSPHRASE"
        local rc=$?
        REPLY_PASSPHRASE=""
        case $rc in
            0) API_TOKEN="$REPLY_TOKEN"; REPLY_TOKEN=""; return 0 ;;
            1) (( attempt < 3 )) && htb_fail "$(t vault_wrong_passphrase)" ;;
            2)
                if [[ "${VAULT_LAST_ERROR:-}" == "kdf:$VAULT_KDF_ARGON2ID" ]]; then
                    htb_fail "$(t vault_argon2_only)"
                else
                    htb_fail "$(t vault_unreadable)"
                fi
                exit 1 ;;
        esac
    done
    htb_fail "$(t vault_wrong_passphrase)"
    exit 1
}

SHERLOCK_ZIP_PASSWORD="hacktheblue"

_htb_curl_auth() {
    printf 'header = "Authorization: Bearer %s"\n' "$API_TOKEN"
    printf 'header = "Accept: application/json"\n'
}

v4_get()  { _htb_curl_auth | curl -s --config - --location --max-time 25 "${BASE_V4}${1}"; }
v5_get()  { _htb_curl_auth | curl -s --config - --location --max-time 25 "${BASE_V5}${1}"; }
_htb_post() {
    local base="$1" path="$2" body="$3"
    [[ -z "$body" ]] && body='{}'
    _htb_curl_auth | curl -s --config - --location --max-time 30 -X POST \
        -H "Content-Type: application/json" \
        --data "$body" "${base}${path}"
}

v4_post() { _htb_post "$BASE_V4" "$1" "$2"; }
v5_post() { _htb_post "$BASE_V5" "$1" "$2"; }

htb_http_code() {
    _htb_curl_auth | curl -s --config - -o /dev/null -w '%{http_code}' --max-time 12 "$1"
}

htb_api_ok() {
    local body="$1"
    if is_json "$body"; then
        [[ "$(jf "$body" '.message')" == "Too Many Attempts." ]] || return 0
        echo; htb_note "$(t api_rate_limited)"; htb_enter; return 1
    fi
    echo
    if printf '%s' "$body" | grep -qi '1015\|too many\|rate limit'; then
        htb_note "$(t api_rate_limited)"
    else
        htb_fail "$(t api_bad_response)"
    fi
    htb_enter
    return 1
}

is_json() { printf '%s' "${1:0:40}" | grep -q '^[[:space:]]*[\[{]'; }
jf()      { printf '%s' "$1" | jq -r "${2} // \"-\"" 2>/dev/null; }

urlencode() {
    local s="$1" i c out="" LC_ALL=C
    for (( i=0; i<${#s}; i++ )); do
        c="${s:i:1}"
        case "$c" in
            [a-zA-Z0-9.~_-]) out+="$c" ;;
            ' ') out+="+" ;;
            *) printf -v c '%%%02X' "'$c"; out+="$c" ;;
        esac
    done
    printf '%s' "$out"
}

htb_read_flag_answer() {
    local m; m="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
    case "$m" in
        *incorrect*|*"not correct"*|*wrong*|*invalid*|*already*|*failed*|*error*) printf 'no'; return ;;
    esac
    case "$m" in
        *correct*|*owned*|*congrat*|*obtained*|*accepted*|*solved*|*"well done"*) printf 'si'; return ;;
    esac
    printf 'duda'
}

htb_mentions_plan() {
    local m; m="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
    case "$m" in
        *vip*|*upgrade*|*subscription*|*"free server"*|*"requires a"*|*premium*) return 0 ;;
    esac
    return 1
}

htb_report_plan() {
    htb_mentions_plan "$1" || return 0
    echo
    htb_note "$(t vip_required)"
    htb_field "$(t your_plan)" "${YLW}${HTB_PLAN:-?}${R}"
    htb_field "$(t plan_upgrade)" "${CYN}${STORE_URL}${R}"
}

htb_read_action_answer() {
    local m; m="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
    case "$m" in
        *cannot*|*"can not"*|*"can't"*|*"could not"*|*"couldn't"*|*"not find"*|*"no active"*|\
        *"please switch"*|*"not allowed"*|*unauthorized*|*forbidden*|*"already have"*|\
        *error*|*invalid*|*failed*|*requires*|*upgrade*|*vip*) printf 'no'; return ;;
    esac
    case "$m" in
        *spawn*|*started*|*starting*|*deployed*|*playing*|*being*|*terminat*|*reset*|*extend*|*success*|*switched*) printf 'si'; return ;;
    esac
    printf 'duda'
}

htb_cols() { tput cols 2>/dev/null || echo 100; }
htb_rows() { tput lines 2>/dev/null || echo 30; }

htb_repeat() {
    local n="${1:-0}" ch="${2:-─}" s=""
    (( n < 0 )) && n=0
    while (( ${#s} < n )); do s+="$ch"; done
    printf '%s' "${s:0:n}"
}

htb_rule() {
    printf '%b\n' "${2:-$GRY}$(htb_repeat "$(htb_cols)" "${1:-─}")${R}"
}

htb_title() {
    local text="$1" color="${2:-$GRN}" w; w=$(htb_cols)
    text="${text:0:$((w - 6))}"
    local fill; fill="$(htb_repeat $((w-2)) '═')"
    printf '%b\n' "${color}╔${fill}╗${R}"
    printf '%b\n' "${color}║${R}${BLD}  ${text}$(htb_repeat $((w - 4 - ${#text})) ' ')${R}${color}║${R}"
    printf '%b\n' "${color}╚${fill}╝${R}"
}

htb_ok()    { printf '%b\n' "  ${GRN}✓${R} $1"; }
htb_fail()  { printf '%b\n' "  ${RED}✗${R} $1"; }
htb_note()  { printf '%b\n' "  ${YLW}$1${R}"; }
htb_pad() {
    local text="$1" width="${2:-18}"
    local pad=$(( width - ${#text} ))
    (( pad < 1 )) && pad=1
    printf '%s%*s' "$text" "$pad" ''
}

htb_field() { printf '  %b%s%b%s\n' "$GRY" "$(htb_pad "$1" 18)" "$R" "$2"; }

htb_drain_keys() {
    local discard
    while read -rsn1 -t 0.01 discard 2>/dev/null; do :; done
}

htb_enter() {
    htb_drain_keys
    printf '\n  %b' "${GRY}$(t press_enter)${R}"
    read -rsn1
    echo
}

htb_invalid() {
    printf '%b\n' "  ${RED}$(t opt_invalid)${R}"
    sleep 0.7
}

htb_confirm() {
    printf '  %b' "${YLW}$1 ${GRY}[$(t yn)]${R} > "
    read -r answer
    [[ "$answer" =~ ^[sSyYдД]$ ]]
}

htb_prompt() {
    printf '  %b' "${CYN}$1${R} > "
    read -r REPLY_TEXT
}

htb_prompt_flag() {
    local pasted; pasted="$(htb_paste 2>/dev/null | head -1 | tr -d '\r\n')"
    if [[ "$pasted" =~ ^HTB\{.*\}$ || "$pasted" =~ ^[0-9a-fA-F]{32}$ ]]; then
        printf '  %b\n' "${DIM}$(t clipboard_flag): ${pasted}${R}"
        printf '  %b' "${CYN}$1${R} > "
        read -r -e -i "$pasted" REPLY_TEXT
    else
        printf '  %b' "${CYN}$1${R} > "
        read -r REPLY_TEXT
    fi
}

htb_spinner() {
    local pid=$1 msg="$2" frames='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏' i=0
    while kill -0 "$pid" 2>/dev/null; do
        printf '\r  %b%s%b  %s' "$GRN" "${frames:i++%10:1}" "$R" "$msg"
        sleep 0.08
    done
    printf '\r%*s\r' "$(htb_cols)" ''
}

htb_fetch() {
    local out; out=$(mktemp "$CONFIG_DIR/.fetch.XXXXXX")
    { "$1" "$2" > "$out"; } &
    htb_spinner $! "${3:-$(t loading)}"
    REPLY_BODY="$(cat "$out")"
    rm -f "$out"
}

htb_clean_text() {
    printf '%s' "$1" \
        | sed -e 's/<[^>]*>/ /g' \
              -e 's/&nbsp;/ /g' -e 's/&amp;/\&/g' -e 's/&lt;/</g' -e 's/&gt;/>/g' \
              -e 's/&quot;/"/g' -e "s/&#039;/'/g" \
        | tr -s ' '
}

htb_wrap_text() {
    htb_clean_text "$1" | fold -s -w "${2:-92}" | sed 's/^/  /'
}

htb_difficulty_color() {
    case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
        easy|"very easy") printf '%s' "$GRN" ;;
        medium)           printf '%s' "$YLW" ;;
        hard)             printf '%s' "$RED" ;;
        insane)           printf '%s' "$PRP" ;;
        *)                printf '%s' "$WHT" ;;
    esac
}

HTB_WELCOME_ES=(
    "la enumeración callada rinde más que el ruido"
    "cada flag es una prueba, no una casualidad"
    "el acceso es el principio del trabajo, no el final"
    "quien entiende el sistema termina mandando en él"
    "la paciencia abre más cajas que cualquier exploit"
    "hoy también hay algo nuevo que aprender"
)
HTB_WELCOME_EN=(
    "quiet enumeration pays better than noise"
    "every flag is proof, never luck"
    "access is where the work starts, not where it ends"
    "whoever understands the system ends up running it"
    "patience opens more boxes than any exploit"
    "there is something new to learn today too"
)
HTB_WELCOME_RU=(
    "тихая разведка даёт больше, чем шум"
    "каждый флаг — доказательство, а не везение"
    "доступ — это начало работы, а не её конец"
    "кто разобрался в системе, тот ей и управляет"
    "терпение вскрывает больше боксов, чем любой эксплойт"
    "сегодня тоже есть чему научиться"
)

htb_random_welcome() {
    local -n pool="HTB_WELCOME_$(printf '%s' "$HTB_LANG" | tr '[:lower:]' '[:upper:]')"
    printf '%s' "${pool[RANDOM % ${#pool[@]}]}"
}

HTB_HOME=$'\033[H'
HTB_ERASE_LINE=$'\033[K'
HTB_ERASE_BELOW=$'\033[J'
HTB_HIDE=$'\033[?25l'
HTB_SHOW=$'\033[?25h'

htb_frame_begin() { printf '%s%s' "$HTB_HIDE" "$HTB_HOME"; }
htb_frame_line()  { printf '%b%s\n' "$1" "$HTB_ERASE_LINE"; }
htb_frame_end()   { printf '%s%s' "$HTB_ERASE_BELOW" "$HTB_SHOW"; }

htb_keys() {
    local out="" pair key label
    for pair in "$@"; do
        key="${pair%%|*}"
        label="${pair#*|}"
        [[ -n "$out" ]] && out+="   "
        out+="${GRN}${key}${R} ${GRY}${label}${R}"
    done
    printf '  %s' "$out"
}

htb_row() {
    printf '%b%s%b%b%s%b' "$WHT" "$(htb_pad "$1" "${3:-20}")" "$R" "$DIM" "$2" "$R"
}

htb_copy() {
    local text="$1"
    [[ -z "$text" || "$text" == "-" ]] && return 1
    if   command -v wl-copy >/dev/null 2>&1; then printf '%s' "$text" | wl-copy 2>/dev/null
    elif command -v xclip   >/dev/null 2>&1; then printf '%s' "$text" | xclip -selection clipboard 2>/dev/null
    elif command -v xsel    >/dev/null 2>&1; then printf '%s' "$text" | xsel --clipboard --input 2>/dev/null
    elif command -v pbcopy  >/dev/null 2>&1; then printf '%s' "$text" | pbcopy 2>/dev/null
    elif htb_in_wsl && command -v clip.exe >/dev/null 2>&1; then printf '%s' "$text" | clip.exe 2>/dev/null
    else return 1
    fi
    return 0
}

htb_in_wsl() {
    [[ -r /proc/version ]] && grep -qi microsoft /proc/version
}

htb_paste() {
    if   command -v wl-paste >/dev/null 2>&1; then wl-paste --no-newline 2>/dev/null
    elif command -v xclip    >/dev/null 2>&1; then xclip -selection clipboard -o 2>/dev/null
    elif command -v xsel     >/dev/null 2>&1; then xsel --clipboard --output 2>/dev/null
    elif command -v pbpaste  >/dev/null 2>&1; then pbpaste 2>/dev/null
    fi
}

htb_show_copied() {
    local text="$1"
    echo
    if htb_copy "$text"; then
        htb_ok "$(t copied)"
        local line
        while IFS= read -r line; do printf '%b\n' "  ${CYN}${line}${R}"; done <<< "$text"
    else
        htb_note "$(t nothing_to_copy)"
    fi
    htb_enter
}

htb_mouse_on()  { printf '\033[?1000h\033[?1006h'; }
htb_mouse_off() { printf '\033[?1006l\033[?1000l'; }

htb_read_key() {
    local first rest byte
    IFS= read -rsn1 first || { REPLY_KEYNAME="QUIT"; return; }
    if [[ "$first" != $'\x1b' ]]; then
        case "$first" in
            ''|$'\n'|$'\r') REPLY_KEYNAME="ENTER" ;;
            $'\x7f'|$'\x08') REPLY_KEYNAME="BACK" ;;
            $'\x03') REPLY_KEYNAME="QUIT" ;;
            *) REPLY_KEYNAME="CHAR:${first}" ;;
        esac
        return
    fi
    IFS= read -rsn1 -t 0.06 rest
    if [[ -z "$rest" ]]; then REPLY_KEYNAME="ESC"; return; fi
    if [[ "$rest" != '[' && "$rest" != 'O' ]]; then REPLY_KEYNAME="ESC"; return; fi
    if [[ "$rest" == '[' ]]; then
        IFS= read -rsn1 -t 0.06 byte
        if [[ "$byte" == '<' ]]; then
            local mouse=""
            while IFS= read -rsn1 -t 0.06 byte; do
                [[ "$byte" == 'M' || "$byte" == 'm' ]] && break
                mouse+="$byte"
            done
            local mb mx my
            IFS=';' read -r mb mx my <<< "$mouse"
            case "$mb" in
                64) REPLY_KEYNAME="UP" ;;
                65) REPLY_KEYNAME="DOWN" ;;
                0)  if [[ "$byte" == 'M' ]]; then
                        REPLY_MOUSE_ROW="$my"; REPLY_MOUSE_COL="$mx"; REPLY_KEYNAME="CLICK"
                    else
                        REPLY_KEYNAME="NONE"
                    fi ;;
                2)  [[ "$byte" == 'M' ]] && REPLY_KEYNAME="BACK" || REPLY_KEYNAME="NONE" ;;
                *)  REPLY_KEYNAME="NONE" ;;
            esac
            return
        fi
        seq="$byte"
        if [[ ! "$byte" =~ [A-Za-z~] ]]; then
            while IFS= read -rsn1 -t 0.06 byte; do
                seq+="$byte"
                [[ "$byte" =~ [A-Za-z~] ]] && break
            done
        fi
        case "$seq" in
            A)  REPLY_KEYNAME="UP" ;;
            B)  REPLY_KEYNAME="DOWN" ;;
            C)  REPLY_KEYNAME="RIGHT" ;;
            D)  REPLY_KEYNAME="LEFT" ;;
            H|1~|7~) REPLY_KEYNAME="HOME" ;;
            F|4~|8~) REPLY_KEYNAME="END" ;;
            5~) REPLY_KEYNAME="PGUP" ;;
            6~) REPLY_KEYNAME="PGDN" ;;
            *)  REPLY_KEYNAME="ESC" ;;
        esac
        return
    fi
    local seq=""
    while IFS= read -rsn1 -t 0.06 byte; do
        seq+="$byte"
        [[ "$byte" =~ [A-Za-z~] ]] && break
    done
    case "$seq" in
        A)  REPLY_KEYNAME="UP" ;;
        B)  REPLY_KEYNAME="DOWN" ;;
        C)  REPLY_KEYNAME="RIGHT" ;;
        D)  REPLY_KEYNAME="LEFT" ;;
        H|1~|7~) REPLY_KEYNAME="HOME" ;;
        F|4~|8~) REPLY_KEYNAME="END" ;;
        5~) REPLY_KEYNAME="PGUP" ;;
        6~) REPLY_KEYNAME="PGDN" ;;
        *)  REPLY_KEYNAME="ESC" ;;
    esac
}

htb_pick() {
    local __title="$1" __color="${2:-$GRN}" __hint="$3" __name="$4" __start="${5:-0}" __head="${6:-}"
    local -n __items="$__name"
    local __count=${#__items[@]}
    REPLY_SEL=-1
    REPLY_KEY="quit"
    (( __count == 0 )) && return

    if [[ ! -t 0 || ! -t 1 ]]; then
        clear; htb_title "$__title" "$__color"; echo
        local __idx
        for __idx in "${!__items[@]}"; do
            printf '  %3d) %b\n' $(( __idx + 1 )) "${__items[$__idx]}"
        done
        [[ -n "$__hint" ]] && printf '\n  %b\n' "${GRY}${__hint}${R}"
        printf '  %b' "$(t pick_number) > "
        local __answer
        read -r __answer
        if [[ "$__answer" =~ ^[0-9]+$ ]] && (( __answer >= 1 && __answer <= __count )); then
            REPLY_SEL=$(( __answer - 1 ))
            REPLY_KEY="enter"
        fi
        return
    fi

    local -a __header=()
    if [[ -n "$__head" ]]; then
        local -n __header_ref="$__head"
        __header=("${__header_ref[@]}")
    fi
    local __cur=$__start __height __window __first __idx
    (( __cur < 0 || __cur >= __count )) && __cur=0
    local __fill __width
    __width=$(htb_cols)
    __fill="$(htb_repeat $((__width-2)) '═')"
    clear
    htb_mouse_on
    while true; do
        __height=$(htb_rows)
        __window=$(( __height - 9 - ${#__header[@]} ))
        (( __window < 3 )) && __window=3
        __first=0
        (( __cur >= __window )) && __first=$(( __cur - __window + 1 ))
        (( __first > __count - __window )) && __first=$(( __count - __window ))
        (( __first < 0 )) && __first=0

        local __shown="${__title:0:$((__width - 6))}"
        htb_frame_begin
        htb_frame_line "${__color}╔${__fill}╗${R}"
        htb_frame_line "${__color}║${R}${BLD}  ${__shown}$(htb_repeat $((__width - 4 - ${#__shown})) ' ')${R}${__color}║${R}"
        htb_frame_line "${__color}╚${__fill}╝${R}"
        for __idx in "${!__header[@]}"; do htb_frame_line "${__header[$__idx]}"; done
        htb_frame_line ""
        for (( __idx=__first; __idx < __first + __window && __idx < __count; __idx++ )); do
            if (( __idx == __cur )); then
                htb_frame_line "  ${__color}▌${R} ${__items[$__idx]}"
            else
                htb_frame_line "    ${__items[$__idx]}"
            fi
        done
        htb_frame_line ""
        [[ -n "$__hint" ]] && htb_frame_line "  ${__hint}"
        htb_frame_line "  ${GRY}$(( __cur + 1 ))/${__count}${R}   ${GRN}↑↓${R} ${GRY}$(t nav_move)${R}   ${GRN}↵${R} ${GRY}$(t nav_open)${R}   ${GRN}←${R} ${GRY}$(t nav_back)${R}   ${GRN}?${R} ${GRY}$(t nav_help)${R}"
        htb_frame_end

        htb_read_key
        if [[ "$REPLY_KEYNAME" == "CLICK" ]]; then
            local __first_row=$(( 5 + ${#__header[@]} ))
            local __hit=$(( REPLY_MOUSE_ROW - __first_row + __first ))
            if (( REPLY_MOUSE_ROW >= __first_row && __hit >= 0 && __hit < __count )); then
                if (( __hit == __cur )); then
                    htb_mouse_off
                    REPLY_SEL=$__cur; REPLY_KEY="enter"; return
                fi
                __cur=$__hit
            fi
            continue
        fi
        case "$REPLY_KEYNAME" in
            UP)    if (( __cur > 0 )); then (( __cur-- )); else __cur=$(( __count - 1 )); fi ;;
            DOWN)  if (( __cur < __count - 1 )); then (( __cur++ )); else __cur=0; fi ;;
            CHAR:k) if (( __cur > 0 )); then (( __cur-- )); else __cur=$(( __count - 1 )); fi ;;
            CHAR:j) if (( __cur < __count - 1 )); then (( __cur++ )); else __cur=0; fi ;;
            PGUP)  __cur=$(( __cur - __window )); (( __cur < 0 )) && __cur=0 ;;
            PGDN|"CHAR: ") __cur=$(( __cur + __window )); (( __cur > __count - 1 )) && __cur=$(( __count - 1 )) ;;
            HOME) __cur=0 ;;
            END)  __cur=$(( __count - 1 )) ;;
            RIGHT|ENTER) htb_mouse_off; REPLY_SEL=$__cur; REPLY_KEY="enter"; return ;;
            LEFT|ESC|BACK|QUIT|CHAR:q|CHAR:Q) htb_mouse_off; REPLY_SEL=-1; REPLY_KEY="quit"; return ;;
            "CHAR:?") htb_mouse_off; REPLY_SEL=$__cur; REPLY_KEY="help"; return ;;
            CHAR:*) htb_mouse_off; REPLY_SEL=$__cur; REPLY_KEY="${REPLY_KEYNAME#CHAR:}"; return ;;
        esac
    done
}

htb_help_row() {
    printf '%b\n' "  ${GRN}$(htb_pad "$1" 22)${R}${GRY}$2${R}"
}

htb_help() {
    clear; htb_title "$(t help_title)" "$CYN"; echo
    htb_help_row "↑ ↓        k j"      "$(t help_move)"
    htb_help_row "↵          →"        "$(t help_select)"
    htb_help_row "←          Esc  q"   "$(t help_back)"
    htb_help_row "AvPág  RePág"        "$(t help_page)"
    htb_help_row "Inicio  Fin"        "$(t help_edges)"
    htb_help_row "$(t help_keys)"      "$(t help_actions)"
    htb_help_row "?"                   "$(t help_show)"
    htb_help_row "c"                   "$(t help_copy)"
    htb_help_row "$(t help_mouse_key)" "$(t help_mouse)"
    htb_help_row "Ctrl+C"              "$(t help_quit)"
    htb_enter
}

htb_show_lines() {
    local __title="$1" __color="$2" __name="$3"
    local -n __lines="$__name"
    clear; htb_title "$__title" "$__color"; echo
    local __line
    for __line in "${__lines[@]}"; do printf '%b\n' "$__line"; done
    htb_enter
}

_pick_filter() {
    local title="$1"; shift
    local -a labels=() values=()
    local pair
    for pair in "$@"; do
        labels+=("${pair%%|*}")
        values+=("${pair#*|}")
    done
    REPLY_FILTER=""
    REPLY_FILTER_SET=0
    htb_pick "$title" "$CYN" "" labels
    if [[ "$REPLY_KEY" == "enter" ]]; then
        REPLY_FILTER="${values[$REPLY_SEL]}"
        REPLY_FILTER_SET=1
    fi
}

_filter_label() {
    local value="$1"; shift
    local pair
    for pair in "$@"; do
        [[ "${pair#*|}" == "$value" ]] && { printf '%s' "${pair%%|*}"; return; }
    done
    printf '%s' "$value"
}
