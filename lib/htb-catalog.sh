#!/usr/bin/env bash

menu_catalog() {
    local -a items=(
        "$(htb_row "$(t catalog_challenges)" "$(t catalog_challenges_desc)")"
        "$(htb_row "$(t catalog_prolabs)" "$(t catalog_prolabs_desc)")"
        "$(htb_row "$(t catalog_fortress)" "$(t catalog_fortress_desc)")"
        "$(htb_row "$(t catalog_sherlocks)" "$(t catalog_sherlocks_desc)")"
        "$(htb_row "$(t catalog_tracks)" "$(t catalog_tracks_desc)")"
        "$(htb_row "$(t catalog_seasons)" "$(t catalog_seasons_desc)")"
    )
    local at=0
    while true; do
        htb_pick "$(t catalog_title)" "$CYN" "" items "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                case "$REPLY_SEL" in
                    0) catalog_challenges ;;
                    1) catalog_prolabs ;;
                    2) catalog_fortress ;;
                    3) catalog_sherlocks ;;
                    4) catalog_tracks ;;
                    5) catalog_seasons ;;
                esac ;;
        esac
    done
}

declare -gA CHALLENGE_CATEGORY

declare -A SHERLOCK_CATEGORY

_load_challenge_categories() {
    (( ${#CHALLENGE_CATEGORY[@]} > 0 )) && return
    local id name
    while IFS=$'\t' read -r id name; do
        [[ -n "$id" ]] && CHALLENGE_CATEGORY["$id"]="$name"
    done < <(v4_get "/challenge/categories/list" | jq -r '.info[]? | [(.id|tostring), .name] | @tsv' 2>/dev/null)
}

_load_sherlock_categories() {
    (( ${#SHERLOCK_CATEGORY[@]} > 0 )) && return
    local id name
    while IFS=$'\t' read -r id name; do
        [[ -n "$id" ]] && SHERLOCK_CATEGORY["$id"]="$name"
    done < <(v4_get "/sherlocks/categories/list" | jq -r '(.info // .data // [])[]? | [(.id|tostring), .name] | @tsv' 2>/dev/null)
}

catalog_challenges() { _content_list challenges; }

_content_list() {
    local kind="$1"
    local state="" keyword="" difficulty="" category="" completed="" sort_field="" sort_desc=1 page=1 last=1 total="?" reload=1 at=0
    local -a cdiff_opts=("$(t filter_any)|" "Very Easy|very-easy" "Easy|easy" "Medium|medium" "Hard|hard" "Insane|insane")
    local -a state_opts=() done_opts=()
    mapfile -t state_opts < <(labs_state_opts)
    mapfile -t done_opts < <(labs_completed_opts)
    local -a rows=() ids=()
    local title_base fav_kind
    if [[ "$kind" == "challenges" ]]; then
        _load_challenge_categories; title_base="$(t catalog_challenges)"; fav_kind="challenge"
    else
        _load_sherlock_categories; title_base="$(t catalog_sherlocks)"; fav_kind="sherlock"
    fi
    while true; do
        if (( reload )); then
            local qs="per_page=25&page=${page}"
            if [[ "$state" == "favorites" ]]; then qs+="&todo=1"; elif [[ -n "$state" ]]; then qs+="&state=${state}"; fi
            [[ -n "$keyword" ]] && qs+="&keyword=$(urlencode "$keyword")"
            [[ -n "$difficulty" ]] && qs+="&difficulty[]=$(urlencode "$difficulty")"
            [[ -n "$category" ]] && qs+="&category[]=$(urlencode "$category")"
            [[ -n "$completed" ]] && qs+="&showCompleted=${completed}"
            qs+="$(labs_sort_query content "$sort_field" "$sort_desc")"
            htb_fetch v4_get "/${kind}?${qs}"
            local raw="$REPLY_BODY"
            htb_api_ok "$raw" || return
            total="$(jf "$raw" '.meta.total')"
            last="$(jf "$raw" '.meta.last_page')"; [[ "$last" =~ ^[0-9]+$ ]] || last=1
            rows=(); ids=()
            local id done fav name cat diff solves rating labels progress released cstate
            while IFS=$'\t' read -r id done fav name cat diff solves rating labels progress released cstate; do
                [[ -z "$id" ]] && continue
                local mark=" " mc="$DIM"; [[ "$done" == "true" ]] && { mark="✓"; mc="$GRN"; }
                [[ "$fav" == "true" ]] && fav="★ " || fav="  "
                local tail="$(t f_solves) ${solves}"
                [[ "$kind" == "sherlocks" ]] && tail="$(t f_progress) ${progress%.*}% · ${tail}"
                local extra=""
                if [[ "$cstate" == unreleased* ]]; then
                    local left; left="$(labs_countdown "$released")" && extra=" · $(t info_countdown) ${left}"
                fi
                ids+=("$id")
                rows+=("$(printf '%b%s%b%b%s%b%b%-30s%b %b%-16s%b %b%-11s%b %b%-12s%b %b%s%b  %b%s · %s%s%b' \
                    "$mc" "$mark" "$R" "$YLW" "$fav" "$R" "$WHT" "${name:0:30}" "$R" "$PRP" "${cat:0:16}" "$R" \
                    "$(htb_difficulty_color "$diff")" "${diff:0:11}" "$R" "$PRP" "${rating:0:12}" "$R" "$GRN" "$labels" "$R" \
                    "$DIM" "$tail" "${released:0:10}" "$extra" "$R")")
            done < <(printf '%s' "$raw" | jq -r '(.data // .challenges // .sherlocks // [])[]? | [
                (.id|tostring), (((.is_owned // .authUserSolve // .isCompleted) // false)|tostring),
                (((.todo // .is_todo // .isTodo) // false)|tostring), (.name // "?"), (.category_name // "?"),
                (.difficulty // "?"), ((.solves // 0)|tostring),
                (((.rating // 0) as $r | ((.rating_count // 0)|tonumber) as $c | if $c == 0 then "-" else "★" + (($r|tonumber*10|round/10)|tostring) + " (" + ($c|tostring) + ")" end)),
                ([ (.labels // [] | if type=="array" then .[] else . end | .name // empty | ascii_upcase) ] | join(" · ")),
                ((.progress // 0)|tostring), ((.release_date // "")|tostring), ((.state // "")|tostring)
            ] | @tsv' 2>/dev/null)
            reload=0
        fi
        local header="${title_base} · ${total}"
        [[ -n "$state" ]] && header+=" · $(_filter_label "$state" "${state_opts[@]}")"
        [[ -n "$keyword" ]] && header+=" · \"${keyword}\""
        if [[ -n "$category" ]]; then
            if [[ "$kind" == "challenges" ]]; then header+=" · ${CHALLENGE_CATEGORY[$category]:-$category}"; else header+=" · ${SHERLOCK_CATEGORY[$category]:-$category}"; fi
        fi
        [[ -n "$difficulty" ]] && header+=" · $(_filter_label "$difficulty" "${cdiff_opts[@]}")"
        [[ -n "$completed" ]] && header+=" · $(_filter_label "$completed" "${done_opts[@]}")"
        local sl; sl="$(labs_sort_label content "$sort_field" "$sort_desc")"
        [[ -n "$sl" ]] && header+=" · ${sl}"
        header+=" · $(t act_page) ${page}/${last}"
        local hint; hint="$(htb_keys "t|$(t act_state)" "s|$(t act_search_server)" "c|$(t act_category)" "d|$(t act_filter_challenge_difficulty)" "v|$(t act_filter_solved)" "a|$(t act_sort)" "A|$(t act_sort_flip)" "m|$(t act_favorite)" "n|$(t page_next)" "p|$(t page_previous)" "x|$(t act_filter_clear)")"
        if (( ${#ids[@]} == 0 )); then
            local -a empty=("  ${DIM}$(t no_results)${R}")
            htb_pick "$header" "$CYN" "$hint" empty 0
        else
            htb_pick "$header" "$CYN" "$hint" rows "$at"
        fi
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                (( ${#ids[@]} == 0 )) && continue
                if [[ "$kind" == "challenges" ]]; then challenge_details "${ids[$REPLY_SEL]}"; else sherlock_details "${ids[$REPLY_SEL]}"; fi
                reload=1 ;;
            t|T) _pick_filter "$(t act_state)" "${state_opts[@]}"
                 (( REPLY_FILTER_SET )) && { state="$REPLY_FILTER"; page=1; at=0; reload=1; } ;;
            s|S) clear; htb_title "$(t act_search_server)" "$WHT"; echo; htb_prompt "$(t ask_search)"
                 keyword="$REPLY_TEXT"; page=1; at=0; reload=1 ;;
            c|C) if [[ "$kind" == "challenges" ]]; then _pick_challenge_category; else _pick_sherlock_category; fi
                 category="$REPLY_CATEGORY"; page=1; at=0; reload=1 ;;
            d|D) _pick_filter "$(t act_filter_challenge_difficulty)" "${cdiff_opts[@]}"
                 (( REPLY_FILTER_SET )) && { difficulty="$REPLY_FILTER"; page=1; at=0; reload=1; } ;;
            v|V) _pick_filter "$(t act_filter_solved)" "${done_opts[@]}"
                 (( REPLY_FILTER_SET )) && { completed="$REPLY_FILTER"; page=1; at=0; reload=1; } ;;
            a)   sort_field="$(labs_sort_next content "$sort_field")"; page=1; reload=1 ;;
            A)   sort_desc=$((1-sort_desc)); [[ -z "$sort_field" ]] && sort_field="rating"; page=1; reload=1 ;;
            m|M) (( ${#ids[@]} == 0 )) && continue; labs_favorite "$fav_kind" "${ids[$REPLY_SEL]}"; reload=1 ;;
            n|N) (( page < last )) && { ((page++)); at=0; reload=1; } ;;
            p|P) (( page > 1 )) && { ((page--)); at=0; reload=1; } ;;
            x|X) state=""; keyword=""; difficulty=""; category=""; completed=""; sort_field=""; sort_desc=1; page=1; at=0; reload=1 ;;
        esac
    done
}

_pick_sherlock_category() {
    local -a names=("$(t challenge_all_categories)") keys=("")
    local k
    for k in $(printf '%s\n' "${!SHERLOCK_CATEGORY[@]}" | sort -n); do
        names+=("${SHERLOCK_CATEGORY[$k]}")
        keys+=("$k")
    done
    REPLY_CATEGORY=""
    htb_pick "$(t act_category)" "$CYN" "" names
    [[ "$REPLY_KEY" == "enter" ]] && REPLY_CATEGORY="${keys[$REPLY_SEL]}"
}

_pick_challenge_category() {
    local -a names=("$(t challenge_all_categories)") keys=("")
    local k
    for k in $(printf '%s\n' "${!CHALLENGE_CATEGORY[@]}" | sort -n); do
        names+=("${CHALLENGE_CATEGORY[$k]}")
        keys+=("$k")
    done
    REPLY_CATEGORY=""
    htb_pick "$(t act_category)" "$CYN" "" names
    [[ "$REPLY_KEY" == "enter" ]] && REPLY_CATEGORY="${keys[$REPLY_SEL]}"
}

challenge_details() {
    local id="$1"
    while true; do
        htb_fetch v4_get "/challenge/info/${id}"
        local c; c="$(printf '%s' "$REPLY_BODY" | jq -c '.challenge // .data // .' 2>/dev/null)"
        local name; name="$(jf "$c" '.name')"
        local diff; diff="$(jf "$c" '.difficulty')"
        local downloadable; downloadable="$(jf "$c" '.download')"
        clear; htb_title "$(t catalog_challenges) · ${name}" "$CYN"; echo
        printf '%b\n\n' "  ${BLD}${WHT}${name}${R}"
        htb_field "$(t f_category)" "${PRP}$(jf "$c" '.category_name')${R}"
        htb_field "$(t f_difficulty)" "$(htb_difficulty_color "$diff")${diff}${R}"
        htb_field "$(t f_points)" "${YLW}$(jf "$c" '.points')${R}"
        htb_field "$(t f_solves)" "$(jf "$c" '.solves')"
        htb_field "$(t f_author)" "$(jf "$c" '.creator_name')"
        htb_field "$(t f_solved)" "$([[ "$(jf "$c" '.authUserSolve')" == "true" ]] && t yes || t no)"
        local crating; crating="$(labs_rating_text "$c")"
        [[ "$crating" != "-" && -n "$crating" ]] && htb_field "$(t f_rating)" "$crating"
        [[ "$(jf "$c" '.isTodo')" == "true" || "$(jf "$c" '.todo')" == "true" ]] && htb_field "$(t act_favorite)" "${YLW}★${R}"
        local description; description="$(jf "$c" '.description')"
        if [[ -n "$description" && "$description" != "-" ]]; then
            echo
            printf '%b\n' "${DIM}$(htb_wrap_text "$description")${R}"
        fi
        echo
        if [[ "$downloadable" == "true" ]]; then
            printf '%b\n' "    ${GRN}·${R} $(jf "$c" '.file_name') — $(t zip_password)"
        else
            printf '%b\n' "    ${DIM}· $(t no_files)${R}"
        fi
        local has_docker; has_docker="$(jf "$c" '.docker')"
        local container_ip; container_ip="$(jf "$c" '.play_info.ip')"
        local container_up=""
        [[ "$container_ip" != "-" && -n "$container_ip" ]] && container_up="1"
        local container_ports container_first_port container_target=""
        container_ports="$(printf '%s' "$c" | jq -r '(.play_info.ports // [])|join(", ")' 2>/dev/null)"
        container_first_port="$(printf '%s' "$c" | jq -r '(.play_info.ports // [])|first // empty' 2>/dev/null)"
        if [[ -n "$container_up" ]]; then
            if [[ -n "$container_first_port" ]]; then
                container_target="${container_ip}:${container_first_port}"
            else
                container_target="$container_ip"
            fi
        fi
        if [[ "$has_docker" == "true" ]]; then
            if [[ -n "$container_up" ]]; then
                printf '%b\n' "    ${GRN}·${R} $(t f_container): ${CYN}${container_target}${R} ${DIM}$(t vm_ports) ${container_ports} · $(t vm_needs_vpn_labs)${R}"
            else
                printf '%b\n' "    ${DIM}· $(t f_container_off): $(t vm_offline)${R}"
            fi
        fi
        echo; htb_rule
        local -a keys=()
        [[ "$downloadable" == "true" ]] && keys+=("d|$(t act_download)")
        if [[ "$has_docker" == "true" ]]; then
            if [[ -n "$container_up" ]]; then keys+=("t|$(t act_stop_lab)"); else keys+=("s|$(t act_start_lab)"); fi
        fi
        [[ -n "$container_up" ]] && keys+=("c|$(t act_copy)")
        keys+=("f|$(t act_flag)" "m|$(t act_favorite)" "o|$(t act_reviews)" "a|$(t act_activity)" "g|$(t act_changelog)" "←|$(t nav_back)")
        htb_keys "${keys[@]}"; echo
        htb_read_key
        case "$REPLY_KEYNAME" in
            CHAR:m|CHAR:M) labs_favorite challenge "$id" ;;
            CHAR:o|CHAR:O) labs_reviews challenge "$id" "$name" ;;
            CHAR:a) labs_activity challenge "$id" "$name" ;;
            CHAR:g|CHAR:G) labs_changelog challenge "$id" "$name" ;;
            CHAR:s|CHAR:S)
                [[ "$has_docker" == "true" && -z "$container_up" ]] || continue
                echo
                local result; result="$(v4_post "/container/start" "{\"containerable_id\":${id}}")"
                local message; message="$(jf "$result" '.message')"
                if printf '%s' "$message" | grep -qi "created\|started"; then htb_ok "$message"; else htb_fail "$message"; htb_report_plan "$message"; fi
                htb_enter ;;
            CHAR:t|CHAR:T)
                [[ "$has_docker" == "true" && -n "$container_up" ]] || continue
                echo
                local result; result="$(v4_post "/container/stop" "{\"containerable_id\":${id}}")"
                htb_ok "$(jf "$result" '.message')"
                htb_enter ;;
            CHAR:d|CHAR:D)
                if [[ "$downloadable" == "true" ]]; then
                    _download_file "${BASE_V4}/challenge/download/${id}" "challenge_${id}_$(_safe_name "$name").zip" 1
                else
                    echo; htb_note "$(t no_files)"; htb_enter
                fi ;;
            CHAR:c|CHAR:C)
                [[ -n "$container_up" ]] || continue
                if [[ -n "$container_first_port" ]]; then
                    htb_show_copied "${container_target}"$'\n'"http://${container_target}"
                else
                    htb_show_copied "$container_target"
                fi ;;
            CHAR:f|CHAR:F) challenge_flag "$id" ;;
            BACK|QUIT|CHAR:q|CHAR:Q) return ;;
        esac
    done
}

challenge_flag() {
    local id="$1"
    echo
    htb_prompt_flag "$(t ask_flag)"
    local flag="$REPLY_TEXT"
    [[ -z "$flag" ]] && return
    _pick_rating
    local rating="$REPLY_RATING"
    (( rating >= 1 && rating <= 10 )) || return
    local result; result="$(v4_post '/challenge/own' "{\"flag\":\"${flag}\",\"challenge_id\":${id},\"difficulty\":$((rating*10))}")"
    local message; message="$(jf "$result" '.message')"
    if [[ "$(htb_read_flag_answer "$message")" == "si" ]]; then
        printf '%b\n' "  ${GRN}${BLD}$(t flag_accepted)${R} — ${message}"
    else
        htb_fail "$message"
    fi
    htb_enter
}

_safe_name() { printf '%s' "$1" | tr -c 'A-Za-z0-9._-' '_'; }

_download_file() {
    local url="$1" filename="$2" encrypted="${3:-0}" password="${4:-}"
    local out="${DL_DIR}/${filename}"
    local code
    code="$(_htb_curl_auth | curl -s --config - -L --max-time 120 -o "$out" -w '%{http_code}' "$url")"
    chmod 600 "$out" 2>/dev/null
    local size; size="$(wc -c < "$out" 2>/dev/null || echo 0)"
    echo
    if [[ "$code" == "200" && "$size" -gt 100 ]]; then
        htb_ok "$(t saved_to) ${out} (${size} B)"
        if (( encrypted )); then
            if [[ -n "$password" && "$password" != "-" ]]; then
                htb_note "$(t zip_password_is) ${password}"
            else
                htb_note "$(t zip_password)"
            fi
        fi
    else
        rm -f "$out"
        htb_fail "$(t download_failed) (HTTP ${code})"
    fi
    htb_enter
}

catalog_prolabs() {
    htb_fetch v4_get "/prolabs"
    htb_api_ok "$REPLY_BODY" || return
    local labs; labs="$(printf '%s' "$REPLY_BODY" | jq -c '.data.labs // .data // []' 2>/dev/null)"
    local -a rows=() ids=() names=()
    local id name skill machines owned flags progress dcolor
    while IFS=$'\t' read -r id name skill machines owned flags progress; do
        [[ -z "$id" ]] && continue
        dcolor="$(htb_difficulty_color "$skill")"
        ids+=("$id"); names+=("$name")
        rows+=("$(printf '%b%-26s%b %b%-13s%b %3s %-9s %b%s/%s %s%b  %s%%' \
            "$WHT" "${name:0:26}" "$R" "$dcolor" "${skill:0:13}" "$R" \
            "$machines" "$(t f_machines)" "$GRN" "$owned" "$flags" "$(t f_flags)" "$R" "$progress")")
    done < <(printf '%s' "$labs" | jq -r '.[] | [
        (.id|tostring), .name, (.skill_level // "?"),
        ((.pro_machines_count // 0)|tostring), ((.pro_flags_owns_count // 0)|tostring),
        ((.pro_flags_count // 0)|tostring), ((.ownership // 0)|tostring)
    ] | @tsv' 2>/dev/null)
    if (( ${#ids[@]} == 0 )); then htb_fail "$(t no_results)"; htb_enter; return; fi
    local at=0
    while true; do
        htb_pick "$(t catalog_prolabs)" "$ORG" "" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) prolab_details "${ids[$REPLY_SEL]}" "${names[$REPLY_SEL]}" ;;
        esac
    done
}

prolab_details() {
    local id="$1" name="$2"
    htb_fetch v4_get "/prolab/${id}/info"
    local info="$REPLY_BODY"
    local progress; progress="$(v4_get "/prolab/${id}/progress")"
    local -a lines=(
        ""
        "  ${BLD}${WHT}${name}${R}"
        ""
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_version)" 18)" "$R" "$(jf "$info" '.data.version')")"
        "$(printf '  %b%s%b%s%%' "$GRY" "$(htb_pad "$(t f_progress)" 18)" "$R" "$(jf "$progress" '.data.ownership')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_entry)" 18)" "$R" "$(printf '%s' "$info" | jq -r '(.data.entry_points // [])|join(", ")' 2>/dev/null)")"
        ""
        "  ${GRN}── $(t f_machines)${R}"
    )
    local row
    while IFS= read -r row; do lines+=("$row"); done < <(v4_get "/prolab/${id}/machines" | jq -r '.data[]? | "    · \(.name) (\(.os))"' 2>/dev/null)
    lines+=("" "  ${GRN}── $(t f_flags)${R}")
    while IFS= read -r row; do lines+=("$row"); done < <(v4_get "/prolab/${id}/flags" | jq -r '.data[]? | "    · \(.title) — \(.points) pts \(if .owned then "✓" else "" end)"' 2>/dev/null)
    htb_show_lines "$name" "$ORG" lines
}

catalog_fortress() {
    htb_fetch v4_get "/fortresses"
    htb_api_ok "$REPLY_BODY" || return
    local forts; forts="$(printf '%s' "$REPLY_BODY" | jq -c '[.data[]?]' 2>/dev/null)"
    local -a rows=() ids=()
    local id name total detail owned points
    while IFS=$'\t' read -r id name total; do
        [[ -z "$id" ]] && continue
        detail="$(v4_get "/fortress/${id}")"
        owned="$(printf '%s' "$detail" | jq '[.data.flags[]? | select(.owned == true)] | length' 2>/dev/null)"
        points="$(jf "$detail" '.data.points')"
        ids+=("$id")
        rows+=("$(printf '%b%-26s%b %b%s/%s %s%b  %b%s pts%b' \
            "$WHT" "${name:0:26}" "$R" "$GRN" "${owned:-0}" "$total" "$(t f_flags)" "$R" "$YLW" "$points" "$R")")
    done < <(printf '%s' "$forts" | jq -r '.[] | [(.id|tostring), .name, ((.number_of_flags // 0)|tostring)] | @tsv' 2>/dev/null)
    if (( ${#ids[@]} == 0 )); then htb_fail "$(t no_results)"; htb_enter; return; fi
    local at=0
    while true; do
        htb_pick "$(t catalog_fortress)" "$RED" "" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) fortress_details "${ids[$REPLY_SEL]}" ;;
        esac
    done
}

fortress_details() {
    local id="$1"
    while true; do
        htb_fetch v4_get "/fortress/${id}"
        local d="$REPLY_BODY"
        local name; name="$(jf "$d" '.data.name')"
        local owned; owned="$(printf '%s' "$d" | jq '[.data.flags[]? | select(.owned == true)] | length' 2>/dev/null)"
        local total; total="$(printf '%s' "$d" | jq '.data.flags | length' 2>/dev/null)"
        clear; htb_title "$(t catalog_fortress) · ${name}" "$RED"; echo
        htb_field "$(t f_ip)" "${CYN}$(jf "$d" '.data.ip')${R}"
        htb_field "$(t f_points)" "${YLW}$(jf "$d" '.data.points')${R}"
        htb_field "$(t f_flags)" "${GRN}${owned:-0}/${total:-0}${R}"
        htb_field "$(t f_progress)" "$(jf "$d" '.data.progress_percent')%"
        local description; description="$(jf "$d" '.data.description')"
        if [[ -n "$description" && "$description" != "-" ]]; then
            echo
            printf '%b\n' "${DIM}$(htb_wrap_text "$description")${R}"
        fi
        echo
        printf '%s' "$d" | jq -r '.data.flags[]? | "    · \(.title) — \(.points) pts \(if .owned then "✓" else "" end)"' 2>/dev/null
        echo; htb_rule
        htb_keys "f|$(t act_flag)" "c|$(t act_copy)" "←|$(t nav_back)"; echo
        htb_read_key
        case "$REPLY_KEYNAME" in
            CHAR:f|CHAR:F)
                echo
                htb_prompt_flag "$(t ask_flag)"
                [[ -z "$REPLY_TEXT" ]] && continue
                local result; result="$(v4_post "/fortress/${id}/flag" "{\"flag\":\"${REPLY_TEXT}\"}")"
                local message; message="$(jf "$result" '.message')"
                if [[ "$(htb_read_flag_answer "$message")" == "si" ]]; then
                    printf '%b\n' "  ${GRN}${BLD}$(t flag_accepted)${R} — ${message}"
                else
                    htb_fail "$message"
                fi
                htb_enter ;;
            CHAR:c|CHAR:C) htb_show_copied "$(jf "$d" '.data.ip')" ;;
            BACK|QUIT|CHAR:q|CHAR:Q) return ;;
        esac
    done
}

catalog_sherlocks() { _content_list sherlocks; }

sherlock_details() {
    local id="$1"
    local reload=1 at=0
    local name password downloadable has_vm vm_up vm_ip container_ip
    local -a task_ids=() task_rows=() header=()

    while true; do
        if (( reload )); then
            htb_fetch v4_get "/sherlocks/${id}"
            local meta="$REPLY_BODY"
            local info; info="$(v4_get "/sherlocks/${id}/info")"
            local progress; progress="$(v4_get "/sherlocks/${id}/progress")"
            local play; play="$(v4_get "/sherlocks/${id}/play")"
            name="$(jf "$meta" '.data.name')"
            password="$(jf "$meta" '.data.file_password')"
            [[ -z "$password" || "$password" == "-" ]] && password="$SHERLOCK_ZIP_PASSWORD"
            downloadable="$(printf '%s' "$meta" | jq -r '[.data.play_methods[]?] | index("download") // empty' 2>/dev/null)"
            has_vm="$(printf '%s' "$meta" | jq -r '[.data.play_methods[]?] | index("vm") // empty' 2>/dev/null)"
            local vm_state; vm_state="$(jf "$play" '.data.play_info.status')"
            vm_ip="$(jf "$play" '.data.play_info.ip')"
            vm_up=""
            [[ "$vm_state" != "-" ]] && vm_up="1"
            local diff; diff="$(jf "$meta" '.data.difficulty')"
            local material=""
            if [[ -n "$downloadable" ]]; then
                if [[ "$password" == "-" || -z "$password" ]]; then
                    material="$(t zip_password)"
                else
                    material="$(t zip_password_is) ${password}"
                fi
            elif [[ -z "$has_vm" ]]; then
                material="$(t sherlock_no_material)"
            fi

            task_ids=(); task_rows=()
            local tid title question expected done_flag mark markcolor
            while IFS=$'\t' read -r tid title question expected done_flag; do
                [[ -z "$tid" ]] && continue
                if [[ "$done_flag" == "true" ]]; then mark="✓"; markcolor="$GRN"; else mark=" "; markcolor="$DIM"; fi
                task_ids+=("$tid")
                task_rows+=("$(printf '%b%s%b %b%-9s%b %b%-72s%b %b%s%b' \
                    "$markcolor" "$mark" "$R" \
                    "$GRY" "${title:0:9}" "$R" \
                    "$WHT" "${question:0:72}" "$R" \
                    "$DIM" "${expected:0:22}" "$R")")
            done < <(v4_get "/sherlocks/${id}/tasks" | jq -r '(.data // [])[]? | [
                (.id|tostring), (.title // "?"), (.description // ""),
                (.masked_flag // ""), ((.completed // false)|tostring)
            ] | @tsv' 2>/dev/null)

            local answered total
            answered="$(jf "$progress" '.data.tasks_answered')"
            total="$(jf "$progress" '.data.total_tasks')"
            [[ "$answered" == "-" ]] && answered=0
            [[ "$total" == "-" ]] && total="${#task_ids[@]}"

            header=(
                "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t f_category)" 18)" "$R" "$PRP" "$(jf "$meta" '.data.category_name')" "$R")"
                "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t f_difficulty)" 18)" "$R" "$(htb_difficulty_color "$diff")" "$diff" "$R")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_rating)" 18)" "$R" "$(jf "$meta" '.data.rating' | cut -c1-4)")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_state)" 18)" "$R" "$(jf "$meta" '.data.state')")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_released)" 18)" "$R" "$(jf "$meta" '.data.release_at' | cut -c1-10)")"
                "$(printf '  %b%s%b%s/%s %s' "$GRY" "$(htb_pad "$(t f_progress)" 18)" "$R" "$answered" "$total" "$(t sherlock_answered)")"
            )
            [[ -n "$material" ]] && header+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t sherlock_material)" 18)" "$R" "$material")")
            if [[ -n "$has_vm" ]]; then
                local vm_line
                if [[ -z "$vm_up" ]]; then
                    vm_line="${DIM}$(t vm_offline)${R}"
                elif [[ "$vm_ip" == "-" ]]; then
                    vm_line="${YLW}$(t vm_deploying)${R}"
                else
                    vm_line="${CYN}${vm_ip}${R}   ${DIM}$(t vm_needs_vpn)${R}"
                fi
                header+=("$(printf '  %b%s%b%b' "$GRY" "$(htb_pad "$(t vm_label)" 18)" "$R" "$vm_line")")
                local vm_user; vm_user="$(jf "$play" '.data.username')"
                local vm_pass; vm_pass="$(jf "$play" '.data.password')"
                if [[ "$vm_user" != "-" || "$vm_pass" != "-" ]]; then
                    header+=("$(printf '  %b%s%b%s %s   %s %s' "$GRY" "$(htb_pad "$(t vm_credentials)" 18)" "$R" "$(t vm_user)" "$vm_user" "$(t vm_password)" "$vm_pass")")
                fi
            fi
            [[ "$(jf "$meta" '.data.isTodo')" == "true" || "$(jf "$meta" '.data.todo')" == "true" ]] && header=("  ${YLW}★${R}" "${header[@]}")
            reload=0
        fi

        if (( ${#task_ids[@]} == 0 )); then
            htb_show_lines "$name" "$CYN" header
            return
        fi

        local -a keys=("↵|$(t action_answer)")
        [[ -n "$downloadable" ]] && keys+=("d|$(t act_download)")
        if [[ -n "$has_vm" ]]; then
            if [[ -n "$vm_up" ]]; then
                keys+=("t|$(t act_stop_lab)" "r|$(t act_reset)")
            else
                keys+=("s|$(t act_start_vm)")
            fi
        fi
        [[ -n "$has_vm" && "$vm_ip" != "-" ]] && keys+=("c|$(t act_copy)")
        keys+=("m|$(t act_favorite)" "o|$(t act_reviews)" "w|$(t act_walkthroughs)" "←|$(t nav_back)")
        htb_pick "$(t catalog_sherlocks) · ${name}" "$CYN" "$(htb_keys "${keys[@]}")" task_rows "$at" header
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            m|M) labs_favorite sherlock "$id"; reload=1 ;;
            o|O) labs_reviews sherlock "$id" "$name" ;;
            w|W)
                local wurl; wurl="$(v4_get "/sherlocks/${id}/writeup")"
                if [[ "$(jf "$wurl" '.url')" == "-" ]]; then echo; htb_fail "$(jf "$wurl" '.message')"; htb_enter; else htb_show_url "$(jf "$wurl" '.url')"; fi ;;
            enter) sherlock_answer "$id" "${task_ids[$REPLY_SEL]}" "${task_rows[$REPLY_SEL]}"; reload=1 ;;
            s|S)
                [[ -n "$has_vm" && -z "$vm_up" ]] || continue
                echo
                local result; result="$(v4_post "/challenge/start/vm/${id}")"
                local message; message="$(jf "$result" '.message')"
                if [[ "$(jf "$result" '.success')" == "true" ]]; then
                    htb_ok "$message"
                    htb_field "$(t vm_user)" "$(jf "$result" '.username')"
                    htb_field "$(t vm_password)" "$(jf "$result" '.password')"
                else
                    htb_fail "$message"
                    htb_report_plan "$message"
                fi
                htb_enter; reload=1 ;;
            t|T)
                [[ -n "$has_vm" && -n "$vm_up" ]] || continue
                echo
                local result; result="$(v4_post "/vm/terminate" "{\"machine_id\":${id}}")"
                htb_ok "$(jf "$result" '.message')"
                htb_enter; reload=1 ;;
            r|R)
                [[ -n "$has_vm" && -n "$vm_up" ]] || continue
                echo
                local result; result="$(v4_post "/challenge/reset/vm/${id}")"
                htb_ok "$(jf "$result" '.message')"
                htb_enter; reload=1 ;;
            c|C)
                [[ "$vm_ip" != "-" && -n "$vm_ip" ]] || continue
                htb_show_copied "${vm_ip}"$'\n'"xfreerdp /v:${vm_ip} /u:LetsDefend /p:'' /cert:ignore /dynamic-resolution" ;;
            d|D)
                if [[ -z "$downloadable" ]]; then
                    echo; htb_note "$(t sherlock_no_material)"; htb_enter
                else
                    local link; link="$(jf "$(v4_get "/sherlocks/${id}/download_link")" '.url')"
                    if [[ -z "$link" || "$link" == "-" ]]; then
                        echo; htb_note "$(t no_link)"; htb_enter
                    else
                        _download_file "$link" "sherlock_${id}_$(_safe_name "$name").zip" 1 "$password"
                    fi
                fi ;;
        esac
    done
}

sherlock_answer() {
    local sherlock="$1" task="$2"
    echo
    htb_prompt "$(t ask_answer)"
    local answer="$REPLY_TEXT"
    [[ -z "$answer" ]] && return
    local result; result="$(v4_post "/sherlocks/${sherlock}/tasks/${task}/flag" "{\"flag\":\"${answer}\"}")"
    local message; message="$(jf "$result" '.message')"
    if [[ "$(htb_read_flag_answer "$message")" == "si" ]]; then
        printf '%b\n' "  ${GRN}${BLD}$(t task_accepted)${R} — ${message}"
    else
        htb_fail "${message}"
    fi
    htb_enter
}

catalog_tracks() {
    htb_fetch v4_get "/tracks"
    htb_api_ok "$REPLY_BODY" || return
    local all; all="$(printf '%s' "$REPLY_BODY" | jq -c 'if type=="array" then . else (.data // []) end' 2>/dev/null)"
    local enrolled; enrolled="$(v4_get "/user/tracks" | jq -c 'if type=="array" then . else (.data // []) end' 2>/dev/null)"
    [[ -z "$enrolled" ]] && enrolled="[]"
    local difficulty="" at=0 rebuild=1
    local -a rows=() ids=() names=()
    local -a tdiff_opts=("$(t filter_any)|" "Very Easy|Very Easy" "Easy|Easy" "Medium|Medium" "Hard|Hard")
    while true; do
        if (( rebuild )); then
            rows=(); ids=(); names=()
            local id name diff likes creator staff enr
            while IFS=$'\t' read -r id name diff likes creator staff enr; do
                [[ -z "$id" ]] && continue
                local mark="  " mc="$DIM" note=""
                case "$enr" in
                    done) mark="✓✓"; mc="$GRN"; note=" · $(t tracks_enrolled_mark)" ;;
                    yes)  mark="✓ "; mc="$YLW"; note=" · $(t tracks_enrolled_mark)" ;;
                esac
                ids+=("$id"); names+=("$name")
                rows+=("$(printf '%b%s%b %b%-34s%b %b%-10s%b %b%s%b  %b%s %s · %s%s%b' \
                    "$mc" "$mark" "$R" "$WHT" "${name:0:34}" "$R" "$(htb_difficulty_color "$diff")" "${diff:0:10}" "$R" "$GRN" "$staff" "$R" "$DIM" "$likes" "$(t f_likes)" "$creator" "$note" "$R")")
            done < <(printf '%s' "$all" | jq -r --argjson en "$enrolled" --arg d "$difficulty" '
                ($en | map({key: (.id|tostring), value: (if (.complete // 0) > 0 then "done" else "yes" end)}) | from_entries) as $e
                | .[] | select($d == "" or ((.difficulty // "") | ascii_downcase) == ($d | ascii_downcase))
                | [(.id|tostring), .name, (.difficulty // "?"), ((.likes // 0)|tostring), (.creator.name // "-"), (if (.staff_pick // 0) == 1 then "STAFF PICK" else "" end), ($e[(.id|tostring)] // "")] | @tsv' 2>/dev/null)
            rebuild=0
        fi
        local title; title="$(t catalog_tracks)"; [[ -n "$difficulty" ]] && title+=" · ${difficulty}"
        if (( ${#ids[@]} == 0 )); then
            local -a empty=("  ${DIM}$(t no_results)${R}")
            htb_pick "$title" "$CYN" "$(htb_keys "d|$(t act_filter_challenge_difficulty)" "x|$(t act_filter_clear)")" empty 0
        else
            htb_pick "$title" "$CYN" "$(htb_keys "d|$(t act_filter_challenge_difficulty)" "x|$(t act_filter_clear)")" rows "$at"
        fi
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) (( ${#ids[@]} > 0 )) && track_details "${ids[$REPLY_SEL]}" "${names[$REPLY_SEL]}" ;;
            d|D) _pick_filter "$(t act_filter_challenge_difficulty)" "${tdiff_opts[@]}"
                 (( REPLY_FILTER_SET )) && { difficulty="$REPLY_FILTER"; at=0; rebuild=1; } ;;
            x|X) difficulty=""; at=0; rebuild=1 ;;
        esac
    done
}

track_details() {
    local id="$1" name="$2"
    htb_fetch v4_get "/tracks/${id}"
    local tr; tr="$(printf '%s' "$REPLY_BODY" | jq -c '.data // .' 2>/dev/null)"
    local diff; diff="$(jf "$tr" '.difficulty')"
    local -a lines=(
        ""
        "  ${BLD}${WHT}${name}${R}"
        ""
        "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t f_difficulty)" 18)" "$R" "$(htb_difficulty_color "$diff")" "$diff" "$R")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_likes)" 18)" "$R" "$(jf "$tr" '.likes')")"
        "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_enrolled)" 18)" "$R" "$([[ "$(jf "$tr" '.enrolled')" == "true" ]] && t yes || t no)")"
        ""
        "  ${CYN}── $(t f_content)${R}"
    )
    local row
    while IFS= read -r row; do lines+=("$row"); done < <(printf '%s' "$tr" | jq -r '(.items // [])[]? | "    · [\(.type)] \(.name) (\(.difficulty // "?"))"' 2>/dev/null)
    htb_show_lines "$name" "$CYN" lines
}

catalog_seasons() {
    htb_fetch v4_get "/season/list"
    local list="$REPLY_BODY"
    htb_api_ok "$list" || return
    local sid; sid="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .id' 2>/dev/null | head -1)"
    if [[ -z "$sid" ]]; then _season_overview "$list"; return; fi
    local name; name="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .name' 2>/dev/null | head -1)"
    local -a items=("${WHT}$(t season_overview)${R}" "${WHT}$(t season_leaderboard)${R}" "${WHT}$(t season_rewards)${R}")
    local at=0
    while true; do
        htb_pick "$(t catalog_seasons) · ${name}" "$PRP" "" items "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                case "$REPLY_SEL" in
                    0) _season_overview "$list" ;;
                    1) labs_season_leaderboard "$sid" "$name" ;;
                    2) labs_season_rewards "$sid" "$name" ;;
                esac ;;
        esac
    done
}

_season_overview() {
    local list="$1"
    local active_id; active_id="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .id' 2>/dev/null | head -1)"
    local -a lines=("")
    if [[ -z "$active_id" ]]; then
        lines+=("  ${YLW}$(t season_none)${R}")
    else
        local name; name="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .name' 2>/dev/null | head -1)"
        local state; state="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .state' 2>/dev/null | head -1)"
        local end; end="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .end_date // ""' 2>/dev/null | head -1)"
        local rank; rank="$(v4_get "/season/user/rank/${active_id}")"
        lines+=("  ${BLD}${WHT}${name}${R}  ${DIM}(${state})${R}")
        local left; left="$(labs_countdown "$end")" && lines+=("  ${DIM}$(t acad_target_time) ${left}${R}")
        lines+=("")
        local league; league="$(jf "$rank" '.data.league')"
        [[ "$league" == "-" ]] && league="→ $(jf "$rank" '.data.next_rank.title')"
        lines+=("$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t season_tier)" 18)" "$R" "$PRP" "$league" "$R")")
        if [[ "$(jf "$rank" '.data.rank')" != "-" ]]; then
            lines+=("$(printf '  %b%s%b%s/%s' "$GRY" "$(htb_pad "$(t season_rank)" 18)" "$R" "$(jf "$rank" '.data.rank')" "$(jf "$rank" '.data.total_ranks')")")
        fi
        lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t season_points)" 18)" "$R" "$(jf "$rank" '.data.total_season_points')")")
        lines+=("$(printf '  %b%s%b%s/%s' "$GRY" "$(htb_pad "$(t season_flags)" 18)" "$R" "$(jf "$rank" '.data.total_season_flags.obtained')" "$(jf "$rank" '.data.total_season_flags.total')")")
        lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t season_bloods)" 18)" "$R" "$(jf "$rank" '.data.total_season_bloods')")")
        local current; current="$(v4_get "/season/machine/active")"
        if [[ "$(jf "$current" '.data.name')" != "-" ]]; then
            lines+=("$(printf '  %b%s%b%b%s · user %s / root %s pts%b' "$GRY" "$(htb_pad "$(t season_next_drop)" 18)" "$R" "$GRN" "$(jf "$current" '.data.name')" "$(jf "$current" '.data.user_own_points')" "$(jf "$current" '.data.root_own_points')" "$R")")
        fi
        lines+=("" "  ${PRP}── $(t season_machines)${R}")
        local row i=0 mname mos mdiff mark released rtime unknown
        while IFS=$'\t' read -r mark mname mos mdiff released rtime unknown; do
            ((i++))
            [[ "$unknown" == "true" ]] && mname="?"
            local when=""
            if [[ "$released" != "true" ]]; then
                local left2; left2="$(labs_countdown "$rtime")" && when=" · $(t info_countdown) ${left2}" || when=" · ${rtime:0:10}"
            fi
            lines+=("    ${mark} $(t season_week) ${i}  ${mname} (${mos} · ${mdiff})${when}")
        done < <(v4_get "/season/machines" | jq -r '(if type=="array" then . else (.data // []) end)[]? | [(if .is_owned_root then "✓✓" elif .is_owned_user then "✓ " else "  " end), (.name // "-"), (.os // "-"), (.difficulty_text // "?"), ((.is_released // true)|tostring), ((.release_time // "")|tostring), ((.unknown // false)|tostring)] | @tsv' 2>/dev/null)
        lines+=("" "  ${DIM}$(t season_spawn_note)${R}")
    fi
    htb_show_lines "$(t catalog_seasons)" "$PRP" lines
}

_challenge_category_json() {
    local k out="{}"
    for k in "${!CHALLENGE_CATEGORY[@]}"; do
        out="$(jq -c -n --argjson o "$out" --arg k "$k" --arg v "${CHALLENGE_CATEGORY[$k]}" '$o + {($k): $v}')"
    done
    printf '%s' "$out"
}

_difficulty_color_json() {
    jq -c -n \
        --arg ve "$(htb_difficulty_color 'Very Easy')" \
        --arg e "$(htb_difficulty_color 'Easy')" \
        --arg m "$(htb_difficulty_color 'Medium')" \
        --arg h "$(htb_difficulty_color 'Hard')" \
        --arg i "$(htb_difficulty_color 'Insane')" \
        '{"Very Easy":$ve,"Easy":$e,"Medium":$m,"Hard":$h,"Insane":$i}'
}
