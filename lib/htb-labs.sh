#!/usr/bin/env bash

HTB_WEB="https://app.hackthebox.com"

labs_countdown() {
    local iso="$1"
    [[ -z "$iso" || "$iso" == "-" || "$iso" == "null" ]] && return 1
    local target now left
    target="$(date -u -d "${iso:0:19}" +%s 2>/dev/null)" || target="$(date -u -j -f '%Y-%m-%dT%H:%M:%S' "${iso:0:19}" +%s 2>/dev/null)" || return 1
    now="$(date -u +%s)"
    left=$(( target - now ))
    (( left <= 0 )) && return 1
    local d=$(( left / 86400 )) h=$(( (left % 86400) / 3600 )) m=$(( (left % 3600) / 60 ))
    if (( d > 0 )); then printf '%dd %dh' "$d" "$h"; else printf '%dh %dm' "$h" "$m"; fi
}

labs_rating_text() {
    printf '%s' "$1" | jq -r '
        (.rating // 0) as $r | ((.ratingCount // .rating_count // 0)|tonumber) as $c
        | if $c == 0 then "-" else "★" + (($r|tonumber*10|round/10)|tostring) + " (" + ($c|tostring) + ")" end' 2>/dev/null
}

labs_labels_text() {
    printf '%s' "$1" | jq -r '
        ([ (.labels // [] | if type=="array" then .[] else . end | .name // empty | ascii_upcase) ]
         + (if ((.staff_pick // 0) == 1 or .staff_pick == true) then ["STAFF PICK"] else [] end))
        | join(" · ")' 2>/dev/null
}

labs_sort_fields() {
    case "$1" in
        machines) printf '%s\n' "|" "rating|$(t sort_rating)" "release_date|$(t sort_release)" "user_difficulty|$(t sort_user_difficulty)" "user_owns|$(t sort_user_owns)" "system_owns|$(t sort_system_owns)" "name|$(t sort_name)" ;;
        *)        printf '%s\n' "|" "rating|$(t sort_rating)" "release_date|$(t sort_release)" "solves|$(t sort_solves)" "name|$(t sort_name)" ;;
    esac
}

labs_sort_query() {
    local field="$1" desc="$2"
    [[ -z "$field" ]] && return
    printf '&sort_by=%s&sort_type=%s' "$field" "$([[ "$desc" == "1" ]] && echo desc || echo asc)"
}

labs_sort_label() {
    local kind="$1" field="$2" desc="$3" pair
    [[ -z "$field" ]] && return
    while IFS= read -r pair; do
        if [[ "${pair%%|*}" == "$field" ]]; then
            printf '%s %s %s' "$(t act_sort)" "${pair#*|}" "$([[ "$desc" == "1" ]] && echo '▼' || echo '▲')"
            return
        fi
    done < <(labs_sort_fields "$kind")
}

labs_sort_next() {
    local kind="$1" field="$2"
    local -a fields=()
    local pair
    while IFS= read -r pair; do fields+=("${pair%%|*}"); done < <(labs_sort_fields "$kind")
    local i
    for i in "${!fields[@]}"; do
        if [[ "${fields[$i]}" == "$field" ]]; then
            printf '%s' "${fields[$(( (i + 1) % ${#fields[@]} ))]}"
            return
        fi
    done
    printf '%s' "${fields[1]}"
}

labs_state_opts() {
    printf '%s\n' "$(t state_all)|" "$(t state_active)|active" "$(t state_retired)|retired" "$(t state_unreleased)|unreleased" "$(t state_favorites)|favorites"
}

labs_completed_opts() {
    printf '%s\n' "$(t filter_any)|" "$(t filter_solved)|complete" "$(t filter_unsolved)|incomplete"
}

labs_favorite() {
    local kind="$1" id="$2"
    local -a paths=()
    case "$kind" in
        machine)   paths=("/machine/todo/update/${id}") ;;
        challenge) paths=("/challenge/todo/update/${id}") ;;
        *)         paths=("/sherlock/todo/update/${id}" "/sherlocks/todo/update/${id}" "/sherlocks/${id}/todo") ;;
    esac
    local p result code
    echo
    for p in "${paths[@]}"; do
        result="$(v4_post "$p" "{}")"
        code="$(printf '%s' "$result" | jq -r '.message // ""' 2>/dev/null)"
        if ! printf '%s' "$code" | grep -qi "could not be found\|not supported"; then
            break
        fi
        result=""
    done
    if [[ -z "$result" ]]; then
        htb_note "$(t not_in_api)"; htb_enter; return
    fi
    if printf '%s' "$result" | jq -e --arg id "$id" '((.info // .data // []) | map(.id // . | tostring) | index($id)) != null' >/dev/null 2>&1; then
        htb_ok "$(t favorite_added)"
    else
        local msg; msg="$(jf "$result" '.message')"
        if [[ "$msg" != "-" ]] && ! printf '%s' "$result" | jq -e '.info // .data' >/dev/null 2>&1; then
            htb_fail "$msg"
        else
            htb_ok "$(t favorite_removed)"
        fi
    fi
    htb_enter
}

labs_reviews() {
    local kind="$1" id="$2" title="$3"
    htb_fetch v4_get "/review/${kind}/${id}/paginated?sort_by[]=created_at&sort_type=desc&per_page=40"
    local raw="$REPLY_BODY"
    local -a lines=("")
    local total; total="$(jf "$raw" '.meta.total')"
    if [[ "$(printf '%s' "$raw" | jq -r '.data | length' 2>/dev/null)" == "0" || -z "$raw" ]]; then
        lines+=("  ${DIM}$(t reviews_none)${R}")
    else
        lines+=("  ${BLD}${WHT}$(t reviews_title) · ${total}${R}" "")
        local stars head user when msg helpful
        while IFS=$'\t' read -r stars head user when msg helpful; do
            [[ -z "$stars" ]] && continue
            local full="" i
            for (( i=0; i<5; i++ )); do (( i < stars )) && full+="★" || full+="☆"; done
            lines+=("  ${YLW}${full}${R} ${WHT}${head}${R}   ${DIM}${user} · ${when:0:10}${R}")
            [[ -n "$msg" && "$msg" != "-" ]] && while IFS= read -r l; do lines+=("${DIM}${l}${R}"); done < <(htb_wrap_text "$msg")
            (( helpful > 0 )) && lines+=("    ${DIM}$(t reviews_helpful) ${helpful}${R}")
            lines+=("")
        done < <(printf '%s' "$raw" | jq -r '.data[]? | [((.stars // 0)|tostring), (.title // "-"), (.user.name // "-"), (.created_at // ""), ((.message // "-")|gsub("[\\n\\t]";" ")), ((.helpful_reviews_count // 0)|tostring)] | @tsv' 2>/dev/null)
    fi
    htb_show_lines "$(t reviews_title) · ${title}" "$CYN" lines
}

labs_activity() {
    local kind="$1" id="$2" title="$3"
    htb_fetch v4_get "/${kind}/activity/${id}"
    local -a lines=("")
    local user type blood when
    while IFS=$'\t' read -r user type blood when; do
        [[ -z "$user" ]] && continue
        local what color
        if [[ -n "$blood" && "$blood" != "null" ]]; then what="${blood} blood"; color="$RED"
        elif [[ "$type" == "blood" ]]; then what="blood"; color="$RED"
        elif [[ "$type" == "own" ]]; then what="solved"; color="$GRN"
        elif [[ "$type" == "root" || "$type" == "system" ]]; then what="${type} flag"; color="$GRN"
        else what="${type} flag"; color="$YLW"; fi
        lines+=("$(printf '  %b%-24s%b %b%-14s%b %b%s%b' "$WHT" "${user:0:24}" "$R" "$color" "$what" "$R" "$DIM" "$when" "$R")")
    done < <(printf '%s' "$REPLY_BODY" | jq -r '(.info.activity // .data // [])[:80][]? | [(.user_name // "-"), (.type // "-"), (.blood_type // ""), (.date_diff // "")] | @tsv' 2>/dev/null)
    (( ${#lines[@]} == 1 )) && lines+=("  ${DIM}$(t activity_none)${R}")
    htb_show_lines "$(t activity_title) · ${title}" "$CYN" lines
}

labs_changelog() {
    local kind="$1" id="$2" title="$3"
    htb_fetch v4_get "/${kind}/changelog/${id}"
    local -a lines=("")
    local when type head desc
    while IFS=$'\t' read -r when type head desc; do
        [[ -z "$when$head" ]] && continue
        case "$type" in 1|enhancement) type="ENHANCEMENT" ;; 2|info) type="INFO" ;; 3|bug) type="BUG" ;; esac
        lines+=("  ${DIM}${when:0:10}${R}  ${YLW}[${type}]${R} ${WHT}${head}${R}")
        [[ -n "$desc" && "$desc" != "-" ]] && while IFS= read -r l; do lines+=("${DIM}${l}${R}"); done < <(htb_wrap_text "$desc")
        lines+=("")
    done < <(printf '%s' "$REPLY_BODY" | jq -r '(.info // .data // [])[]? | [(.created_at // ""), ((.type // "")|tostring), (.title // "-"), ((.description // "-")|gsub("[\\n\\t]";" "))] | @tsv' 2>/dev/null)
    (( ${#lines[@]} == 1 )) && lines+=("  ${DIM}$(t changelog_none)${R}")
    htb_show_lines "$(t changelog_title) · ${title}" "$CYN" lines
}

labs_walkthroughs() {
    local id="$1" title="$2"
    htb_fetch v4_get "/machine/walkthroughs/${id}"
    local raw="$REPLY_BODY"
    local -a labels=() urls=()
    if [[ "$(jf "$raw" '.message.official.filename')" != "-" ]]; then
        labels+=("$(t walkthrough_official)  (PDF)"); urls+=("official")
    fi
    local yt; yt="$(jf "$raw" '.message.video.youtube_id')"
    if [[ "$yt" != "-" ]]; then
        labels+=("$(t walkthrough_video)  $(jf "$raw" '.message.video.creator_name')  youtube"); urls+=("https://www.youtube.com/watch?v=${yt}")
    fi
    local user rating votes when lang url
    while IFS=$'\t' read -r user rating votes when lang url; do
        [[ -z "$url" ]] && continue
        labels+=("$(printf '%-18s %3s%% (%s)  %s  %s' "${user:0:18}" "$rating" "$votes" "${when:0:10}" "$lang")"); urls+=("$url")
    done < <(printf '%s' "$raw" | jq -r '(.message.writeups // [])[:60][]? | [(.user_name // "-"), ((.rating // 0)|tostring), ((.total_ratings // 0)|tostring), (.created_at // ""), (.language_name // ""), (.url // "")] | @tsv' 2>/dev/null)
    if (( ${#labels[@]} == 0 )); then
        echo; htb_note "$(t walkthrough_none)"; htb_enter; return
    fi
    local at=0
    while true; do
        htb_pick "$(t walkthroughs_title) · ${title}" "$CYN" "$(htb_keys "↵|$(t open_in_browser)")" labels "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                local url="${urls[$REPLY_SEL]}"
                if [[ "$url" == "official" ]]; then
                    url="$(jf "$(v4_get "/machine/writeup/${id}")" '.url')"
                    if [[ "$url" == "-" ]]; then echo; htb_fail "$(jf "$(v4_get "/machine/writeup/${id}")" '.message')"; htb_enter; continue; fi
                fi
                htb_show_url "$url" ;;
        esac
    done
}

labs_machine_info() {
    local m="$1" id="$2"
    local name; name="$(jf "$m" '.name')"
    local -a lines=("" "  ${BLD}${WHT}${name}${R}" "")
    local synopsis; synopsis="$(jf "$m" '.synopsis')"
    if [[ "$synopsis" != "-" ]]; then
        while IFS= read -r l; do lines+=("${DIM}${l}${R}"); done < <(htb_wrap_text "$synopsis")
        lines+=("")
    fi
    local ub; ub="$(jf "$m" '.userBlood.user.name')"
    local rb; rb="$(jf "$m" '.rootBlood.user.name')"
    if [[ "$ub" != "-" || "$rb" != "-" ]]; then
        lines+=("  ${CYN}── $(t info_bloods)${R}")
        [[ "$ub" != "-" ]] && lines+=("$(printf '  %b%-6s%b%b%-24s%b %b%s%b' "$GRY" "user" "$R" "$RED" "$ub" "$R" "$DIM" "$(jf "$m" '.userBlood.created_at' | cut -c1-10) · $(jf "$m" '.userBlood.blood_difference')" "$R")")
        [[ "$rb" != "-" ]] && lines+=("$(printf '  %b%-6s%b%b%-24s%b %b%s%b' "$GRY" "root" "$R" "$RED" "$rb" "$R" "$DIM" "$(jf "$m" '.rootBlood.created_at' | cut -c1-10) · $(jf "$m" '.rootBlood.blood_difference')" "$R")")
        lines+=("")
    fi
    local tags; tags="$(v4_get "/machine/tags/${id}")"
    local cat names
    local have_tags=0
    while IFS=$'\t' read -r cat names; do
        [[ -z "$cat" ]] && continue
        (( have_tags )) || { lines+=("  ${CYN}── $(t info_tags)${R}"); have_tags=1; }
        lines+=("$(printf '  %b%-18s%b%b%s%b' "$GRY" "${cat:0:18}" "$R" "$PRP" "$names" "$R")")
    done < <(printf '%s' "$tags" | jq -r '(.info // []) | group_by(.category) | .[] | [(.[0].category // "-"), (map(.name) | join(", "))] | @tsv' 2>/dev/null)
    (( have_tags )) && lines+=("")
    local matrix; matrix="$(v4_get "/machine/graph/matrix/${id}")"
    if printf '%s' "$matrix" | jq -e '.info.aggregate' >/dev/null 2>&1; then
        lines+=("  ${CYN}── $(t info_matrix)${R}")
        local key val width bar j
        for key in enum real cve custom ctf; do
            val="$(printf '%s' "$matrix" | jq -r ".info.aggregate.${key} // 0" 2>/dev/null)"
            width="$(printf '%s' "$val" | awk '{v=$1; if(v<0)v=0; if(v>10)v=10; printf "%d", v*3+0.5}')"
            bar=""; for (( j=0; j<width; j++ )); do bar+="█"; done
            lines+=("$(printf '  %b%-8s%b%b%s%b %b%s%b' "$GRY" "$key" "$R" "$GRN" "$bar" "$R" "$DIM" "$val" "$R")")
        done
        lines+=("")
    fi
    local status; status="$(jf "$m" '.info_status')"
    [[ "$status" != "-" ]] && while IFS= read -r l; do lines+=("${DIM}${l}${R}"); done < <(htb_wrap_text "$status")
    htb_show_lines "$(t info_title) · ${name}" "$CYN" lines
}

labs_live_ip() {
    local id="$1"
    local act; act="$(v4_get '/machine/active')"
    [[ "$(jf "$act" '.info.id')" == "$id" ]] || return 1
    REPLY_LIVE_IP="$(jf "$act" '.info.ip')"
    REPLY_LIVE_EXPIRES="$(jf "$act" '.info.expires_at')"
    return 0
}

labs_global_search() {
    clear; htb_title "$(t machines_global_search)" "$WHT"; echo
    htb_prompt "$(t ask_search)"
    local term="$REPLY_TEXT"
    [[ -z "$term" ]] && return
    htb_fetch v4_get "/search/fetch?query=$(urlencode "$term")"
    local raw="$REPLY_BODY"
    local -a rows=() kinds=() ids=()
    local kind label id value sp
    for kind in machines challenges users teams; do
        case "$kind" in
            machines) label="$(t search_machines)" ;; challenges) label="$(t search_challenges)" ;;
            users) label="$(t search_users)" ;; teams) label="$(t search_teams)" ;;
        esac
        while IFS=$'\t' read -r id value sp; do
            [[ -z "$id" ]] && continue
            rows+=("$(printf '%b%-10s%b %b%s%b%s' "$GRY" "$label" "$R" "$WHT" "$value" "$R" "$([[ "$sp" == "true" ]] && echo '  Starting Point')")")
            kinds+=("$kind"); ids+=("$id")
        done < <(printf '%s' "$raw" | jq -r --arg k "$kind" '(.[$k] // [])[]? | [(.id|tostring), (.value // "-"), ((.isSp // false)|tostring)] | @tsv' 2>/dev/null)
    done
    if (( ${#rows[@]} == 0 )); then echo; htb_note "$(t no_results)"; htb_enter; return; fi
    local at=0
    while true; do
        htb_pick "$(t machines_global_search): ${term}" "$GRN" "" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                case "${kinds[$REPLY_SEL]}" in
                    machines)   machine_details "${ids[$REPLY_SEL]}" ;;
                    challenges) challenge_details "${ids[$REPLY_SEL]}" ;;
                    users)      labs_user_page "${ids[$REPLY_SEL]}" ;;
                    teams)      htb_show_url "${HTB_WEB}/teams/${ids[$REPLY_SEL]}" ;;
                esac ;;
        esac
    done
}

labs_user_page() {
    local uid="$1"
    htb_fetch v4_get "/user/profile/basic/${uid}"
    local b="$REPLY_BODY"
    local name; name="$(jf "$b" '.profile.name')"
    if [[ "$name" == "-" ]]; then echo; htb_fail "$(t api_bad_response)"; htb_enter; return; fi
    local -a lines=(
        ""
        "  ${BLD}${WHT}${name}${R}"
        ""
        "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t p_rank)" 18)" "$R" "$YLW" "$(jf "$b" '.profile.rank')" "$R")"
        "$(printf '  %b%s%b#%s' "$GRY" "$(htb_pad "$(t stats_global_rank)" 18)" "$R" "$(jf "$b" '.profile.ranking')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_points)" 18)" "$R" "$(jf "$b" '.profile.points')")"
        "$(printf '  %b%s%buser %s · system %s' "$GRY" "$(htb_pad "$(t p_owns)" 18)" "$R" "$(jf "$b" '.profile.user_owns')" "$(jf "$b" '.profile.system_owns')")"
        "$(printf '  %b%s%buser %s · system %s' "$GRY" "$(htb_pad "$(t p_bloods)" 18)" "$R" "$(jf "$b" '.profile.user_bloods')" "$(jf "$b" '.profile.system_bloods')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_respects)" 18)" "$R" "$(jf "$b" '.profile.respects')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_country)" 18)" "$R" "$(jf "$b" '.profile.country_name')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t p_team)" 18)" "$R" "$(jf "$b" '.profile.team.name')")"
        ""
        "  ${DIM}${HTB_WEB}/users/${uid}${R}"
    )
    htb_show_lines "$(t user_view_title) · ${name}" "$CYN" lines
}

labs_dashboard() {
    local which="$1" title="$2"
    htb_fetch v5_get "/user/dashboard/${which}"
    local raw="$REPLY_BODY"
    local -a rows=() kinds=() ids=()
    local key kind id name os diff progress done total user root
    for key in machines startingPoints sherlocks challenges; do
        case "$key" in sherlocks) kind="sherlock" ;; challenges) kind="challenge" ;; *) kind="machine" ;; esac
        while IFS=$'\t' read -r id name os diff progress done total user root; do
            [[ -z "$id" ]] && continue
            local mark="  " mc="$DIM"
            if [[ "$user" == "true" && "$root" == "true" ]]; then mark="✓✓"; mc="$GRN"; elif [[ "$user" == "true" || "$root" == "true" ]]; then mark="✓ "; mc="$YLW"; fi
            [[ "$os" == "-" ]] && os=""
            rows+=("$(printf '%b%s%b %b%-9s%b%b%-28s%b %b%-9s%b%b%-10s%b %b%s %s%% · %s %s/%s%b' "$mc" "$mark" "$R" "$GRY" "$([[ "$key" == startingPoints ]] && echo sp || echo "$kind")" "$R" "$WHT" "${name:0:28}" "$R" "$CYN" "${os:0:9}" "$R" "$(htb_difficulty_color "$diff")" "${diff:0:10}" "$R" "$DIM" "$(t f_progress)" "${progress%.*}" "$(t f_tasks)" "$done" "$total" "$R")")
            kinds+=("$kind"); ids+=("$id")
        done < <(printf '%s' "$raw" | jq -r --arg k "$key" '(.[$k] // [])[]? | [(.id|tostring), (.name // "-"), (.os // "-"), (.difficulty // "-"), ((.progress // 0)|tostring), ((.tasksCompleted // 0)|tostring), ((.tasksTotal // 0)|tostring), ((.user_flag // false)|tostring), ((.root_flag // false)|tostring)] | @tsv' 2>/dev/null)
    done
    if (( ${#rows[@]} == 0 )); then echo; htb_note "$(t dashboard_empty)"; htb_enter; return; fi
    local at=0
    while true; do
        htb_pick "$title" "$GRN" "" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                case "${kinds[$REPLY_SEL]}" in
                    machine)   machine_details "${ids[$REPLY_SEL]}" ;;
                    sherlock)  sherlock_details "${ids[$REPLY_SEL]}" ;;
                    challenge) challenge_details "${ids[$REPLY_SEL]}" ;;
                esac ;;
        esac
    done
}

labs_season_leaderboard() {
    local sid="$1" sname="$2"
    local me; me="$(v4_get "/season/user/rank/${sid}")"
    local my_rank; my_rank="$(jf "$me" '.data.rank')"
    local page=1
    while true; do
        htb_fetch v4_get "/season/players/leaderboard?per_page=15&season=${sid}&page=${page}"
        local raw="$REPLY_BODY"
        local last; last="$(jf "$raw" '.meta.last_page')"; [[ "$last" =~ ^[0-9]+$ ]] || last=1
        local -a lines=("" "$(printf '  %b%-6s%-24s%-10s%7s%7s%7s%7s%b' "$GRY" "#" "" "tier" "pts" "user" "root" "bloods" "$R")")
        local rank name tier pts uo ro bl
        while IFS=$'\t' read -r rank name tier pts uo ro bl; do
            [[ -z "$rank" ]] && continue
            local c="$WHT" rc="$YLW"
            [[ "$rank" == "$my_rank" ]] && { c="$GRN"; rc="$GRN"; }
            lines+=("$(printf '  %b%-6s%b%b%-24s%b%b%-10s%b%7s%b%7s%7s%b%b%7s%b' "$rc" "$rank" "$R" "$c" "${name:0:22}" "$R" "$PRP" "$tier" "$R" "$pts" "$DIM" "$uo" "$ro" "$R" "$RED" "$bl" "$R")")
        done < <(printf '%s' "$raw" | jq -r '.data[]? | [(.rank|tostring), (.name // "-"), (.league_rank // "-"), ((.points // 0)|tostring), ((.user_owns // 0)|tostring), ((.root_owns // 0)|tostring), (((.user_bloods // 0) + (.root_bloods // 0))|tostring)] | @tsv' 2>/dev/null)
        if [[ "$my_rank" != "-" ]]; then
            lines+=("" "  ${GRN}#${my_rank}${R}  $(t season_you) · $(jf "$me" '.data.league') · $(jf "$me" '.data.total_season_points') $(t season_points) · $(jf "$me" '.data.total_season_flags.obtained') $(t season_flags)")
        fi
        lines+=("" "$(htb_keys "n|$(t page_next)" "p|$(t page_previous)")")
        clear; htb_title "$(t season_leaderboard) · ${sname} · $(t act_page) ${page}/${last}" "$PRP"; echo
        local l; for l in "${lines[@]}"; do printf '%b\n' "$l"; done
        htb_read_key
        case "$REPLY_KEYNAME" in
            CHAR:n|CHAR:N) (( page < last )) && ((page++)) ;;
            CHAR:p|CHAR:P) (( page > 1 )) && ((page--)) ;;
            BACK|QUIT|ESC|LEFT|CHAR:q|CHAR:Q) return ;;
        esac
    done
}

labs_season_rewards() {
    local sid="$1" sname="$2"
    htb_fetch v4_get "/season/rewards/${sid}"
    local -a lines=("")
    local l
    while IFS= read -r l; do lines+=("$l"); done < <(printf '%s' "$REPLY_BODY" | jq -r --arg c "$CYN" --arg p "$PRP" --arg g "$GRY" --arg d "$DIM" --arg r "$R" --arg flags "$(t season_flags)" '
        (.data // [])[] | (.reward_types // []) | (if type=="array" then .[] else . end)
        | "  " + $c + "── " + (.name // "-") + $r,
          (if .description then "  " + $d + (.description|gsub("[\\n]";" ")) + $r else empty end),
          ((.groups // [])[] | "  " + $p + ((.name // "-") + "            ")[0:12] + $r + $g + ((.subtitle // "") + "                ")[0:16] + $r + $d + (if .rank_up_requirement then "top " + (.rank_up_requirement|tostring) + "  " else "" end) + (if .flags_needed then (.flags_needed|tostring) + " " + $flags else "" end) + $r),
          ""' 2>/dev/null)
    (( ${#lines[@]} <= 1 )) && lines+=("  ${DIM}$(t no_results)${R}")
    htb_show_lines "$(t season_rewards) · ${sname}" "$PRP" lines
}

labs_rankings() {
    local kind="$1" title="$2"
    htb_fetch v4_get "/rankings/${kind}"
    local -a lines=("" "$(printf '  %b%-6s%-28s%9s%10s%8s%8s%b' "$GRY" "#" "" "$([[ "$kind" == countries ]] && echo members || { [[ "$kind" == universities ]] && echo students; })" "pts" "user" "root" "$R")")
    local rank name third pts uo ro
    while IFS=$'\t' read -r rank name third pts uo ro; do
        [[ -z "$rank" ]] && continue
        lines+=("$(printf '  %b%-6s%b%b%-28s%b%b%9s%b%b%10s%b%b%8s%8s%b' "$YLW" "$rank" "$R" "$WHT" "${name:0:26}" "$R" "$DIM" "$third" "$R" "$GRN" "$pts" "$R" "$DIM" "$uo" "$ro" "$R")")
    done < <(printf '%s' "$REPLY_BODY" | jq -r --arg k "$kind" '(.data // [])[:150][]? | [(.rank|tostring), (if $k=="countries" then ((.country // "") + (if .country_name then "  " + .country_name else "" end)) else (.name // "-") end), (if $k=="countries" then ((.members // "")|tostring) elif $k=="universities" then ((.students // "")|tostring) else "" end), ((.points // 0)|tostring), ((.user_owns // 0)|tostring), ((.root_owns // 0)|tostring)] | @tsv' 2>/dev/null)
    (( ${#lines[@]} <= 2 )) && lines+=("  ${RED}$(t api_bad_response)${R}")
    htb_show_lines "$title" "$CYN" lines
}

labs_pwnbox() {
    htb_fetch v4_get "/pwnbox/status"
    local status="$REPLY_BODY"
    local usage; usage="$(v4_get '/pwnbox/usage')"
    local -a lines=("")
    local st; st="$(jf "$status" '.status')"
    if [[ "$st" == "-" ]]; then
        lines+=("  ${DIM}$(t pwnbox_none)${R}" "  ${DIM}$(jf "$status" '.message')${R}")
    else
        lines+=("$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t vm_label)" 18)" "$R" "$GRN" "$st" "$R")")
        local key v
        for key in hostname vnc_view_only_password vnc_password expires_at location; do
            v="$(jf "$status" ".${key}")"
            [[ "$v" != "-" ]] && lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$key" 18)" "$R" "$v")")
        done
    fi
    lines+=("" "$(printf '  %b%s%b%s / %s' "$GRY" "$(htb_pad "$(t pwnbox_usage)" 28)" "$R" "$(jf "$usage" '.used')" "$(jf "$usage" '.allowed')")")
    lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "sessions" 28)" "$R" "$(jf "$usage" '.sessions')")")
    htb_show_lines "$(t pwnbox_labs)" "$PRP" lines
}
