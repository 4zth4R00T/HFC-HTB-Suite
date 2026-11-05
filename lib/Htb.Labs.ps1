$Script:HtbWeb = "https://app.hackthebox.com"

function Get-HtbCountdown($Value) {
    if ($null -eq $Value -or "$Value" -eq "" -or "$Value" -eq "-") { return "" }
    try {
        $target = if ($Value -is [datetime]) { $Value.ToUniversalTime() } else { [datetime]::Parse("$Value", [cultureinfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::AdjustToUniversal) }
    } catch { return "" }
    $left = $target - (Get-Date).ToUniversalTime()
    if ($left.TotalSeconds -le 0) { return "" }
    if ($left.Days -gt 0) { return "$($left.Days)d $($left.Hours)h" }
    return "$($left.Hours)h $($left.Minutes)m"
}

function Get-HtbRatingText($Item) {
    $count = 0
    if ($null -ne $Item.ratingCount) { $count = [int]$Item.ratingCount } elseif ($null -ne $Item.rating_count) { $count = [int]$Item.rating_count }
    if ($count -le 0) { return "-" }
    $rating = [double]$Item.rating
    return ("★{0:N1} ({1})" -f $rating, $count)
}

function Get-HtbLabelsText($Item) {
    $out = @()
    if ($Item.labels) {
        foreach ($l in @($Item.labels)) { if ($l.name) { $out += "$($l.name)".ToUpper() } }
    }
    if ($Item.staff_pick -eq 1 -or $Item.staff_pick -eq $true) { $out += "STAFF PICK" }
    return ($out -join " · ")
}

function Get-HtbSortFields([string]$Kind) {
    if ($Kind -eq "machines") {
        return @(
            [pscustomobject]@{ Api = ""; Label = "" }
            [pscustomobject]@{ Api = "rating"; Label = (T 'sort_rating') }
            [pscustomobject]@{ Api = "release_date"; Label = (T 'sort_release') }
            [pscustomobject]@{ Api = "user_difficulty"; Label = (T 'sort_user_difficulty') }
            [pscustomobject]@{ Api = "user_owns"; Label = (T 'sort_user_owns') }
            [pscustomobject]@{ Api = "system_owns"; Label = (T 'sort_system_owns') }
            [pscustomobject]@{ Api = "name"; Label = (T 'sort_name') }
        )
    }
    return @(
        [pscustomobject]@{ Api = ""; Label = "" }
        [pscustomobject]@{ Api = "rating"; Label = (T 'sort_rating') }
        [pscustomobject]@{ Api = "release_date"; Label = (T 'sort_release') }
        [pscustomobject]@{ Api = "solves"; Label = (T 'sort_solves') }
        [pscustomobject]@{ Api = "name"; Label = (T 'sort_name') }
    )
}

function Get-HtbSortQuery([string]$Field, [bool]$Desc) {
    if (-not $Field) { return "" }
    return "&sort_by=$Field&sort_type=$(if ($Desc) { 'desc' } else { 'asc' })"
}

function Get-HtbSortLabel([string]$Kind, [string]$Field, [bool]$Desc) {
    if (-not $Field) { return "" }
    $f = Get-HtbSortFields $Kind | Where-Object { $_.Api -eq $Field } | Select-Object -First 1
    if (-not $f) { return "" }
    return "$(T 'act_sort') $($f.Label) $(if ($Desc) { '▼' } else { '▲' })"
}

function Get-HtbSortNext([string]$Kind, [string]$Field) {
    $fields = @(Get-HtbSortFields $Kind)
    for ($i = 0; $i -lt $fields.Count; $i++) {
        if ($fields[$i].Api -eq $Field) { return $fields[($i + 1) % $fields.Count].Api }
    }
    return $fields[1].Api
}

function Get-HtbStateOptions {
    @(
        [pscustomobject]@{ Label = (T 'state_all'); Value = "" }
        [pscustomobject]@{ Label = (T 'state_active'); Value = "active" }
        [pscustomobject]@{ Label = (T 'state_retired'); Value = "retired" }
        [pscustomobject]@{ Label = (T 'state_unreleased'); Value = "unreleased" }
        [pscustomobject]@{ Label = (T 'state_favorites'); Value = "favorites" }
    )
}

function Get-HtbCompletedOptions {
    @(
        [pscustomobject]@{ Label = (T 'filter_any'); Value = "" }
        [pscustomobject]@{ Label = (T 'filter_solved'); Value = "complete" }
        [pscustomobject]@{ Label = (T 'filter_unsolved'); Value = "incomplete" }
    )
}

function Set-HtbFavorite([string]$Kind, $Id) {
    $paths = switch ($Kind) {
        "machine"   { @("/machine/todo/update/$Id") }
        "challenge" { @("/challenge/todo/update/$Id") }
        default     { @("/sherlock/todo/update/$Id", "/sherlocks/todo/update/$Id", "/sherlocks/$Id/todo") }
    }
    Write-Host ""
    $result = $null
    foreach ($p in $paths) {
        $r = HtbV4Post $p @{}
        if ($r.Code -ne 404 -and $r.Code -ne 405) { $result = $r; break }
    }
    if ($null -eq $result) { Write-HtbNote (T 'not_in_api'); Wait-HtbEnter; return }
    if (-not $result.Ok) { Write-HtbFail "$(HtbValue $result.Data 'message')"; Wait-HtbEnter; return }
    $ids = @(if ($result.Data.info) { $result.Data.info } else { $result.Data.data })
    $now = $false
    foreach ($v in $ids) { if ("$($v.id)" -eq "$Id" -or "$v" -eq "$Id") { $now = $true } }
    if ($now) { Write-HtbOk (T 'favorite_added') } else { Write-HtbOk (T 'favorite_removed') }
    Wait-HtbEnter
}

function Show-HtbReviews([string]$Kind, $Id, [string]$Title) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $r = HtbV4Get "/review/$Kind/$Id/paginated?sort_by[]=created_at&sort_type=desc&per_page=40"
    $rows = @($r.Data.data)
    $lines = @("")
    if ($rows.Count -eq 0) {
        $lines += "  $Faint$(T 'reviews_none')$Reset"
    } else {
        $lines += "  $Bold$White$(T 'reviews_title') · $($r.Data.meta.total)$Reset"
        $lines += ""
        foreach ($rv in $rows) {
            $st = [int]$rv.stars
            $stars = ("★" * [Math]::Min(5, $st)) + ("☆" * (5 - [Math]::Min(5, $st)))
            $lines += "  $Yellow$stars$Reset $White$($rv.title)$Reset   $Faint$($rv.user.name) · $(Get-HtbDate $rv.created_at)$Reset"
            if ($rv.message) { $lines += (Format-HtbParagraph "$($rv.message)") }
            if ([int]$rv.helpful_reviews_count -gt 0) { $lines += "    $Faint$(T 'reviews_helpful') $($rv.helpful_reviews_count)$Reset" }
            $lines += ""
        }
    }
    Show-HtbLines "$(T 'reviews_title') · $Title" $Cyan $lines
}

function Show-HtbActivity([string]$Kind, $Id, [string]$Title) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $r = HtbV4Get "/$Kind/activity/$Id"
    $rows = @(if ($r.Data.info.activity) { $r.Data.info.activity } else { $r.Data.data })
    $lines = @("")
    if ($rows.Count -eq 0) { $lines += "  $Faint$(T 'activity_none')$Reset" }
    foreach ($a in ($rows | Select-Object -First 80)) {
        $what = if ($a.blood_type) { "$($a.blood_type) blood" } elseif ($a.type -eq "blood") { "blood" } elseif ($a.type -eq "own") { "solved" } else { "$($a.type) flag" }
        $color = if ($a.blood_type -or $a.type -eq "blood") { $Red } elseif ($a.type -eq "root" -or $a.type -eq "system" -or $a.type -eq "own") { $Green } else { $Yellow }
        $lines += ("  $White" + "$($a.user_name)".PadRight(24) + "$Reset $color" + $what.PadRight(14) + "$Reset $Faint$($a.date_diff)$Reset")
    }
    Show-HtbLines "$(T 'activity_title') · $Title" $Cyan $lines
}

function Show-HtbChangelog([string]$Kind, $Id, [string]$Title) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $r = HtbV4Get "/$Kind/changelog/$Id"
    $rows = @(if ($r.Data.info) { $r.Data.info } else { $r.Data.data })
    $lines = @("")
    if ($rows.Count -eq 0) { $lines += "  $Faint$(T 'changelog_none')$Reset" }
    foreach ($c in $rows) {
        $kind = switch ("$($c.type)") { "1" { "ENHANCEMENT" } "enhancement" { "ENHANCEMENT" } "2" { "INFO" } "info" { "INFO" } "3" { "BUG" } "bug" { "BUG" } default { "$($c.type)" } }
        $lines += "  $Faint$(Get-HtbDate $c.created_at)$Reset  $Yellow[$kind]$Reset $White$($c.title)$Reset"
        if ($c.description) { $lines += (Format-HtbParagraph "$($c.description)") }
        $lines += ""
    }
    Show-HtbLines "$(T 'changelog_title') · $Title" $Cyan $lines
}

function Show-HtbUrl([string]$Url) {
    Copy-HtbText $Url | Out-Null
    try { Open-HtbUrl $Url } catch { Show-HtbCopied $Url }
}

function Show-HtbWalkthroughs($Id, [string]$Title) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $r = HtbV4Get "/machine/walkthroughs/$Id"
    $msg = $r.Data.message
    $labels = @(); $urls = @()
    if ($msg.official -and $msg.official.filename) { $labels += "$(T 'walkthrough_official')  (PDF)"; $urls += "official" }
    if ($msg.video -and $msg.video.youtube_id) {
        $labels += "$(T 'walkthrough_video')  $($msg.video.creator_name)  youtube"
        $urls += "https://www.youtube.com/watch?v=$($msg.video.youtube_id)"
    }
    foreach ($w in @($msg.writeups | Select-Object -First 60)) {
        $user = "$($w.user_name)"; if ($user.Length -gt 18) { $user = $user.Substring(0, 18) }
        $labels += ("{0,-18} {1,3}% ({2})  {3}  {4}" -f $user, [int]$w.rating, [int]$w.total_ratings, (Get-HtbDate $w.created_at), "$($w.language_name)")
        $urls += "$($w.url)"
    }
    if ($labels.Count -eq 0) { Write-Host ""; Write-HtbNote (T 'walkthrough_none'); Wait-HtbEnter; return }
    $at = 0
    while ($true) {
        Select-HtbItem -Title "$(T 'walkthroughs_title') · $Title" -Color $Cyan -Hint (Write-HtbKeys "↵|$(T 'open_in_browser')") -Items $labels -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' {
                $url = $urls[$ReplySel]
                if ($url -eq "official") {
                    $w = HtbV4Get "/machine/writeup/$Id"
                    $url = "$(HtbValue $w.Data 'url')"
                    if ($url -eq "-") { Write-Host ""; Write-HtbFail "$(HtbValue $w.Data 'message')"; Wait-HtbEnter; continue }
                }
                Show-HtbUrl $url
            }
        }
    }
}

function Show-HtbMachineInfo($M, $Id) {
    $name = "$(HtbValue $M 'name')"
    $lines = @("", "  $Bold$White$name$Reset", "")
    if ($M.synopsis) { $lines += (Format-HtbParagraph "$($M.synopsis)"); $lines += "" }
    $bloods = @()
    if ($M.userBlood -and $M.userBlood.user) { $bloods += ("  $Grey" + "user".PadRight(6) + "$Reset$Red" + "$($M.userBlood.user.name)".PadRight(24) + "$Reset $Faint$(Get-HtbDate $M.userBlood.created_at) · $($M.userBlood.blood_difference)$Reset") }
    if ($M.rootBlood -and $M.rootBlood.user) { $bloods += ("  $Grey" + "root".PadRight(6) + "$Reset$Red" + "$($M.rootBlood.user.name)".PadRight(24) + "$Reset $Faint$(Get-HtbDate $M.rootBlood.created_at) · $($M.rootBlood.blood_difference)$Reset") }
    if ($bloods.Count -gt 0) { $lines += "  $Cyan── $(T 'info_bloods')$Reset"; $lines += $bloods; $lines += "" }
    $tags = @((HtbV4Get "/machine/tags/$Id").Data.info)
    if ($tags.Count -gt 0) {
        $lines += "  $Cyan── $(T 'info_tags')$Reset"
        foreach ($g in ($tags | Group-Object category)) {
            $lines += ("  $Grey" + "$($g.Name)".PadRight(18) + "$Reset$Purple" + (($g.Group | ForEach-Object { $_.name }) -join ", ") + "$Reset")
        }
        $lines += ""
    }
    $matrix = (HtbV4Get "/machine/graph/matrix/$Id").Data.info.aggregate
    if ($matrix) {
        $lines += "  $Cyan── $(T 'info_matrix')$Reset"
        foreach ($k in @("enum", "real", "cve", "custom", "ctf")) {
            $v = [double]$matrix.$k
            $width = [int][Math]::Round([Math]::Max(0, [Math]::Min(10, $v)) * 3)
            $lines += ("  $Grey" + $k.PadRight(8) + "$Reset$Green" + ("█" * $width) + "$Reset $Faint$("{0:N1}" -f $v)$Reset")
        }
        $lines += ""
    }
    if ($M.info_status) { $lines += (Format-HtbParagraph "$($M.info_status)") }
    Show-HtbLines "$(T 'info_title') · $name" $Cyan $lines
}

function Get-HtbLiveStatus($Id) {
    $act = (HtbV4Get "/machine/active").Data.info
    if (-not $act -or "$($act.id)" -ne "$Id") { return $null }
    $ip = "$(HtbValue $act 'ip')"
    return [pscustomobject]@{ Ip = $ip; Expires = "$(HtbValue $act 'expires_at')"; Spawning = ($ip -eq "-" -or $ip -eq "") }
}

function Show-HtbGlobalSearch {
    Clear-Host; Write-HtbTitle (T 'machines_global_search') $White; Write-Host ""
    $term = Read-HtbText (T 'ask_search')
    if (-not $term) { return }
    Write-Host "  $Grey$(T 'loading')$Reset"
    $r = HtbV4Get "/search/fetch?query=$([uri]::EscapeDataString($term))"
    $rows = @(); $kinds = @(); $ids = @()
    $groups = @(
        [pscustomobject]@{ Key = "machines"; Label = (T 'search_machines') }
        [pscustomobject]@{ Key = "challenges"; Label = (T 'search_challenges') }
        [pscustomobject]@{ Key = "users"; Label = (T 'search_users') }
        [pscustomobject]@{ Key = "teams"; Label = (T 'search_teams') }
    )
    foreach ($pair in $groups) {
        $key = $pair.Key; $label = $pair.Label
        foreach ($v in @(@($r.Data.$key) | Where-Object { $null -ne $_ })) {
            $extra = if ($key -eq "machines" -and $v.isSp) { "  Starting Point" } else { "" }
            $rows += ("$Grey" + $label.PadRight(10) + "$Reset $White$($v.value)$Reset$extra")
            $kinds += $key; $ids += "$($v.id)"
        }
    }
    if ($rows.Count -eq 0) { Write-Host ""; Write-HtbNote (T 'no_results'); Wait-HtbEnter; return }
    $at = 0
    while ($true) {
        Select-HtbItem -Title "$(T 'machines_global_search'): $term" -Color $Green -Items $rows -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' {
                switch ($kinds[$ReplySel]) {
                    'machines'   { Show-HtbMachineDetail $ids[$ReplySel] }
                    'challenges' { Show-HtbChallengeDetail $ids[$ReplySel] }
                    'users'      { Show-HtbUserPage $ids[$ReplySel] }
                    'teams'      { Show-HtbUrl "$HtbWeb/teams/$($ids[$ReplySel])" }
                }
            }
        }
    }
}

function Show-HtbUserPage($Uid) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $b = (HtbV4Get "/user/profile/basic/$Uid").Data.profile
    if (-not $b -or -not $b.name) { Write-Host ""; Write-HtbFail (T 'api_bad_response'); Wait-HtbEnter; return }
    $lines = @(
        ""
        "  $Bold$White$($b.name)$Reset"
        ""
        ("  $Grey" + (T 'p_rank').PadRight(18) + "$Reset$Yellow$(HtbValue $b 'rank')$Reset")
        ("  $Grey" + (T 'stats_global_rank').PadRight(18) + "$Reset#$(HtbValue $b 'ranking')")
        ("  $Grey" + (T 'p_points').PadRight(18) + "$Reset$(HtbValue $b 'points')")
        ("  $Grey" + (T 'p_owns').PadRight(18) + "$Reset" + "user $(HtbValue $b 'user_owns') · system $(HtbValue $b 'system_owns')")
        ("  $Grey" + (T 'p_bloods').PadRight(18) + "$Reset" + "user $(HtbValue $b 'user_bloods') · system $(HtbValue $b 'system_bloods')")
        ("  $Grey" + (T 'p_respects').PadRight(18) + "$Reset$(HtbValue $b 'respects')")
        ("  $Grey" + (T 'p_country').PadRight(18) + "$Reset$(HtbValue $b 'country_name')")
        ("  $Grey" + (T 'p_team').PadRight(18) + "$Reset$(HtbValue $b 'team.name')")
        ""
        "  $Faint$HtbWeb/users/$Uid$Reset"
    )
    Show-HtbLines "$(T 'user_view_title') · $($b.name)" $Cyan $lines
}

function Show-HtbDashboard([string]$Which, [string]$Title) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $r = HtbV5Get "/user/dashboard/$Which"
    $rows = @(); $kinds = @(); $ids = @()
    $groups = @(
        [pscustomobject]@{ Key = "machines"; Kind = "machine" }
        [pscustomobject]@{ Key = "startingPoints"; Kind = "machine" }
        [pscustomobject]@{ Key = "sherlocks"; Kind = "sherlock" }
        [pscustomobject]@{ Key = "challenges"; Kind = "challenge" }
    )
    foreach ($pair in $groups) {
        $key = $pair.Key; $kind = $pair.Kind
        foreach ($v in @(@($r.Data.$key) | Where-Object { $null -ne $_ })) {
            $mark = if ($v.user_flag -and $v.root_flag) { "✓✓" } elseif ($v.user_flag -or $v.root_flag) { "✓ " } else { "  " }
            $mc = if ($v.root_flag) { $Green } elseif ($v.user_flag) { $Yellow } else { $Faint }
            $os = "$($v.os)"; if ($os -eq "-") { $os = "" }
            $diff = "$($v.difficulty)"
            $label = if ($key -eq "startingPoints") { "sp" } else { $kind }
            $name = "$($v.name)"; if ($name.Length -gt 28) { $name = $name.Substring(0, 28) }
            $rows += ("$mc$mark$Reset $Grey" + $label.PadRight(9) + "$Reset$White" + $name.PadRight(28) + "$Reset $Cyan" + $os.PadRight(9) + "$Reset$(Get-HtbDifficultyColor $diff)" + $diff.PadRight(10) + "$Reset $Faint$(T 'f_progress') $([int][double]$v.progress)% · $(T 'f_tasks') $($v.tasksCompleted)/$($v.tasksTotal)$Reset")
            $kinds += $kind; $ids += "$($v.id)"
        }
    }
    if ($rows.Count -eq 0) { Write-Host ""; Write-HtbNote (T 'dashboard_empty'); Wait-HtbEnter; return }
    $at = 0
    while ($true) {
        Select-HtbItem -Title $Title -Color $Green -Items $rows -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' {
                switch ($kinds[$ReplySel]) {
                    'machine'   { Show-HtbMachineDetail $ids[$ReplySel] }
                    'sherlock'  { Show-HtbSherlockDetail $ids[$ReplySel] }
                    'challenge' { Show-HtbChallengeDetail $ids[$ReplySel] }
                }
            }
        }
    }
}

function Show-HtbSeasonLeaderboard($Sid, [string]$Name) {
    $me = (HtbV4Get "/season/user/rank/$Sid").Data.data
    $myRank = "$(HtbValue $me 'rank')"
    $page = 1
    while ($true) {
        Write-Host "  $Grey$(T 'loading')$Reset"
        $r = HtbV4Get "/season/players/leaderboard?per_page=15&season=$Sid&page=$page"
        $last = if ($r.Data.meta.last_page) { [int]$r.Data.meta.last_page } else { 1 }
        Clear-Host; Write-HtbTitle "$(T 'season_leaderboard') · $Name · $(T 'act_page') $page/$last" $Purple; Write-Host ""
        Write-Host ("  $Grey" + "#".PadRight(6) + "".PadRight(24) + "tier".PadRight(10) + "pts".PadLeft(7) + "user".PadLeft(7) + "root".PadLeft(7) + "bloods".PadLeft(7) + "$Reset")
        foreach ($p in @($r.Data.data)) {
            $mine = ("$($p.rank)" -eq $myRank)
            $c = if ($mine) { $Green } else { $White }
            $rc = if ($mine) { $Green } else { $Yellow }
            $n = "$($p.name)"; if ($n.Length -gt 22) { $n = $n.Substring(0, 22) }
            $bl = [int]$p.user_bloods + [int]$p.root_bloods
            Write-Host ("  $rc" + "$($p.rank)".PadRight(6) + "$Reset$c" + $n.PadRight(24) + "$Reset$Purple" + "$($p.league_rank)".PadRight(10) + "$Reset" + "$($p.points)".PadLeft(7) + "$Faint" + "$($p.user_owns)".PadLeft(7) + "$($p.root_owns)".PadLeft(7) + "$Reset$Red" + "$bl".PadLeft(7) + "$Reset")
        }
        if ($myRank -ne "-") {
            Write-Host ""
            Write-Host "  $Green#$myRank$Reset  $(T 'season_you') · $(HtbValue $me 'league') · $(HtbValue $me 'total_season_points') $(T 'season_points') · $(HtbValue $me 'total_season_flags.obtained') $(T 'season_flags')"
        }
        Write-Host ""
        Write-Host (Write-HtbKeys "n|$(T 'page_next')" "p|$(T 'page_previous')")
        $k = Read-HtbKey
        switch -Regex ($k) {
            '^CHAR:[nN]$' { if ($page -lt $last) { $page++ } }
            '^CHAR:[pP]$' { if ($page -gt 1) { $page-- } }
            '^(BACK|QUIT|CHAR:[qQ])$' { return }
        }
    }
}

function Show-HtbSeasonRewards($Sid, [string]$Name) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $r = HtbV4Get "/season/rewards/$Sid"
    $lines = @("")
    foreach ($block in @($r.Data.data)) {
        foreach ($t in @($block.reward_types)) {
            $lines += "  $Cyan── $($t.name)$Reset"
            if ($t.description) { $lines += (Format-HtbParagraph "$($t.description)") }
            foreach ($g in @($t.groups)) {
                $req = if ($g.rank_up_requirement) { "top $($g.rank_up_requirement)  " } else { "" }
                $flags = if ($g.flags_needed) { "$($g.flags_needed) $(T 'season_flags')" } else { "" }
                $lines += ("  $Purple" + "$($g.name)".PadRight(12) + "$Reset$Grey" + "$($g.subtitle)".PadRight(16) + "$Reset$Faint$req$flags$Reset")
            }
            $lines += ""
        }
    }
    if ($lines.Count -le 1) { $lines += "  $Faint$(T 'no_results')$Reset" }
    Show-HtbLines "$(T 'season_rewards') · $Name" $Purple $lines
}

function Show-HtbRankings([string]$Kind, [string]$Title) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $r = HtbV4Get "/rankings/$Kind"
    $rows = @($r.Data.data)
    $third = switch ($Kind) { "countries" { "members" } "universities" { "students" } default { "" } }
    $lines = @("", ("  $Grey" + "#".PadRight(6) + "".PadRight(28) + $third.PadLeft(9) + "pts".PadLeft(10) + "user".PadLeft(8) + "root".PadLeft(8) + "$Reset"))
    foreach ($e in ($rows | Select-Object -First 150)) {
        $name = if ($Kind -eq "countries") { if ($e.country_name) { "$($e.country)  $($e.country_name)" } else { "$($e.country)" } } else { "$($e.name)" }
        if ($name.Length -gt 26) { $name = $name.Substring(0, 26) }
        $t = switch ($Kind) { "countries" { "$($e.members)" } "universities" { "$($e.students)" } default { "" } }
        $lines += ("  $Yellow" + "$($e.rank)".PadRight(6) + "$Reset$White" + $name.PadRight(28) + "$Reset$Faint" + $t.PadLeft(9) + "$Reset$Green" + "$($e.points)".PadLeft(10) + "$Reset$Faint" + "$($e.user_owns)".PadLeft(8) + "$($e.root_owns)".PadLeft(8) + "$Reset")
    }
    if ($rows.Count -eq 0) { $lines += "  $Red$(T 'api_bad_response')$Reset" }
    Show-HtbLines $Title $Cyan $lines
}

function Show-HtbPwnbox {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $status = (HtbV4Get "/pwnbox/status").Data
    $usage = (HtbV4Get "/pwnbox/usage").Data
    $lines = @("")
    if (-not $status.status) {
        $lines += "  $Faint$(T 'pwnbox_none')$Reset"
        if ($status.message) { $lines += "  $Faint$($status.message)$Reset" }
    } else {
        $lines += ("  $Grey" + (T 'vm_label').PadRight(18) + "$Reset$Green$($status.status)$Reset")
        foreach ($k in @("hostname", "vnc_view_only_password", "vnc_password", "expires_at", "location")) {
            if ($status.$k) { $lines += ("  $Grey" + $k.PadRight(18) + "$Reset$($status.$k)") }
        }
    }
    $lines += ""
    $lines += ("  $Grey" + (T 'pwnbox_usage').PadRight(28) + "$Reset$(HtbValue $usage 'used') / $(HtbValue $usage 'allowed')")
    $lines += ("  $Grey" + "sessions".PadRight(28) + "$Reset$(HtbValue $usage 'sessions')")
    Show-HtbLines (T 'pwnbox_labs') $Purple $lines
}
