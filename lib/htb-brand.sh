#!/usr/bin/env bash

HTB_AUTHOR="Aztharot"
HTB_AUTHOR_INSTAGRAM="instagram.com/its_aztharot"
HTB_AUTHOR_GITHUB="github.com/Aztharot-RO0t3d"
HTB_CREW="HFC — Hackers Fight Club"
HTB_CREW_INSTAGRAM="instagram.com/hackersfightclub"
HTB_CREW_BLOG="blog.hackersfight.club"
HTB_THANKS="Lorne · Ookami · Pablocro"

HFC_MARK=(
'██╗  ██╗███████╗ ██████╗'
'██║  ██║██╔════╝██╔════╝'
'███████║█████╗  ██║     '
'██╔══██║██╔══╝  ██║     '
'██║  ██║██║     ╚██████╗'
'╚═╝  ╚═╝╚═╝      ╚═════╝'
)

WORDMARK=(
'██╗  ██╗████████╗██████╗   ███████╗██╗   ██╗██╗████████╗███████╗'
'██║  ██║╚══██╔══╝██╔══██╗  ██╔════╝██║   ██║██║╚══██╔══╝██╔════╝'
'███████║   ██║   ██████╔╝  ███████╗██║   ██║██║   ██║   █████╗  '
'██╔══██║   ██║   ██╔══██╗  ╚════██║██║   ██║██║   ██║   ██╔══╝  '
'██║  ██║   ██║   ██████╔╝  ███████║╚██████╔╝██║   ██║   ███████╗'
'╚═╝  ╚═╝   ╚═╝   ╚═════╝   ╚══════╝ ╚═════╝ ╚═╝   ╚═╝   ╚══════╝'
)

htb_center_indent() {
    local width="$1" cols pad
    cols=$(htb_cols)
    pad=$(( (cols - width) / 2 ))
    (( pad < 0 )) && pad=0
    printf '%s' "$pad"
}

htb_draw_block() {
    local -n __block="$1"
    local pad; printf -v pad '%*s' "$(htb_center_indent "${#__block[0]}")" ''
    local line
    for line in "${__block[@]}"; do
        printf '%b\n' "${pad}${GRN}${line}${R}"
    done
}

htb_show_mark() { htb_draw_block HFC_MARK; }

htb_follow_lines() {
    local pad; printf -v pad '%*s' "$(htb_center_indent 58)" ''
    printf '%b\n' "${pad}${WHT}${HTB_AUTHOR}${R} ${GRY}— $(t follow_author)${R} ${CYN}${HTB_AUTHOR_INSTAGRAM}${R}"
    printf '%b\n' "${pad}${GRN}${HTB_CREW}${R} ${GRY}— $(t follow_crew)${R} ${CYN}${HTB_CREW_INSTAGRAM}${R}"
}

htb_boot() {
    [[ "${HTB_NO_ANIM:-0}" == "1" ]] && return
    clear
    printf '%s' "$HTB_HIDE"
    echo
    htb_draw_block HFC_MARK
    echo
    htb_draw_block WORDMARK
    echo
    htb_follow_lines
    echo
    local pad; printf -v pad '%*s' "$(htb_center_indent 50)" ''
    printf '%b\n' "${pad}${DIM}$(htb_random_welcome)${R}"
    sleep 0.9
    printf '%s' "$HTB_SHOW"
}
