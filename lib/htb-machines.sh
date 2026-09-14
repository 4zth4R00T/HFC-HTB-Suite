#!/usr/bin/env bash

menu_machines() {
    local -a items=(
        "$(htb_row "$(t machines_active)" "$(t machines_active_desc)")"
        "$(htb_row "$(t machines_retired)" "$(t machines_retired_desc)")"
        "$(htb_row "$(t machines_search)" "$(t machines_search_desc)")"
        "$(htb_row "$(t machines_target)" "$(t machines_target_desc)")"
        "$(htb_row "$(t machines_sp)" "$(t machines_sp_desc)")"
    )
    local at=0
    while true; do
        htb_pick "$(t machines_title)" "$GRN" "" items "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                case "$REPLY_SEL" in
                    0) machines_browse "$(t machines_active)" "active" "" ;;
                    1) machines_browse "$(t machines_retired)" "retired" "" ;;
                    2)
                        clear; htb_title "$(t machines_search)" "$WHT"; echo
                        htb_prompt "$(t ask_search)"
                        [[ -n "$REPLY_TEXT" ]] && machines_browse "$(t machines_search): $REPLY_TEXT" "" "$REPLY_TEXT"
                        ;;
                    3) machines_target ;;
                    4) machines_starting_point ;;
                esac ;;
        esac
    done
}

_machine_rows() {
    local json="$1"
    MACHINE_IDS=()
    MACHINE_ROWS=()
    local id name os diff pts owns uo ro free date mark markcolor dcolor
    while IFS=$'\t' read -r id name os diff pts owns uo ro free date; do
        [[ -z "$id" ]] && continue
        if [[ "$uo" == "true" && "$ro" == "true" ]]; then mark="✓✓"; markcolor="$GRN"
        elif [[ "$uo" == "true" || "$ro" == "true" ]]; then mark="✓ "; markcolor="$YLW"
        else mark="  "; markcolor="$DIM"; fi
        dcolor="$(htb_difficulty_color "$diff")"
        [[ "$free" == "true" ]] && free="Free" || free="VIP"
        MACHINE_IDS+=("$id")
        MACHINE_ROWS+=("$(printf '%b%s%b %b%-26s%b %b%-8s%b %b%-9s%b %b%4s pts%b  %b%6s owns · %-4s · %s%b' \
            "$markcolor" "$mark" "$R" \
            "$WHT" "${name:0:26}" "$R" \
            "$CYN" "${os:0:8}" "$R" \
            "$dcolor" "${diff:0:9}" "$R" \
            "$YLW" "$pts" "$R" \
            "$DIM" "$owns" "$free" "${date:0:10}" "$R")")
    done < <(printf '%s' "$json" | jq -r '.[] | [
        (.id|tostring), .name, (.os // "?"),
        (.difficultyText // .difficulty // "?"),
        ((.points // .static_points // 0)|tostring),
        ((.userOwnsCount // 0)|tostring),
        ((.authUserInUserOwns // false)|tostring),
        ((.authUserInRootOwns // false)|tostring),
        ((.free // false)|tostring),
        ((.release // .releaseDate // .retiredDate // "")|tostring)
    ] | @tsv' 2>/dev/null)
}

machines_browse() {
    local title="$1" state="$2" keyword="$3"
    local page=1 difficulty="" os="" solved="" reload=1 total="?" last=1 at=0
    local -a diff_opts=("$(t filter_any)|" "Very Easy|very-easy" "Easy|easy" "Medium|medium" "Hard|hard" "Insane|insane")
    local -a os_opts=("$(t filter_any)|" "Linux|linux" "Windows|windows" "FreeBSD|freebsd" "OpenBSD|openbsd" "Android|android" "Other|other")
    local -a solved_opts=("$(t filter_any)|" "$(t filter_solved)|solved" "$(t filter_unsolved)|unsolved")
    while true; do
        if (( reload )); then
            local qs="per_page=25&page=${page}"
            [[ -n "$state" ]] && qs+="&state=${state}"
            [[ -n "$keyword" ]] && qs+="&keyword=$(urlencode "$keyword")"
            [[ -n "$difficulty" ]] && qs+="&difficulty[]=$(urlencode "$difficulty")"
            [[ -n "$os" ]] && qs+="&os[]=$(urlencode "$os")"
            htb_fetch v5_get "/machines?${qs}"
            local raw="$REPLY_BODY"
            htb_api_ok "$raw" || return
            total="$(jf "$raw" '.meta.total')"
            last="$(jf "$raw" '.meta.last_page')"
            [[ "$last" =~ ^[0-9]+$ ]] || last=1
            local filtered; filtered="$(printf '%s' "$raw" | jq -c '.data // []')"
            case "$solved" in
                solved)   filtered="$(printf '%s' "$filtered" | jq -c '[.[] | select((.authUserInUserOwns // false) or (.authUserInRootOwns // false))]')" ;;
                unsolved) filtered="$(printf '%s' "$filtered" | jq -c '[.[] | select(((.authUserInUserOwns // false) or (.authUserInRootOwns // false)) | not)]')" ;;
            esac
            _machine_rows "$filtered"
            reload=0
        fi
        if (( ${#MACHINE_IDS[@]} == 0 )); then
            htb_fail "$(t no_results)"; htb_enter; return
        fi
        local shown="$total"
        [[ -n "$solved" ]] && shown="${#MACHINE_IDS[@]}"
        local header="${title}"
        [[ -n "$difficulty" ]] && header+=" · $(_filter_label "$difficulty" "${diff_opts[@]}")"
        [[ -n "$os" ]] && header+=" · $(_filter_label "$os" "${os_opts[@]}")"
        [[ -n "$solved" ]] && header+=" · $(_filter_label "$solved" "${solved_opts[@]}")"
        header+=" · ${shown} · $(t act_page) ${page}/${last}"
        local hint
        hint="$(htb_keys "s|$(t act_spawn)" "f|$(t act_flag)" "t|$(t act_stop)" "r|$(t act_reset)" "e|$(t act_extend)" "n|$(t page_next)" "p|$(t page_previous)" "d|$(t act_filter_difficulty)" "o|$(t act_filter_os)" "v|$(t act_filter_solved)" "x|$(t act_filter_clear)" "c|$(t act_copy)")"
        htb_pick "$header" "$GRN" "$hint" MACHINE_ROWS "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        local target="${MACHINE_IDS[$REPLY_SEL]}"
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) machine_details "$target" ;;
            s|S) machine_spawn "$target"; reload=1 ;;
            f|F) machine_flag "$target"; reload=1 ;;
            t|T) machine_stop; reload=1 ;;
            r|R) machine_reset ;;
            e|E) machine_extend ;;
            n|N) (( page < last )) && { ((page++)); reload=1; } ;;
            p|P) (( page > 1 )) && { ((page--)); reload=1; } ;;
            d|D) _pick_filter "$(t act_filter_difficulty)" "${diff_opts[@]}"
                 (( REPLY_FILTER_SET )) && { difficulty="$REPLY_FILTER"; page=1; reload=1; } ;;
            o|O) _pick_filter "$(t act_filter_os)" "${os_opts[@]}"
                 (( REPLY_FILTER_SET )) && { os="$REPLY_FILTER"; page=1; reload=1; } ;;
            v|V) _pick_filter "$(t act_filter_solved)" "${solved_opts[@]}"
                 (( REPLY_FILTER_SET )) && { solved="$REPLY_FILTER"; reload=1; } ;;
            x|X) difficulty=""; os=""; solved=""; page=1; reload=1 ;;
            c|C) htb_show_copied "$(jf "$(v4_get '/machine/active')" '.info.ip')" ;;
        esac
    done
}

machine_details() {
    local id="$1" at=0 reload=1
    local -a header=()
    while true; do
        if (( reload )); then
            htb_fetch v4_get "/machine/profile/${id}"
            local m; m="$(printf '%s' "$REPLY_BODY" | jq -c '.info // .' 2>/dev/null)"
            local diff; diff="$(jf "$m" '.difficultyText')"
            local name; name="$(jf "$m" '.name')"
            local single; single="$(jf "$m" '.isSingleFlag')"
            local owns
            if [[ "$single" == "true" ]]; then
                owns="$(jf "$m" '.root_owns_count')"
            else
                owns="user $(jf "$m" '.user_owns_count') · root $(jf "$m" '.root_owns_count')"
            fi
            _load_task_rows "$id"
            header=(
                "  ${BLD}${WHT}${name}${R}"
                ""
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "ID" 18)" "$R" "$id")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_os)" 18)" "$R" "$(jf "$m" '.os')")"
                "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t f_difficulty)" 18)" "$R" "$(htb_difficulty_color "$diff")" "$diff" "$R")"
                "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t f_points)" 18)" "$R" "$YLW" "$(jf "$m" '.points')" "$R")"
                "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t f_ip)" 18)" "$R" "$CYN" "$(jf "$m" '.playInfo.ip')" "$R")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_owns)" 18)" "$R" "$owns")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_rating)" 18)" "$R" "$(jf "$m" '.stars')")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_user_rating)" 18)" "$R" "$(_rating_label "$m")")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_released)" 18)" "$R" "$(jf "$m" '.release' | cut -c1-10)")"
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "$(t f_author)" 18)" "$R" "$(jf "$m" '.maker.name')")"
            )
            (( ${#TASK_IDS[@]} > 0 )) && header+=("$(_progress_line)")
            header+=("")
            reload=0
        fi
        local -a keys=("s|$(t act_spawn)")
        (( ${#TASK_IDS[@]} > 0 )) && keys+=("↵|$(t task_answer)")
        keys+=("f|$(t act_flag)" "t|$(t act_stop)" "r|$(t act_reset)" "e|$(t act_extend)" "v|$(t votes_title)" "c|$(t act_copy)")
        if (( ${#TASK_IDS[@]} == 0 )); then
            local -a empty_rows=("  ${DIM}$(t no_tasks)${R}")
            htb_pick "$name" "$GRN" "$(htb_keys "${keys[@]}")" empty_rows 0 header
        else
            htb_pick "$name" "$GRN" "$(htb_keys "${keys[@]}")" TASK_ROWS "$at" header
        fi
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                (( ${#TASK_IDS[@]} == 0 )) && continue
                _answer_task "$id" "${TASK_IDS[$REPLY_SEL]}"; reload=1 ;;
            s|S) machine_spawn "$id"; reload=1 ;;
            f|F) machine_flag "$id"; reload=1 ;;
            t|T) machine_stop; reload=1 ;;
            r|R) machine_reset ;;
            e|E) machine_extend ;;
            v|V) _show_votes "$m" ;;
            c|C) htb_show_copied "$(jf "$m" '.playInfo.ip')" ;;
        esac
    done
}

_rating_label() {
    local m="$1"
    printf '%s' "$m" | jq -r --arg none "-" '
        (.feedbackForChart // {}) as $c
        | [$c.counterCake, $c.counterVeryEasy, $c.counterEasy, $c.counterTooEasy,
           $c.counterMedium, $c.counterBitHard, $c.counterHard, $c.counterTooHard,
           $c.counterExHard, $c.counterBrainFuck] | map(. // 0) as $v
        | ($v | add) as $total
        | if $total == 0 then $none
          else
            ([range(0;10) | ($v[.] * (.+1))] | add) / $total
            | (. + 0.5 | floor) as $idx
            | ($idx | if . < 1 then 1 elif . > 10 then 10 else . end | tostring) + "\t" + ($total|tostring)
          end' 2>/dev/null | {
        IFS=$'\t' read -r idx total
        if [[ -z "$idx" || "$idx" == "-" ]]; then printf '%s' "-"
        else printf '%s (%s)' "$(t "rate_${idx}")" "$total"; fi
    }
}

_show_votes() {
    local m="$1"
    local -a lines=("")
    local i count top=0
    local -a counts=()
    local -a keys=(counterCake counterVeryEasy counterEasy counterTooEasy counterMedium
                   counterBitHard counterHard counterTooHard counterExHard counterBrainFuck)
    for i in "${!keys[@]}"; do
        count="$(printf '%s' "$m" | jq -r ".feedbackForChart.${keys[$i]} // 0" 2>/dev/null)"
        [[ "$count" =~ ^[0-9]+$ ]] || count=0
        counts+=("$count")
        (( count > top )) && top=$count
    done
    if (( top == 0 )); then
        echo; htb_note "$(t no_results)"; htb_enter; return
    fi
    for i in "${!counts[@]}"; do
        local width=$(( counts[i] * 24 / top )) bar="" j
        for (( j = 0; j < width; j++ )); do bar+="█"; done
        lines+=("$(printf '  %b%s%b%b%s%b %b%s%b' "$GRY" "$(htb_pad "$(t "rate_$((i+1))")" 18)" "$R" \
            "$GRN" "$bar" "$R" "$DIM" "${counts[$i]}" "$R")")
    done
    htb_show_lines "$(t votes_title)" "$CYN" lines
}

machine_spawn() {
    local id="$1"
    local active; active="$(v4_get '/machine/active')"
    local current; current="$(jf "$active" '.info.name')"
    if [[ "$current" != "-" && -n "$current" ]]; then
        echo
        htb_confirm "${current} — $(t confirm_replace)" || return
        v4_post "/vm/terminate" "{\"machine_id\":$(jf "$active" '.info.id')}" >/dev/null 2>&1
        sleep 2
    fi
    echo
    local result; result="$(v4_post '/vm/spawn' "{\"machine_id\":${id}}")"
    local message; message="$(jf "$result" '.message')"
    if [[ "$(htb_read_action_answer "$message")" != "si" ]]; then
        htb_fail "$message"
        htb_report_plan "$message"
        htb_enter; return
    fi
    htb_ok "$message"
    printf '  %b' "${GRY}$(t spawn_waiting)${R}"
    local ip="-" i
    for (( i=0; i<10; i++ )); do
        sleep 3
        ip="$(jf "$(v4_get '/machine/active')" '.info.ip')"
        [[ "$ip" != "-" && -n "$ip" ]] && break
        printf '.'
    done
    echo
    if [[ "$ip" == "-" || -z "$ip" ]]; then
        htb_note "$(t spawn_no_ip)"
    else
        htb_field "$(t f_ip)" "${CYN}${ip}${R}"
    fi
    htb_enter
}

_machine_action() {
    local endpoint="$1" label="$2" ask="$3"
    local active; active="$(v4_get '/machine/active')"
    local name; name="$(jf "$active" '.info.name')"
    if [[ "$name" == "-" || -z "$name" ]]; then
        echo; htb_note "$(t no_active_machine)"; htb_enter; return
    fi
    echo
    if [[ -n "$ask" ]]; then
        htb_confirm "$ask" || return
    fi
    local result; result="$(v4_post "$endpoint" "{\"machine_id\":$(jf "$active" '.info.id')}")"
    local message; message="$(jf "$result" '.message')"
    if [[ "$(htb_read_action_answer "$message")" == "si" ]]; then
        htb_ok "${label}: ${message}"
    else
        htb_fail "$message"
        htb_report_plan "$message"
    fi
    htb_enter
}

machine_stop()   { _machine_action "/vm/terminate" "$(t act_stop)"   "$(t confirm_stop)"; }
machine_reset()  { _machine_action "/vm/reset"     "$(t act_reset)"  "$(t confirm_reset)"; }
machine_extend() { _machine_action "/vm/extend"    "$(t act_extend)" ""; }

_pick_rating() {
    local -a names=()
    local i
    for i in $(seq 1 10); do names+=("$(printf '%2d  %s' "$i" "$(t "rate_$i")")"); done
    REPLY_RATING=0
    htb_pick "$(t rate_title)" "$CYN" "" names
    [[ "$REPLY_KEY" == "enter" ]] && REPLY_RATING=$(( REPLY_SEL + 1 ))
}

machines_starting_point() {
    htb_fetch v4_get "/sp/tiers/progress"
    local raw="$REPLY_BODY"
    htb_api_ok "$raw" || return
    local -a tier_ids=() tier_names=() rows=()
    local id name pct desc bar filled
    while IFS=$'\t' read -r id name pct desc; do
        [[ -z "$id" ]] && continue
        filled=$(( pct * 20 / 100 ))
        bar=""
        local __i
        for (( __i = 0; __i < 20; __i++ )); do
            if (( __i < filled )); then bar+="█"; else bar+="░"; fi
        done
        tier_ids+=("$id"); tier_names+=("$name")
        rows+=("$(printf '%b%-10s%b %b%s%b %b%3s%%%b  %b%s%b' \
            "$WHT" "$name" "$R" "$GRN" "$bar" "$R" "$CYN" "$pct" "$R" "$DIM" "${desc:0:60}" "$R")")
    done < <(printf '%s' "$raw" | jq -r '.data[] | [(.id|tostring), .name, ((.completion_percentage // 0)|tostring), (.description // "")] | @tsv' 2>/dev/null)
    (( ${#tier_ids[@]} == 0 )) && { htb_fail "$(t no_results)"; htb_enter; return; }

    local at=0
    while true; do
        htb_pick "$(t machines_sp)" "$GRN" "" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) _sp_tier_machines "${tier_ids[$REPLY_SEL]}" "${tier_names[$REPLY_SEL]}" ;;
        esac
    done
}

_sp_tier_machines() {
    local tier="$1" tier_name="$2" at=0 reload=1
    while true; do
        if (( reload )); then
            htb_fetch v5_get "/machines?spTier=${tier}"
            local raw="$REPLY_BODY"
            htb_api_ok "$raw" || return
            _machine_rows "$(printf '%s' "$raw" | jq -c '.data // []')"
            reload=0
        fi
        (( ${#MACHINE_IDS[@]} == 0 )) && { htb_fail "$(t no_results)"; htb_enter; return; }
        local hint
        hint="$(htb_keys "s|$(t act_spawn)" "f|$(t act_flag)" "t|$(t act_stop)" "c|$(t act_copy)")"
        htb_pick "$(t machines_sp) · ${tier_name} · ${#MACHINE_IDS[@]}" "$GRN" "$hint" MACHINE_ROWS "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        local target="${MACHINE_IDS[$REPLY_SEL]}"
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) machine_details "$target" ;;
            s|S) machine_spawn "$target"; reload=1 ;;
            f|F) machine_flag "$target"; reload=1 ;;
            t|T) machine_stop; reload=1 ;;
            c|C) htb_show_copied "$(jf "$(v4_get '/machine/active')" '.info.ip')" ;;
        esac
    done
}

_load_task_rows() {
    local id="$1"
    local raw; raw="$(v4_get "/machines/${id}/tasks")"
    local n; n="$(printf '%s' "$raw" | jq -r '(.data // []) | length' 2>/dev/null)"
    [[ "$n" =~ ^[0-9]+$ ]] || n=0
    (( n == 0 )) && raw="$(v4_get "/machines/${id}/adventure")"
    TASK_IDS=(); TASK_ROWS=(); TASK_DONE=0
    local tid title done_ desc expected answer mark markcolor extra
    while IFS=$'\x1f' read -r tid title done_ desc expected answer; do
        [[ -z "$title" ]] && continue
        if [[ "$done_" == "true" ]]; then
            mark="✓"; markcolor="$GRN"; (( TASK_DONE++ ))
        else
            mark=" "; markcolor="$DIM"
        fi
        TASK_IDS+=("$tid")
        extra="$(t task_expected): ${expected}"
        [[ "$done_" == "true" && -n "$answer" && "$answer" != "null" ]] && extra="$(t task_answer): ${answer}"
        TASK_ROWS+=("$(printf '%b%s%b %b%s%b %b%s%b  %b%s%b' \
            "$markcolor" "$mark" "$R" "$GRY" "$(htb_pad "${title:0:20}" 20)" "$R" \
            "$WHT" "$(htb_pad "${desc:0:78}" 78)" "$R" "$DIM" "${extra:0:44}" "$R")")
    done < <(printf '%s' "$raw" | jq -r '(.data // [])[] | [
        ((.id // "")|tostring), (.title // "-"),
        ((.completed // false)|tostring),
        ((.description // "") | gsub("[\n\r\t]"; " ")),
        (.masked_flag // "-"), (.flag // "")
    ] | join("\u001f")' 2>/dev/null)
}

_progress_line() {
    local color="$YLW"
    (( TASK_DONE == ${#TASK_IDS[@]} )) && color="$GRN"
    printf '  %b%s%b%b%s/%s %s%b' "$GRY" "$(htb_pad "$(t f_progress)" 18)" "$R" \
        "$color" "$TASK_DONE" "${#TASK_IDS[@]}" "$(t tasks_answered)" "$R"
}

_answer_task() {
    local id="$1" task_id="$2"
    echo
    htb_prompt_flag "$(t task_answer)"
    local answer="$REPLY_TEXT"
    [[ -z "$answer" ]] && return
    local result
    if [[ -z "$task_id" ]]; then
        result="$(v5_post '/machine/own' "{\"flag\":\"${answer}\",\"id\":${id},\"difficulty\":5}")"
    else
        result="$(v4_post "/machines/${id}/tasks/${task_id}/flag" "{\"flag\":\"${answer}\"}")"
    fi
    local message; message="$(jf "$result" '.message')"
    case "$(htb_read_flag_answer "$message")" in
        si) printf '%b\n' "  ${GRN}${BLD}$(t task_accepted)${R} — ${message}" ;;
        no) htb_fail "$message" ;;
        *)  htb_note "$message" ;;
    esac
    htb_enter
}

machine_flag() {
    local id="$1"
    echo
    htb_prompt_flag "$(t ask_flag)"
    local flag="$REPLY_TEXT"
    [[ -z "$flag" ]] && return
    _pick_rating
    local rating="$REPLY_RATING"
    (( rating >= 1 && rating <= 10 )) || return
    local result; result="$(v5_post '/machine/own' "{\"flag\":\"${flag}\",\"id\":${id},\"difficulty\":${rating}}")"
    local message; message="$(jf "$result" '.message')"
    case "$(htb_read_flag_answer "$message")" in
        si) printf '%b\n' "  ${GRN}${BLD}$(t flag_accepted)${R} — ${message}" ;;
        no) htb_fail "$message" ;;
        *)  htb_note "$message" ;;
    esac
    htb_enter
}

machines_target() {
    local at=0 reload=1
    local -a header=()
    local id ip name
    while true; do
        if (( reload )); then
            htb_fetch v4_get "/machine/active"
            local raw="$REPLY_BODY"
            name="$(jf "$raw" '.info.name')"
            if [[ "$name" == "-" || -z "$name" ]]; then
                clear; htb_title "$(t machines_target)" "$GRN"; echo
                htb_note "$(t no_active_machine)"
                htb_enter; return
            fi
            id="$(jf "$raw" '.info.id')"
            ip="$(jf "$raw" '.info.ip')"
            _load_task_rows "$id"
            header=(
                "  ${BLD}${WHT}${name}${R}"
                ""
                "$(printf '  %b%s%b%s' "$GRY" "$(htb_pad "ID" 18)" "$R" "$id")"
                "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t f_ip)" 18)" "$R" "$CYN" "$ip" "$R")"
                "$(printf '  %b%s%b%b%s%b' "$GRY" "$(htb_pad "$(t f_expires)" 18)" "$R" "$YLW" "$(jf "$raw" '.info.expires_at')" "$R")"
            )
            (( ${#TASK_IDS[@]} > 0 )) && header+=("$(_progress_line)")
            header+=("")
            reload=0
        fi
        local -a keys=()
        (( ${#TASK_IDS[@]} > 0 )) && keys+=("↵|$(t task_answer)")
        keys+=("f|$(t act_flag)" "t|$(t act_stop)" "r|$(t act_reset)" "e|$(t act_extend)" "c|$(t act_copy)")
        if (( ${#TASK_IDS[@]} == 0 )); then
            local -a empty_rows=("  ${DIM}$(t no_tasks)${R}")
            htb_pick "$(t machines_target)" "$GRN" "$(htb_keys "${keys[@]}")" empty_rows 0 header
        else
            htb_pick "$(t machines_target)" "$GRN" "$(htb_keys "${keys[@]}")" TASK_ROWS "$at" header
        fi
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter)
                (( ${#TASK_IDS[@]} == 0 )) && continue
                _answer_task "$id" "${TASK_IDS[$REPLY_SEL]}"; reload=1 ;;
            f|F) machine_flag "$id"; reload=1 ;;
            t|T) machine_stop; reload=1 ;;
            r|R) machine_reset ;;
            e|E) machine_extend ;;
            c|C) htb_show_copied "$ip" ;;
        esac
    done
}

machine_flag() {
    local id="$1"
    echo
    htb_prompt_flag "$(t ask_flag)"
    local flag="$REPLY_TEXT"
    [[ -z "$flag" ]] && return
    _pick_rating
    local rating="$REPLY_RATING"
    (( rating >= 1 && rating <= 10 )) || return
    local result; result="$(v5_post '/machine/own' "{\"flag\":\"${flag}\",\"id\":${id},\"difficulty\":${rating}}")"
    local message; message="$(jf "$result" '.message')"
    case "$(htb_read_flag_answer "$message")" in
        si) printf '%b\n' "  ${GRN}${BLD}$(t flag_accepted)${R} — ${message}" ;;
        no) htb_fail "$message" ;;
        *)  htb_note "$message" ;;
    esac
    htb_enter
}
