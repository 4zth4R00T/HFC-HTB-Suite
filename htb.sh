#!/usr/bin/env bash

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_DIR="${SCRIPT_DIR}/lib"

source "${LIB_DIR}/htb-core.sh"
source "${LIB_DIR}/htb-vault.sh"
source "${LIB_DIR}/htb-i18n.sh"
source "${LIB_DIR}/htb-brand.sh"
source "${LIB_DIR}/htb-machines.sh"
source "${LIB_DIR}/htb-catalog.sh"
source "${LIB_DIR}/htb-vpn.sh"
source "${LIB_DIR}/htb-profile.sh"

htb_status_line() {
    local info; info="$(v4_get '/user/info')"
    local nick; nick="$(jf "$info" '.info.name')"
    if [[ "$nick" == "-" || -z "$nick" ]]; then
        printf '%s' "$(t main_prompt)"
        return
    fi
    HTB_PLAN="$(jf "$info" '.info.subscriptionType')"
    HTB_VIP="$(jf "$info" '.info.canAccessVIP')"
    local active; active="$(v5_get '/virtual_machine/active')"
    local target; target="$(jf "$active" '.info.name')"
    if [[ "$target" == "-" || -z "$target" ]]; then
        printf '%s' "$nick"
        return
    fi
    local kind; kind="$(jf "$active" '.info.type')"
    [[ "$kind" != "-" && "$kind" != "machine" ]] && target="${target} (${kind})"
    local ip; ip="$(jf "$(v4_get '/machine/active')" '.info.ip')"
    if [[ "$ip" == "-" || -z "$ip" ]]; then
        printf '%s  ·  %s' "$nick" "$target"
    else
        printf '%s  ·  %s %s' "$nick" "$target" "$ip"
    fi
}

htb_menu_hint() {
    printf '%b' "${WHT}${HTB_AUTHOR}${R} ${GRY}$(t follow_author)${R} ${CYN}${HTB_AUTHOR_INSTAGRAM}${R}   ${GRN}${HTB_CREW}${R} ${GRY}$(t follow_crew)${R} ${CYN}${HTB_CREW_INSTAGRAM}${R}"
}

main_menu() {
    local status; status="$(htb_status_line)"
    local -a items=(
        "$(htb_row "$(t menu_machines)" "$(t menu_machines_desc)")"
        "$(htb_row "$(t menu_catalog)" "$(t menu_catalog_desc)")"
        "$(htb_row "$(t menu_vpn)" "$(t menu_vpn_desc)")"
        "$(htb_row "$(t menu_profile)" "$(t menu_profile_desc)")"
        "$(htb_row "$(t menu_stats)" "$(t menu_stats_desc)")"
        "$(htb_row "$(t menu_diagnostics)" "$(t menu_diagnostics_desc)")"
        "$(htb_row "$(t menu_settings)" "$(t menu_settings_desc)")"
        "$(htb_row "$(t menu_credits)" "$(t menu_credits_desc)")"
        "${YLW}$(t menu_quit)${R}"
    )
    local at=0
    while true; do
        htb_pick "$status" "$GRN" "$(htb_menu_hint)" items "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) break ;;
            help) htb_help ;;
            enter)
                case "$REPLY_SEL" in
                    0) menu_machines ;;
                    1) menu_catalog ;;
                    2) menu_vpn ;;
                    3) menu_profile ;;
                    4) menu_stats ;;
                    5) menu_diagnostics ;;
                    6) menu_settings ;;
                    7) menu_credits ;;
                    8) break ;;
                esac
                status="$(htb_status_line)"
                items=(
                    "$(htb_row "$(t menu_machines)" "$(t menu_machines_desc)")"
                    "$(htb_row "$(t menu_catalog)" "$(t menu_catalog_desc)")"
                    "$(htb_row "$(t menu_vpn)" "$(t menu_vpn_desc)")"
                    "$(htb_row "$(t menu_profile)" "$(t menu_profile_desc)")"
                    "$(htb_row "$(t menu_stats)" "$(t menu_stats_desc)")"
                    "$(htb_row "$(t menu_diagnostics)" "$(t menu_diagnostics_desc)")"
                    "$(htb_row "$(t menu_settings)" "$(t menu_settings_desc)")"
                    "$(htb_row "$(t menu_credits)" "$(t menu_credits_desc)")"
                    "${YLW}$(t menu_quit)${R}"
                )
                ;;
        esac
    done
    clear
    printf '%b\n\n' "  ${GRY}$(t session_ended)${R}"
}

htb_cleanup() {
    htb_mouse_off
    printf '%s' "$HTB_SHOW"
}
trap htb_cleanup EXIT INT TERM

htb_load_theme
htb_load_lang
htb_check_deps

if [[ ! -s "$LANG_FILE" ]]; then
    menu_language
fi

htb_boot
htb_load_token
main_menu
