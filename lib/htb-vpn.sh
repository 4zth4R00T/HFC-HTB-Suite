#!/usr/bin/env bash

VPN_PRODUCTS=(labs starting_point sherlocks fortresses release_arena competitive)

menu_vpn() {
    local -a items=()
    local product
    for product in "${VPN_PRODUCTS[@]}"; do items+=("${WHT}${product}${R}"); done
    local at=0
    while true; do
        htb_pick "$(t vpn_title) · $(t vpn_product)" "$PRP" "" items "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) vpn_servers "${VPN_PRODUCTS[$REPLY_SEL]}" ;;
        esac
    done
}

vpn_servers() {
    local product="$1"
    htb_fetch v4_get "/connections/servers?product=${product}"
    local raw="$REPLY_BODY"
    htb_api_ok "$raw" || return
    if [[ "$(jf "$raw" '.status')" != "true" ]]; then
        clear; htb_title "$(t vpn_title) · ${product}" "$PRP"; echo
        htb_note "$(t vpn_unavailable)"
        htb_field "" "$(jf "$raw" '.message')"
        htb_report_plan "$(jf "$raw" '.message')"
        htb_enter; return
    fi

    local assigned; assigned="$(jf "$raw" '.data.assigned.id')"
    local assigned_name; assigned_name="$(jf "$raw" '.data.assigned.friendly_name')"
    local assigned_loc; assigned_loc="$(jf "$raw" '.data.assigned.location')"

    local -a ids=() rows=()
    local id name location clients full mark markcolor
    while IFS=$'\t' read -r id name location clients full; do
        [[ -z "$id" ]] && continue
        if [[ "$id" == "$assigned" ]]; then mark="▶"; markcolor="$GRN"; else mark=" "; markcolor="$DIM"; fi
        ids+=("$id")
        rows+=("$(printf '%b%s%b %b%-30s%b %b%-11s%b %b%s %s%b %b%s%b' \
            "$markcolor" "$mark" "$R" "$WHT" "${name:0:30}" "$R" \
            "$CYN" "${location:0:11}" "$R" \
            "$DIM" "$clients" "$(t vpn_clients)" "$R" \
            "$RED" "$([[ "$full" == "true" ]] && t vpn_full)" "$R")")
    done < <(printf '%s' "$raw" | jq -r '
        .data.options | to_entries[] | .value | to_entries[] | .value.servers | to_entries[] | .value
        | [(.id|tostring), .friendly_name, (.location // "?"), ((.current_clients // 0)|tostring), ((.full // false)|tostring)]
        | @tsv' 2>/dev/null | sort -t$'\t' -k3,3 -k2,2)

    if (( ${#ids[@]} == 0 )); then htb_fail "$(t no_results)"; htb_enter; return; fi

    local at=0
    while true; do
        local header="$(t vpn_title) · ${product} · $(t vpn_assigned): ${assigned_name} (${assigned}, ${assigned_loc})"
        htb_pick "$header" "$PRP" "$(htb_keys "↵|$(t act_download) UDP" "t|$(t act_download) TCP" "s|$(t act_switch)")" rows "$at"
        (( REPLY_SEL >= 0 )) && at=$REPLY_SEL
        local target="${ids[$REPLY_SEL]}"
        case "$REPLY_KEY" in
            quit) return ;;
            help) htb_help ;;
            enter) vpn_download "$product" "$target" 0 && { assigned="$target"; assigned_name="$(jf "$(v4_get "/connections/servers?product=${product}")" '.data.assigned.friendly_name')"; } ;;
            t|T)   vpn_download "$product" "$target" 1 && { assigned="$target"; assigned_name="$(jf "$(v4_get "/connections/servers?product=${product}")" '.data.assigned.friendly_name')"; } ;;
            s|S)
                if vpn_switch "$target"; then
                    assigned="$target"
                    assigned_name="$(jf "$(v4_get "/connections/servers?product=${product}")" '.data.assigned.friendly_name')"
                fi ;;
        esac
    done
}

vpn_switch() {
    local server="$1"
    echo
    printf '  %b\n' "${GRY}$(t vpn_switching)${R}"
    local result; result="$(v4_post "/connections/servers/switch/${server}")"
    if [[ "$(jf "$result" '.status')" == "true" ]]; then
        htb_ok "$(jf "$result" '.message') → $(jf "$result" '.data.friendly_name')"
        htb_enter
        return 0
    fi
    htb_fail "$(jf "$result" '.message')"
    htb_enter
    return 1
}

vpn_download() {
    local product="$1" server="$2" tcp="$3"
    local assigned; assigned="$(jf "$(v4_get "/connections/servers?product=${product}")" '.data.assigned.id')"
    if [[ "$server" != "$assigned" ]]; then
        echo
        htb_confirm "$(t confirm_switch)" || return 1
        local switch; switch="$(v4_post "/connections/servers/switch/${server}")"
        if [[ "$(jf "$switch" '.status')" != "true" ]]; then
            htb_fail "$(jf "$switch" '.message')"; htb_enter; return 1
        fi
    fi
    local proto="udp"; [[ "$tcp" == "1" ]] && proto="tcp"
    local out="${DL_DIR}/htb_${product}_${server}_${proto}.ovpn"
    local code
    code="$(_htb_curl_auth | curl -s --config - -L --max-time 60 -o "$out" -w '%{http_code}' "${BASE_V4}/access/ovpnfile/${server}/${tcp}")"
    chmod 600 "$out" 2>/dev/null
    local size; size="$(wc -c < "$out" 2>/dev/null || echo 0)"
    echo
    if [[ "$code" == "200" && "$size" -gt 500 ]]; then
        cp "$out" "${VPN_DIR}/current.ovpn"
        chmod 600 "${VPN_DIR}/current.ovpn" 2>/dev/null
        htb_ok "$(t vpn_saved)"
        htb_field "$(t saved_to)" "$out"
        htb_field "$(t vpn_copy)" "$VPN_DIR"
        htb_field "$(t vpn_connect)" "$out"
        htb_enter
        return 0
    fi
    rm -f "$out"
    htb_fail "$(t download_failed) (HTTP ${code})"
    htb_enter
    return 1
}
