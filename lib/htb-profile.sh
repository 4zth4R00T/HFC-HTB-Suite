#!/usr/bin/env bash

_round_points() {
    local raw="$1"
    [[ "$raw" =~ ^-?[0-9]+$ ]] && { printf '%s' "$raw"; return; }
    [[ "$raw" =~ ^-?[0-9]*\.[0-9]+$ ]] && { printf '%.2f' "$raw"; return; }
    printf '%s' "$raw"
}

menu_profile() {
    htb_fetch v4_get "/user/info"
    local info="$REPLY_BODY"
    htb_api_ok "$info" || return
    if [[ "$(jf "$info" '.info.id')" == "-" ]]; then
        clear; htb_title "$(t profile_title)" "$GRN"; echo
        htb_fail "$(t token_invalid)"
        htb_enter; return
    fi
    local uid; uid="$(jf "$info" '.info.id')"
    local basic; basic="$(v4_get "/user/profile/basic/${uid}")"
    local p; p="$(printf '%s' "$basic" | jq -c '.profile // .' 2>/dev/null)"

    local -a lines=(
        ""
        "  ${BLD}${WHT}$(jf "$p" '.name')${R}"
        ""
        "  ${GRN}── $(t p_account)${R}"
        "$(printf '  %b%s%b%b%s%b  %b(#%s)%b' "$GRY" "$(htb_pad "$(t p_rank)" 18)" "$R" "$YLW" "$(jf "$p" '.rank')" "$R" "$DIM" "$(jf "$p" '.ranking')" "$R")"
        "$(printf '  %b%s%b%s — %s%% (%s pts)' "$GRY" "$(htb_pad "$(t p_next_rank)" 18)" "$R" "$(jf "$p" '.next_rank')" "$(jf "$p" '.current_rank_progress')" "$(_round_points "$(jf "$p" '.next_rank_points')")")"
        "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t p_points)" 18)" "$R" "$GRN" "$(jf "$p" '.points')" "$R")"
        "$(printf '  %b%s%buser %s · system %s' "$GRY" "$(htb_pad "$(t p_owns)" 18)" "$R" "$(jf "$p" '.user_owns')" "$(jf "$p" '.system_owns')")"
        "$(printf '  %b%s%buser %s · system %s · challenge %s' "$GRY" "$(htb_pad "$(t p_bloods)" 18)" "$R" "$(jf "$p" '.user_bloods')" "$(jf "$p" '.system_bloods')" "$(jf "$p" '.challenge_bloods')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_respects)" 18)" "$R" "$(jf "$p" '.respects')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_country)" 18)" "$R" "$(jf "$p" '.country_name')")"
        "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t p_server)" 18)" "$R" "$CYN" "$(jf "$p" '.server')" "$R")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_since)" 18)" "$R" "$(jf "$p" '.joined_date' | cut -c1-10)")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_team)" 18)" "$R" "$(jf "$p" '.team')")"
        ""
        "  ${GRN}── $(t p_subscription)${R}"
        "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t p_plan)" 18)" "$R" "$YLW" "$(jf "$info" '.info.subscriptionType')" "$R")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_vip)" 18)" "$R" "$([[ "$(jf "$info" '.info.canAccessVIP')" == "true" ]] && t yes || t no)")"
    )

    local subs; subs="$(v4_get '/user/subscriptions')"
    local count; count="$(printf '%s' "$subs" | jq '.subscriptions | length' 2>/dev/null)"
    if [[ "$count" =~ ^[0-9]+$ && "$count" -gt 0 ]]; then
        local row
        while IFS= read -r row; do lines+=("$row"); done < <(printf '%s' "$subs" | jq -r '.subscriptions[]? | "    · \(.name) (\(.status))"' 2>/dev/null)
    else
        lines+=("    ${DIM}· $(t sub_none)${R}" "    ${DIM}· ${STORE_URL}${R}")
    fi

    lines+=("" "  ${GRN}── $(t p_season)${R}")
    local list; list="$(v4_get '/season/list')"
    local season_id; season_id="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .id' 2>/dev/null | head -1)"
    if [[ -z "$season_id" ]]; then
        lines+=("    ${YLW}· $(t season_none)${R}")
    else
        local season_name; season_name="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .name' 2>/dev/null | head -1)"
        local season_state; season_state="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .state' 2>/dev/null | head -1)"
        local rank; rank="$(v4_get "/season/user/rank/${season_id}")"
        local league; league="$(jf "$rank" '.data.league')"
        [[ "$league" == "-" ]] && league="→ $(jf "$rank" '.data.next_rank.title')"
        lines+=("$(printf '  %b%s%b%s  %b(%s)%b' "$GRY" "$(htb_pad "$(t p_season)" 18)" "$R" "$season_name" "$DIM" "$season_state" "$R")")
        lines+=("$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t season_tier)" 18)" "$R" "$PRP" "$league" "$R")")
        lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t season_points)" 18)" "$R" "$(jf "$rank" '.data.total_season_points')")")
        lines+=("$(printf '  %b%s%b%s/%s' "$GRY" "$(htb_pad "$(t season_flags)" 18)" "$R" "$(jf "$rank" '.data.total_season_flags.obtained')" "$(jf "$rank" '.data.total_season_flags.total')")")
        lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t season_bloods)" 18)" "$R" "$(jf "$rank" '.data.total_season_bloods')")")
    fi

    htb_show_lines "$(t profile_title)" "$GRN" lines
}

menu_diagnostics() {
    clear; htb_title "$(t diag_title)" "$CYN"; echo
    printf '%b\n\n' "  ${GRY}$(t diag_running)${R}"
    printf '  %b%-8s%-30s%-10s%s%b\n\n' "$WHT" "$(t diag_status)" "$(t diag_endpoint)" "HTTP" "$(t diag_latency)" "$R"

    local uid; uid="$(jf "$(v4_get '/user/info')" '.info.id')"
    local season; season="$(v4_get '/season/list' | jq -r '.data[]? | select(.active == true) | .id' 2>/dev/null | head -1)"
    [[ -z "$season" ]] && season=1

    local -a probes=(
        "v4 /user/info|${BASE_V4}/user/info"
        "v4 /user/profile/basic|${BASE_V4}/user/profile/basic/${uid}"
        "v4 /user/subscriptions|${BASE_V4}/user/subscriptions"
        "v5 /machines|${BASE_V5}/machines?per_page=1"
        "v4 /machine/active|${BASE_V4}/machine/active"
        "v4 /connections/servers|${BASE_V4}/connections/servers?product=labs"
        "v4 /challenge/list|${BASE_V4}/challenge/list"
        "v4 /challenge/categories|${BASE_V4}/challenge/categories/list"
        "v4 /prolabs|${BASE_V4}/prolabs"
        "v4 /fortresses|${BASE_V4}/fortresses"
        "v4 /sherlocks|${BASE_V4}/sherlocks?page=1"
        "v4 /tracks|${BASE_V4}/tracks"
        "v4 /season/list|${BASE_V4}/season/list"
        "v4 /season/user/rank|${BASE_V4}/season/user/rank/${season}"
    )

    local healthy=0 total=0 row label url start finish ms code mark color
    for row in "${probes[@]}"; do
        IFS='|' read -r label url <<< "$row"
        start=$(date +%s%N)
        code="$(htb_http_code "$url")"
        finish=$(date +%s%N)
        ms=$(( (finish - start) / 1000000 ))
        ((total++))
        case "$code" in
            2*)     mark="OK";   color="$GRN"; ((healthy++)) ;;
            401|403) mark="AUTH"; color="$YLW" ;;
            *)      mark="FAIL"; color="$RED" ;;
        esac
        printf '  %b%-8s%b%-30s%-10s%b%s ms%b\n' "$color" "$mark" "$R" "$label" "$code" "$DIM" "$ms" "$R"
    done

    echo
    printf '%b\n\n' "  ${WHT}${healthy}/${total} $(t diag_summary)${R}"
    htb_field "$(t diag_token)" "$([[ -n "$API_TOKEN" ]] && printf '%b' "${GRN}$(t diag_configured)${R}" || printf '%b' "${RED}$(t diag_missing)${R}")"
    htb_field "$(t diag_theme)" "$HTB_THEME"
    htb_field "$(t diag_language)" "$HTB_LANG"
    htb_field "$(t diag_config)" "$CONFIG_DIR"
    htb_field "$(t diag_downloads)" "$DL_DIR"
    htb_enter
}

menu_credits() {
    clear; htb_title "$(t credits_title)" "$GRN"; echo
    htb_show_mark
    echo
    htb_field "$(t credits_author)" "${WHT}${HTB_AUTHOR}${R}"
    htb_field "" "${CYN}${HTB_AUTHOR_INSTAGRAM}${R}"
    htb_field "" "${DIM}${HTB_AUTHOR_GITHUB}${R}"
    echo
    htb_field "" "${GRN}${HTB_CREW}${R}"
    htb_field "" "${CYN}${HTB_CREW_INSTAGRAM}${R}"
    htb_field "" "${DIM}${HTB_CREW_BLOG}${R}"
    echo
    printf '%b\n\n' "  ${WHT}$(t credits_belongs)${R}"
    printf '%b\n' "  ${GRY}$(t credits_thanks)${R} ${ORG}${HTB_THANKS}${R}"
    printf '%b\n\n' "  ${DIM}$(t credits_thanks_note)${R}"
    printf '%b\n' "  ${DIM}$(t credits_license)${R}"
    htb_enter
}

menu_settings() {
    local at=0
    while true; do
        local -a items=(
            "$(htb_row "$(t set_theme)" "$HTB_THEME")"
            "$(htb_row "$(t set_language)" "$HTB_LANG")"
            "$(htb_row "$(t set_token)" "$(t set_token_desc)")"
            "$(htb_row "$(t set_passphrase)" "$(t set_passphrase_desc)")"
            "$(htb_row "$(t set_forget)" "$(t set_forget_desc)")"
            "$(htb_row "$(t set_paths)" "$CONFIG_DIR")"
        )
        htb_pick "$(t settings_title)" "$WHT" "" items "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                case "$REPLY_SEL" in
                    0) [[ "$HTB_THEME" == "dark" ]] && htb_set_theme light || htb_set_theme dark ;;
                    1) menu_language ;;
                    2)
                        clear; htb_title "$(t set_token)" "$WHT"; echo
                        printf '  %b' "${CYN}$(t ask_token)${R} > "
                        IFS= read -rs REPLY_TEXT
                        echo
                        if [[ -n "$REPLY_TEXT" ]]; then
                            local nuevo="$REPLY_TEXT"
                            REPLY_TEXT=""
                            echo
                            if htb_vault_ask_new_passphrase && htb_vault_seal "$nuevo" "$REPLY_PASSPHRASE"; then
                                REPLY_PASSPHRASE=""
                                API_TOKEN="$nuevo"
                                rm -f "$TOKEN_FILE" "$LEGACY_DIR/token"
                                echo
                                if [[ "$(jf "$(v4_get '/user/info')" '.info.id')" != "-" ]]; then
                                    htb_ok "$(t token_updated)"
                                else
                                    htb_fail "$(t token_invalid)"
                                fi
                            else
                                REPLY_PASSPHRASE=""
                                htb_fail "$(t vault_cancelled)"
                            fi
                            nuevo=""
                            htb_enter
                        fi ;;
                    3)
                        clear; htb_title "$(t set_passphrase)" "$WHT"; echo
                        if htb_vault_ask_new_passphrase && htb_vault_seal "$API_TOKEN" "$REPLY_PASSPHRASE"; then
                            REPLY_PASSPHRASE=""
                            htb_ok "$(t passphrase_updated)"
                        else
                            REPLY_PASSPHRASE=""
                            htb_fail "$(t vault_cancelled)"
                        fi
                        htb_enter ;;
                    4)
                        clear; htb_title "$(t set_forget)" "$YLW"; echo
                        if htb_confirm "$(t confirm_forget)"; then
                            htb_vault_wipe
                            htb_ok "$(t token_removed)"
                            htb_enter
                        fi ;;
                    5)
                        local -a paths=(
                            ""
                            "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t diag_config)" 18)" "$R" "$CONFIG_DIR")"
                            "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t diag_downloads)" 18)" "$R" "$DL_DIR")"
                            "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "VPN" 18)" "$R" "$VPN_DIR")"
                            "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t diag_token)" 18)" "$R" "$VAULT_FILE")"
                            "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t vault_cipher)" 18)" "$R" "$(htb_vault_label)")"
                        )
                        htb_show_lines "$(t set_paths)" "$WHT" paths ;;
                esac ;;
        esac
    done
}

menu_language() {
    local -a items=("Español" "English" "Русский")
    htb_pick "$(t lang_title)" "$WHT" "$(t lang_prompt)" items
    [[ "$REPLY_KEY" != "enter" ]] && return
    case "$REPLY_SEL" in
        0) htb_set_lang es ;;
        1) htb_set_lang en ;;
        2) htb_set_lang ru ;;
    esac
}

htb_bar() {
    local done="$1" total="$2" color="$3" width=28
    local filled=0
    if [[ "$total" =~ ^[0-9]+$ ]] && (( total > 0 )); then
        filled=$(( done * width / total ))
        (( filled > width )) && filled=$width
    fi
    local percent=0
    (( total > 0 )) && percent=$(( done * 100 / total ))
    printf '%b%s%b%b%s%b  %s/%s  %b%s%%%b' \
        "$color" "$(htb_repeat "$filled" '█')" "$R" \
        "$DIM" "$(htb_repeat $((width-filled)) '░')" "$R" \
        "$done" "$total" \
        "$GRY" "$percent" "$R"
}

htb_bar_row() {
    printf '  %b%s%b%s' "$GRY" "$(htb_pad "$1" 18)" "$R" "$(htb_bar "$2" "$3" "$4")"
}

menu_stats() {
    htb_fetch v4_get "/user/info"
    local info="$REPLY_BODY"
    htb_api_ok "$info" || return
    local uid; uid="$(jf "$info" '.info.id')"
    local basic; basic="$(v4_get "/user/profile/basic/${uid}")"
    local machines; machines="$(v4_get "/user/profile/progress/machines/${uid}")"
    local challenges; challenges="$(v4_get "/user/profile/progress/challenges/${uid}")"
    local sherlocks; sherlocks="$(v4_get "/user/profile/progress/sherlocks/${uid}")"
    local fortresses; fortresses="$(v4_get "/user/profile/progress/fortress/${uid}")"
    local prolabs; prolabs="$(v4_get "/user/profile/progress/prolab/${uid}")"
    local dashboard; dashboard="$(v4_get '/user/dashboard')"
    local board; board="$(v4_get '/rankings/users')"

    local -a lines=(
        ""
        "  ${BLD}${WHT}$(jf "$basic" '.profile.name')${R}"
        ""
        "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t p_rank)" 18)" "$R" "$YLW" "$(jf "$basic" '.profile.rank')" "$R")"
        "$(printf '  %b%s%b#%s' "$GRY" "$(htb_pad "$(t stats_global_rank)" 18)" "$R" "$(jf "$basic" '.profile.ranking')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_points)" 18)" "$R" "$(jf "$basic" '.profile.points')")"
        "$(htb_bar_row "$(jf "$basic" '.profile.next_rank')" "$(jf "$basic" '.profile.current_rank_progress')" 100 "$GRN")"
        ""
        "  ${GRN}── $(t stats_machines)${R}"
        "$(htb_bar_row "$(t stats_solved)" "$(jf "$machines" '.profile.machine_owns.solved')" "$(jf "$machines" '.profile.machine_owns.total')" "$GRN")"
    )

    local name owned total
    while IFS=$'\t' read -r name owned total; do
        [[ -z "$name" ]] && continue
        lines+=("$(htb_bar_row "$name" "$owned" "$total" "$(htb_difficulty_color "$name")")")
    done < <(printf '%s' "$machines" | jq -r '.profile.machine_difficulties[]? | [.name, ((.owned_machines // 0)|tostring), ((.total_machines // 0)|tostring)] | @tsv' 2>/dev/null)

    lines+=("" "  ${PRP}── $(t stats_challenges)${R}")
    lines+=("$(htb_bar_row "$(t stats_solved)" "$(jf "$challenges" '.profile.challenge_owns.solved')" "$(jf "$challenges" '.profile.challenge_owns.total')" "$PRP")")
    while IFS=$'\t' read -r name owned total; do
        [[ -z "$name" || "$owned" == "0" ]] && continue
        lines+=("$(htb_bar_row "$name" "$owned" "$total" "$PRP")")
    done < <(printf '%s' "$challenges" | jq -r '[.profile.challenge_categories[]? | select(((.owned_flags // .owns) // 0) > 0)] | sort_by(-((.owned_flags // .owns) // 0)) | .[0:8][] | [.name, (((.owned_flags // .owns) // 0)|tostring), (((.total_flags // .total) // 0)|tostring)] | @tsv' 2>/dev/null)

    lines+=("" "  ${CYN}── $(t stats_sherlocks)${R}")
    lines+=("$(htb_bar_row "$(t stats_solved)" "$(jf "$sherlocks" '.profile.challenge_owns.solved')" "$(jf "$sherlocks" '.profile.challenge_owns.total')" "$CYN")")
    while IFS=$'\t' read -r name owned total; do
        [[ -z "$name" ]] && continue
        lines+=("$(htb_bar_row "$name" "$owned" "$total" "$CYN")")
    done < <(printf '%s' "$sherlocks" | jq -r '.profile.challenge_categories[]? | select((.owned_flags // 0) > 0) | [.name, ((.owned_flags // 0)|tostring), ((.total_flags // 0)|tostring)] | @tsv' 2>/dev/null)

    local started
    started="$(printf '%s' "$fortresses" | jq -r '[.profile.fortresses[]? | select((.owned_flags // 0) > 0)] | length' 2>/dev/null)"
    if [[ "$started" =~ ^[0-9]+$ && "$started" -gt 0 ]]; then
        lines+=("" "  ${RED}── $(t stats_fortress)${R}")
        while IFS=$'\t' read -r name owned total; do
            [[ -z "$name" ]] && continue
            lines+=("$(htb_bar_row "$name" "$owned" "$total" "$RED")")
        done < <(printf '%s' "$fortresses" | jq -r '.profile.fortresses[]? | select((.owned_flags // 0) > 0) | [.name, ((.owned_flags // 0)|tostring), ((.total_flags // 0)|tostring)] | @tsv' 2>/dev/null)
    fi

    started="$(printf '%s' "$prolabs" | jq -r '[.profile.prolabs[]? | select((.owned_flags // 0) > 0)] | length' 2>/dev/null)"
    if [[ "$started" =~ ^[0-9]+$ && "$started" -gt 0 ]]; then
        lines+=("" "  ${ORG}── $(t stats_prolabs)${R}")
        while IFS=$'\t' read -r name owned total; do
            [[ -z "$name" ]] && continue
            lines+=("$(htb_bar_row "$name" "$owned" "$total" "$ORG")")
        done < <(printf '%s' "$prolabs" | jq -r '.profile.prolabs[]? | select((.owned_flags // 0) > 0) | [.name, ((.owned_flags // 0)|tostring), ((.total_flags // 0)|tostring)] | @tsv' 2>/dev/null)
    fi

    local season_id; season_id="$(v4_get '/season/list' | jq -r '.data[]? | select(.active == true) | .id' 2>/dev/null | head -1)"
    if [[ -n "$season_id" ]]; then
        local rank; rank="$(v4_get "/season/user/rank/${season_id}")"
        local obtained; obtained="$(jf "$rank" '.data.total_season_flags.obtained')"
        local total_flags; total_flags="$(jf "$rank" '.data.total_season_flags.total')"
        [[ "$obtained" == "-" ]] && obtained=0
        [[ "$total_flags" == "-" ]] && total_flags=0
        local league; league="$(jf "$rank" '.data.league')"
        [[ "$league" == "-" ]] && league="→ $(jf "$rank" '.data.next_rank.title')"
        lines+=("" "  ${PRP}── $(t p_season)${R}")
        lines+=("$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t season_tier)" 18)" "$R" "$PRP" "$league" "$R")")
        lines+=("$(htb_bar_row "$(t season_flags)" "$obtained" "$total_flags" "$GRN")")
        lines+=("$(printf '  %b%s%b%b%s%b  %s' "$GRY" "$(htb_pad "$(t stats_streak)" 18)" "$R" "$GRN" "$(htb_repeat "$obtained" '▮')" "$R" "$obtained")")
        lines+=("$(printf '  %b%s%b%s' "$DIM" "$(htb_pad '' 18)" "$R" "$(t stats_streak_note)")")
    fi

    lines+=("" "  ${WHT}── $(t stats_leaderboard)${R}")
    lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t stats_online)" 18)" "$R" "$(jf "$dashboard" '.dashboard_players.online_players')")")
    local position points player
    while IFS=$'\t' read -r position player points; do
        [[ -z "$position" ]] && continue
        lines+=("$(printf '    %b#%-4s%b %b%-24s%b %b%s pts%b' "$YLW" "$position" "$R" "$WHT" "${player:0:24}" "$R" "$GRY" "$points" "$R")")
    done < <(printf '%s' "$board" | jq -r '.data[0:5][]? | [((.rank // 0)|tostring), .name, ((.points // 0)|tostring)] | @tsv' 2>/dev/null)
    lines+=("$(printf '    %b#%-4s%b %b%-24s%b %b%s pts%b' "$GRN" "$(jf "$basic" '.profile.ranking')" "$R" "$GRN" "$(jf "$basic" '.profile.name')" "$R" "$GRY" "$(jf "$basic" '.profile.points')" "$R")")

    htb_show_lines "$(t stats_title)" "$GRN" lines
}
