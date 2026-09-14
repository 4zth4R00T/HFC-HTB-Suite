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

_load_challenge_categories() {
    (( ${#CHALLENGE_CATEGORY[@]} > 0 )) && return
    local id name
    while IFS=$'\t' read -r id name; do
        [[ -n "$id" ]] && CHALLENGE_CATEGORY["$id"]="$name"
    done < <(v4_get "/challenge/categories/list" | jq -r '.info[]? | [(.id|tostring), .name] | @tsv' 2>/dev/null)
}

_load_all_challenges() {
    local page=1 last=1 raw chunk
    local tmp; tmp="$(mktemp)"
    while true; do
        clear
        printf '\n  %b\n' "${GRY}$(t loading) ${page}/${last}${R}"
        raw="$(v4_get "/challenges?per_page=100&page=${page}")"
        chunk="$(printf '%s' "$raw" | jq -c '.data // []' 2>/dev/null)"
        [[ -z "$chunk" || "$chunk" == "[]" ]] && break
        printf '%s\n' "$chunk" >> "$tmp"
        last="$(printf '%s' "$raw" | jq -r '.meta.last_page // 1' 2>/dev/null)"
        [[ "$last" =~ ^[0-9]+$ ]] || last=1
        (( page >= last || page >= 40 )) && break
        ((page++))
    done
    REPLY_ALL="$(jq -c -s 'add // []' "$tmp" 2>/dev/null)"
    rm -f "$tmp"
    [[ -n "$REPLY_ALL" ]] || REPLY_ALL="[]"
}

catalog_challenges() {
    _load_all_challenges
    local all="$REPLY_ALL"
    if [[ -z "$all" || "$all" == "[]" ]]; then
        htb_fail "$(t no_results)"; htb_enter; return
    fi
    _load_challenge_categories

    local category="" difficulty="" unsolved=0 rebuild=1 at=0
    local -a cdiff_opts=("$(t filter_any)|" "Very Easy|Very Easy" "Easy|Easy" "Medium|Medium" "Hard|Hard" "Insane|Insane")
    local -a rows=() ids=()
    while true; do
        if (( rebuild )); then
            rows=(); ids=()
            local id row
            while IFS=$'\t' read -r id row; do
                [[ -z "$id" ]] && continue
                ids+=("$id")
                rows+=("$row")
            done < <(printf '%s' "$all" | jq -r \
                --arg category "$category" --arg difficulty "$difficulty" \
                --argjson unsolved "$unsolved" \
                --argjson cats "$(_challenge_category_json)" \
                --arg solved_label "$(t f_solves)" \
                --arg reset "$R" --arg white "$WHT" --arg purple "$PRP" \
                --arg dim "$DIM" --arg green "$GRN" \
                --argjson colors "$(_difficulty_color_json)" '
                def pad($n): (. + ("                                     "))[0:$n];
                .[] | . as $c
                | (($c.category_id // $c.challenge_category_id)|tostring) as $cat
                | (($c.difficulty // "?")) as $diff
                | ((($c.is_owned // $c.authUserSolve // $c.isCompleted) // false)) as $done
                | select($category == "" or $cat == $category)
                | select($difficulty == "" or $diff == $difficulty)
                | select($unsolved == 0 or ($done | not))
                | [ ($c.id|tostring),
                    (if $done then $green + "✓" else $dim + " " end) + $reset + " "
                    + $white + (($c.name // "?")[0:32] | pad(32)) + $reset + " "
                    + $purple + ((($cats[$cat]) // "?")[0:16] | pad(16)) + $reset + " "
                    + (($colors[$diff]) // "") + ($diff[0:11] | pad(11)) + $reset + " "
                    + $dim + $solved_label + " " + (($c.solves // 0)|tostring) + $reset
                  ] | @tsv' 2>/dev/null)
            rebuild=0
        fi
        if (( ${#ids[@]} == 0 )); then
            htb_fail "$(t no_results)"; htb_enter; return
        fi
        local shown="—"
        [[ -n "$category" ]] && shown="${CHALLENGE_CATEGORY[$category]}"
        local dshown; dshown="$(t filter_any)"
        [[ -n "$difficulty" ]] && dshown="$difficulty"
        local toggle; toggle="$(t no)"; (( unsolved )) && toggle="$(t yes)"
        local header="$(t catalog_challenges) · $(t act_category): ${shown} · $(t act_filter_challenge_difficulty): ${dshown} · $(t act_unsolved): ${toggle}"
        htb_pick "$header" "$CYN" "$(htb_keys "c|$(t act_category)" "d|$(t act_filter_challenge_difficulty)" "u|$(t act_unsolved)" "x|$(t act_filter_clear)")" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) challenge_details "${ids[$REPLY_SEL]}" ;;
            c|C) _pick_challenge_category; category="$REPLY_CATEGORY"; at=0; rebuild=1 ;;
            d|D) _pick_filter "$(t act_filter_challenge_difficulty)" "${cdiff_opts[@]}"
                 (( REPLY_FILTER_SET )) && { difficulty="$REPLY_FILTER"; at=0; rebuild=1; } ;;
            u|U) unsolved=$((1-unsolved)); at=0; rebuild=1 ;;
            x|X) category=""; difficulty=""; unsolved=0; at=0; rebuild=1 ;;
        esac
    done
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
        keys+=("f|$(t act_flag)" "←|$(t nav_back)")
        htb_keys "${keys[@]}"; echo
        htb_read_key
        case "$REPLY_KEYNAME" in
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

catalog_sherlocks() {
    local page=1 reload=1 last=1 at=0
    local -a rows=() ids=()
    while true; do
        if (( reload )); then
            htb_fetch v4_get "/sherlocks?page=${page}"
            local raw="$REPLY_BODY"
            htb_api_ok "$raw" || return
            last="$(jf "$raw" '.meta.last_page')"
            [[ "$last" =~ ^[0-9]+$ ]] || last=1
            rows=(); ids=()
            local id owned name diff cat solves dcolor mark markcolor
            while IFS=$'\t' read -r id owned name diff cat solves; do
                [[ -z "$id" ]] && continue
                dcolor="$(htb_difficulty_color "$diff")"
                if [[ "$owned" == "true" ]]; then mark="✓"; markcolor="$GRN"; else mark=" "; markcolor="$DIM"; fi
                ids+=("$id")
                rows+=("$(printf '%b%s%b %b%-32s%b %b%-18s%b %b%-11s%b %b%s%b' \
                    "$markcolor" "$mark" "$R" "$WHT" "${name:0:32}" "$R" \
                    "$PRP" "${cat:0:18}" "$R" "$dcolor" "${diff:0:11}" "$R" \
                    "$DIM" "$solves" "$R")")
            done < <(printf '%s' "$raw" | jq -r '(.data // .sherlocks // [])[] | [
                (.id|tostring), ((.is_owned // false)|tostring), .name,
                (.difficulty // "?"), (.category_name // "?"), ((.solves // 0)|tostring)
            ] | @tsv' 2>/dev/null)
            reload=0
        fi
        if (( ${#ids[@]} == 0 )); then htb_fail "$(t no_results)"; htb_enter; return; fi
        htb_pick "$(t catalog_sherlocks) · $(t act_page) ${page}/${last}" "$CYN" "$(htb_keys "n|$(t page_next)" "p|$(t page_previous)")" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) sherlock_details "${ids[$REPLY_SEL]}" ;;
            n|N) (( page < last )) && { ((page++)); reload=1; } ;;
            p|P) (( page > 1 )) && { ((page--)); reload=1; } ;;
        esac
    done
}

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
        keys+=("←|$(t nav_back)")
        htb_pick "$(t catalog_sherlocks) · ${name}" "$CYN" "$(htb_keys "${keys[@]}")" task_rows "$at" header
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
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
    local list; list="$(printf '%s' "$REPLY_BODY" | jq -c 'if type=="array" then . else (.data // []) end' 2>/dev/null)"
    local -a rows=() ids=() names=()
    local id name diff likes dcolor
    while IFS=$'\t' read -r id name diff likes; do
        [[ -z "$id" ]] && continue
        dcolor="$(htb_difficulty_color "$diff")"
        ids+=("$id"); names+=("$name")
        rows+=("$(printf '%b%-38s%b %b%-11s%b %b%s %s%b' \
            "$WHT" "${name:0:38}" "$R" "$dcolor" "${diff:0:11}" "$R" "$DIM" "$likes" "$(t f_likes)" "$R")")
    done < <(printf '%s' "$list" | jq -r '.[] | [(.id|tostring), .name, (.difficulty // "?"), ((.likes // 0)|tostring)] | @tsv' 2>/dev/null)
    if (( ${#ids[@]} == 0 )); then htb_fail "$(t no_results)"; htb_enter; return; fi
    local at=0
    while true; do
        htb_pick "$(t catalog_tracks)" "$CYN" "" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) track_details "${ids[$REPLY_SEL]}" "${names[$REPLY_SEL]}" ;;
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
    local active_id; active_id="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .id' 2>/dev/null | head -1)"
    local -a lines=("")
    if [[ -z "$active_id" ]]; then
        lines+=("  ${YLW}$(t season_none)${R}")
    else
        local name; name="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .name' 2>/dev/null | head -1)"
        local state; state="$(printf '%s' "$list" | jq -r '.data[]? | select(.active == true) | .state' 2>/dev/null | head -1)"
        local rank; rank="$(v4_get "/season/user/rank/${active_id}")"
        lines+=("  ${BLD}${WHT}${name}${R}  ${DIM}(${state})${R}" "")
        local league; league="$(jf "$rank" '.data.league')"
        [[ "$league" == "-" ]] && league="→ $(jf "$rank" '.data.next_rank.title')"
        lines+=("$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t season_tier)" 18)" "$R" "$PRP" "$league" "$R")")
        if [[ "$(jf "$rank" '.data.rank')" != "-" ]]; then
            lines+=("$(printf '  %b%s%b%s/%s' "$GRY" "$(htb_pad "$(t season_rank)" 18)" "$R" "$(jf "$rank" '.data.rank')" "$(jf "$rank" '.data.total_ranks')")")
        fi
        lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t season_points)" 18)" "$R" "$(jf "$rank" '.data.total_season_points')")")
        lines+=("$(printf '  %b%s%b%s/%s' "$GRY" "$(htb_pad "$(t season_flags)" 18)" "$R" "$(jf "$rank" '.data.total_season_flags.obtained')" "$(jf "$rank" '.data.total_season_flags.total')")")
        lines+=("$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t season_bloods)" 18)" "$R" "$(jf "$rank" '.data.total_season_bloods')")")
        lines+=("" "  ${PRP}── $(t season_machines)${R}")
        local row
        while IFS= read -r row; do lines+=("$row"); done < <(v4_get "/season/machines" | jq -r '(if type=="array" then . else (.data // []) end)[]? | "    \(if .is_owned_root then "✓✓" elif .is_owned_user then "✓ " else "  " end) \(.name) (\(.os) · \(.difficulty_text // "?"))"' 2>/dev/null)
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
