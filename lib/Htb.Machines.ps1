function Show-HtbMachinesMenu {
    $items = @(
        (Write-HtbRow (T 'machines_active')  (T 'machines_active_desc'))
        (Write-HtbRow (T 'machines_retired') (T 'machines_retired_desc'))
        (Write-HtbRow (T 'machines_search')  (T 'machines_search_desc'))
        (Write-HtbRow (T 'machines_target')  (T 'machines_target_desc'))
        (Write-HtbRow (T 'machines_sp')      (T 'machines_sp_desc'))
    )
    $at = 0
    while ($true) {
        Select-HtbItem -Title (T 'machines_title') -Color $Green -Items $items -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' {
                switch ($ReplySel) {
                    0 { Show-HtbMachines (T 'machines_active') "active" "" }
                    1 { Show-HtbMachines (T 'machines_retired') "retired" "" }
                    2 {
                        Clear-Host; Write-HtbTitle (T 'machines_search') $White; Write-Host ""
                        $term = Read-HtbText (T 'ask_search')
                        if ($term) { Show-HtbMachines "$(T 'machines_search'): $term" "" $term }
                    }
                    3 { Show-HtbTarget }
                    4 { Show-HtbStartingPoint }
                }
            }
        }
    }
}

function Get-HtbMachineRows($Machines) {
    $rows = @()
    foreach ($m in $Machines) {
        $mark = if ($m.authUserInUserOwns -and $m.authUserInRootOwns) { "✓✓" }
                elseif ($m.authUserInUserOwns -or $m.authUserInRootOwns) { "✓ " } else { "  " }
        $markColor = if ($mark -eq "✓✓") { $Green } elseif ($mark -eq "✓ ") { $Yellow } else { $Faint }
        $difficulty = if ($m.difficultyText) { "$($m.difficultyText)" } else { "$($m.difficulty)" }
        $access = if ($m.free) { "Free" } else { "VIP" }
        $released = if ($m.release) { Get-HtbDate $m.release } else { Get-HtbDate $m.releaseDate }
        $name = "$($m.name)"
        if ($name.Length -gt 26) { $name = $name.Substring(0, 26) }
        $rows += ("$markColor$mark$Reset $White" + $name.PadRight(26) + "$Reset $Cyan" +
                  "$($m.os)".PadRight(8) + "$Reset $(Get-HtbDifficultyColor $difficulty)" +
                  $difficulty.PadRight(9) + "$Reset $Yellow" + "$($m.points)".PadLeft(4) +
                  " pts$Reset  $Faint" + "$($m.userOwnsCount)".PadLeft(6) + " owns · " +
                  $access.PadRight(4) + " · $released$Reset")
    }
    return $rows
}

function Show-HtbMachines([string]$Title, [string]$State, [string]$Keyword) {
    $page = 1; $difficulty = ""; $os = ""; $solved = ""; $reload = $true; $at = 0
    $machines = @(); $total = "?"; $last = 1
    $diffOptions = @(
        [pscustomobject]@{ Label = (T 'filter_any'); Value = "" }
        [pscustomobject]@{ Label = "Very Easy"; Value = "very-easy" }
        [pscustomobject]@{ Label = "Easy";      Value = "easy" }
        [pscustomobject]@{ Label = "Medium";    Value = "medium" }
        [pscustomobject]@{ Label = "Hard";      Value = "hard" }
        [pscustomobject]@{ Label = "Insane";    Value = "insane" }
    )
    $osOptions = @(
        [pscustomobject]@{ Label = (T 'filter_any'); Value = "" }
        [pscustomobject]@{ Label = "Linux";   Value = "linux" }
        [pscustomobject]@{ Label = "Windows"; Value = "windows" }
        [pscustomobject]@{ Label = "FreeBSD"; Value = "freebsd" }
        [pscustomobject]@{ Label = "OpenBSD"; Value = "openbsd" }
        [pscustomobject]@{ Label = "Android"; Value = "android" }
        [pscustomobject]@{ Label = "Other";   Value = "other" }
    )
    $solvedOptions = @(
        [pscustomobject]@{ Label = (T 'filter_any');      Value = "" }
        [pscustomobject]@{ Label = (T 'filter_solved');   Value = "solved" }
        [pscustomobject]@{ Label = (T 'filter_unsolved'); Value = "unsolved" }
    )
    while ($true) {
        if ($reload) {
            $query = "per_page=25&page=$page"
            if ($State) { $query += "&state=$State" }
            if ($Keyword) { $query += "&keyword=$([uri]::EscapeDataString($Keyword))" }
            if ($difficulty) { $query += "&difficulty[]=$([uri]::EscapeDataString($difficulty))" }
            if ($os) { $query += "&os[]=$([uri]::EscapeDataString($os))" }
            Write-Host "  $Grey$(T 'loading')$Reset"
            $response = HtbV5Get "/machines?$query"
            $machines = @($response.Data.data)
            if ($solved -eq "solved") {
                $machines = @($machines | Where-Object { $_.authUserInUserOwns -or $_.authUserInRootOwns })
            } elseif ($solved -eq "unsolved") {
                $machines = @($machines | Where-Object { -not ($_.authUserInUserOwns -or $_.authUserInRootOwns) })
            }
            $total = "$($response.Data.meta.total)"
            $last = if ($response.Data.meta.last_page) { [int]$response.Data.meta.last_page } else { 1 }
            $reload = $false
        }
        if ($machines.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }

        $shown = if ($solved) { $machines.Count } else { $total }
        $header = "$Title"
        if ($difficulty) { $header += " · $(Get-HtbFilterLabel $difficulty $diffOptions)" }
        if ($os) { $header += " · $(Get-HtbFilterLabel $os $osOptions)" }
        if ($solved) { $header += " · $(Get-HtbFilterLabel $solved $solvedOptions)" }
        $header += " · $shown · $(T 'act_page') $page/$last"
        $hint = Write-HtbKeys "s|$(T 'act_spawn')" "f|$(T 'act_flag')" "t|$(T 'act_stop')" "r|$(T 'act_reset')" "e|$(T 'act_extend')" "n|$(T 'page_next')" "p|$(T 'page_previous')" "d|$(T 'act_filter_difficulty')" "o|$(T 'act_filter_os')" "v|$(T 'act_filter_solved')" "x|$(T 'act_filter_clear')" "c|$(T 'act_copy')"

        Select-HtbItem -Title $header -Color $Green -Hint $hint -Items (Get-HtbMachineRows $machines) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        $target = if ($ReplySel -ge 0) { $machines[$ReplySel].id } else { $null }

        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' { Show-HtbMachineDetail $target }
            '^[sS]$'  { Start-HtbMachine $target; $reload = $true }
            '^[fF]$'  { Submit-HtbMachineFlag $target; $reload = $true }
            '^[tT]$'  { Invoke-HtbMachineAction "/vm/terminate" (T 'act_stop')   (T 'confirm_stop');  $reload = $true }
            '^[rR]$'  { Invoke-HtbMachineAction "/vm/reset"     (T 'act_reset')  (T 'confirm_reset') }
            '^[eE]$'  { Invoke-HtbMachineAction "/vm/extend"    (T 'act_extend') "" }
            '^[nN]$'  { if ($page -lt $last) { $page++; $reload = $true } }
            '^[pP]$'  { if ($page -gt 1) { $page--; $reload = $true } }
            '^[dD]$'  {
                Select-HtbFilter (T 'act_filter_difficulty') $diffOptions
                if ($ReplyFilterSet) { $difficulty = $ReplyFilter; $page = 1; $at = 0; $reload = $true }
            }
            '^[oO]$'  {
                Select-HtbFilter (T 'act_filter_os') $osOptions
                if ($ReplyFilterSet) { $os = $ReplyFilter; $page = 1; $at = 0; $reload = $true }
            }
            '^[vV]$'  {
                Select-HtbFilter (T 'act_filter_solved') $solvedOptions
                if ($ReplyFilterSet) { $solved = $ReplyFilter; $at = 0; $reload = $true }
            }
            '^[xX]$'  { $difficulty = ""; $os = ""; $solved = ""; $page = 1; $at = 0; $reload = $true }
            '^[cC]$'  { Show-HtbCopied "$(HtbValue (HtbV4Get '/machine/active').Data.info 'ip')" }
        }
    }
}

function Select-HtbRating {
    $Script:ReplyRating = 0
    $names = @(1..10 | ForEach-Object { "{0,2}  {1}" -f $_, (T "rate_$_") })
    Select-HtbItem -Title (T 'rate_title') -Color $Cyan -Items $names
    if ($ReplyKey -eq "enter") { $Script:ReplyRating = $ReplySel + 1 }
}

function Show-HtbStartingPoint {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $response = HtbV4Get "/sp/tiers/progress"
    if (-not (Test-HtbApi $response)) { return }
    $tiers = @($response.Data.data)
    if ($tiers.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }

    $at = 0
    while ($true) {
        $rows = foreach ($t in $tiers) {
            $pct = [int]($t.completion_percentage)
            $filled = [int][Math]::Round($pct * 20 / 100)
            $bar = ("█" * $filled) + ("░" * (20 - $filled))
            ("$White" + "$($t.name)".PadRight(10) + "$Reset $Green$bar$Reset $Cyan" +
             ("{0,3}%" -f $pct) + "$Reset  $Faint$($t.description)$Reset")
        }
        Select-HtbItem -Title (T 'machines_sp') -Color $Green -Items @($rows) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' { Show-HtbSpTier $tiers[$ReplySel].id "$($tiers[$ReplySel].name)" }
        }
    }
}

function Show-HtbSpTier($Tier, [string]$TierName) {
    $at = 0; $reload = $true; $machines = @()
    while ($true) {
        if ($reload) {
            Write-Host "  $Grey$(T 'loading')$Reset"
            $machines = @((HtbV5Get "/machines?spTier=$Tier").Data.data)
            $reload = $false
        }
        if ($machines.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }
        $hint = Write-HtbKeys "s|$(T 'act_spawn')" "f|$(T 'act_flag')" "t|$(T 'act_stop')" "c|$(T 'act_copy')"
        Select-HtbItem -Title "$(T 'machines_sp') · $TierName · $($machines.Count)" -Color $Green -Hint $hint -Items (Get-HtbMachineRows $machines) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        $target = if ($ReplySel -ge 0) { $machines[$ReplySel].id } else { $null }
        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' { Show-HtbMachineDetail $target }
            '^[sS]$'  { Start-HtbMachine $target; $reload = $true }
            '^[fF]$'  { Submit-HtbMachineFlag $target; $reload = $true }
            '^[tT]$'  { Invoke-HtbMachineAction "/vm/terminate" (T 'act_stop') (T 'confirm_stop'); $reload = $true }
            '^[cC]$'  { Show-HtbCopied "$(HtbValue (HtbV4Get '/machine/active').Data.info 'ip')" }
        }
    }
}

function Get-HtbTaskRows($Id) {
    $tasks = @((HtbV4Get "/machines/$Id/tasks").Data.data | Where-Object { $null -ne $_ })
    if ($tasks.Count -eq 0) {
        $tasks = @((HtbV4Get "/machines/$Id/adventure").Data.data | Where-Object { $null -ne $_ })
    }
    return $tasks
}

function Format-HtbTaskRows($Tasks) {
    $rows = @()
    foreach ($task in $Tasks) {
        $done = [bool]$task.completed
        $mark = if ($done) { "✓" } else { " " }
        $markColor = if ($done) { $Green } else { $Faint }
        $title = "$($task.title)"; if ($title.Length -gt 20) { $title = $title.Substring(0, 20) }
        $desc = ("$($task.description)" -replace '[\r\n\t]+', ' ')
        if ($desc.Length -gt 78) { $desc = $desc.Substring(0, 78) }
        $extra = if ($done -and $task.flag) { "$(T 'task_answer'): $($task.flag)" }
                 else { "$(T 'task_expected'): $($task.masked_flag)" }
        if ($extra.Length -gt 44) { $extra = $extra.Substring(0, 44) }
        $rows += ("$markColor$mark$Reset $Grey" + $title.PadRight(20) + "$Reset $White" +
                  $desc.PadRight(78) + "$Reset  $Faint$extra$Reset")
    }
    return $rows
}

function Get-HtbProgressLine($Tasks) {
    $done = @($Tasks | Where-Object { $_.completed }).Count
    $color = if ($done -eq $Tasks.Count) { $Green } else { $Yellow }
    return ("  $Grey" + (T 'f_progress').PadRight(18) + "$Reset$color$done/$($Tasks.Count) $(T 'tasks_answered')$Reset")
}

function Get-HtbRatingLabel($Chart) {
    $keys = @('counterCake','counterVeryEasy','counterEasy','counterTooEasy','counterMedium',
              'counterBitHard','counterHard','counterTooHard','counterExHard','counterBrainFuck')
    $counts = @($keys | ForEach-Object { [int]($Chart.$_) })
    $total = ($counts | Measure-Object -Sum).Sum
    if (-not $total -or $total -eq 0) { return "-" }
    $weighted = 0
    for ($i = 0; $i -lt 10; $i++) { $weighted += ($i + 1) * $counts[$i] }
    $idx = [Math]::Round($weighted / $total)
    if ($idx -lt 1) { $idx = 1 }; if ($idx -gt 10) { $idx = 10 }
    return "$(T "rate_$idx") ($total)"
}

function Show-HtbVotes($Chart) {
    $keys = @('counterCake','counterVeryEasy','counterEasy','counterTooEasy','counterMedium',
              'counterBitHard','counterHard','counterTooHard','counterExHard','counterBrainFuck')
    $counts = @($keys | ForEach-Object { [int]($Chart.$_) })
    $top = ($counts | Measure-Object -Maximum).Maximum
    if (-not $top -or $top -eq 0) { Write-Host ""; Write-HtbNote (T 'no_results'); Wait-HtbEnter; return }
    $lines = @("")
    for ($i = 0; $i -lt 10; $i++) {
        $width = [int][Math]::Round($counts[$i] * 24 / $top)
        $lines += ("  $Grey" + (T "rate_$($i+1)").PadRight(18) + "$Reset$Green" + ("█" * $width) +
                   "$Reset $Faint$($counts[$i])$Reset")
    }
    Show-HtbLines (T 'votes_title') $Cyan $lines
}


function Submit-HtbMachineTask($Id, $TaskId) {
    Write-Host ""
    $answer = Read-HtbFlagText (T 'task_answer')
    if (-not $answer) { return }
    $result = if ($null -eq $TaskId -or "$TaskId" -eq "") {
        HtbV5Post "/machine/own" @{ flag = $answer; id = [int]$Id; difficulty = 5 }
    } else {
        HtbV4Post "/machines/$Id/tasks/$TaskId/flag" @{ flag = $answer }
    }
    $message = "$(HtbValue $result.Data 'message')"
    if ((Read-HtbFlagAnswer $message) -eq 'si') {
        Write-Host "  $Green$Bold$(T 'task_accepted')$Reset — $message"
    } else { Write-HtbFail $message }
    Wait-HtbEnter
}

function Show-HtbMachineDetail($Id) {
    $at = 0; $reload = $true
    $header = @(); $tasks = @(); $rows = @(); $m = $null; $name = ""
    while ($true) {
        if ($reload) {
            Write-Host "  $Grey$(T 'loading')$Reset"
            $m = (HtbV4Get "/machine/profile/$Id").Data.info
            $name = "$(HtbValue $m 'name')"
            $difficulty = "$(HtbValue $m 'difficultyText')"
            $owns = if ($m.isSingleFlag) { "$(HtbValue $m 'root_owns_count')" }
                    else { "user $(HtbValue $m 'user_owns_count') · root $(HtbValue $m 'root_owns_count')" }
            $tasks = @(Get-HtbTaskRows $Id)
            $rows = @(Format-HtbTaskRows $tasks)
            $header = @(
                "  $Bold$White$name$Reset"
                ""
                ("  $Grey" + "ID".PadRight(18) + "$Reset$Id")
                ("  $Grey" + (T 'f_os').PadRight(18) + "$Reset$(HtbValue $m 'os')")
                ("  $Grey" + (T 'f_difficulty').PadRight(18) + "$Reset$(Get-HtbDifficultyColor $difficulty)$difficulty$Reset")
                ("  $Grey" + (T 'f_points').PadRight(18) + "$Reset$Yellow$(HtbValue $m 'points')$Reset")
                ("  $Grey" + (T 'f_ip').PadRight(18) + "$Reset$Cyan$(HtbValue $m 'playInfo.ip')$Reset")
                ("  $Grey" + (T 'f_owns').PadRight(18) + "$Reset$owns")
                ("  $Grey" + (T 'f_rating').PadRight(18) + "$Reset$(HtbValue $m 'stars')")
                ("  $Grey" + (T 'f_user_rating').PadRight(18) + "$Reset$(Get-HtbRatingLabel $m.feedbackForChart)")
                ("  $Grey" + (T 'f_released').PadRight(18) + "$Reset$(Get-HtbDate $m.release)")
                ("  $Grey" + (T 'f_author').PadRight(18) + "$Reset$(HtbValue $m 'maker.name')")
            )
            if ($tasks.Count -gt 0) { $header += (Get-HtbProgressLine $tasks) }
            $header += ""
            $reload = $false
        }
        $keys = @("s|$(T 'act_spawn')")
        if ($tasks.Count -gt 0) { $keys += "↵|$(T 'task_answer')" }
        $keys += @("f|$(T 'act_flag')", "t|$(T 'act_stop')", "r|$(T 'act_reset')",
                   "e|$(T 'act_extend')", "v|$(T 'votes_title')", "c|$(T 'act_copy')")
        $items = if ($tasks.Count -gt 0) { $rows } else { @("  $Faint$(T 'no_tasks')$Reset") }
        Select-HtbItem -Title $name -Color $Green -Hint (Write-HtbKeys @keys) -Items @($items) -Start $at -Header $header
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' {
                if ($tasks.Count -gt 0) {
                    Submit-HtbMachineTask $Id $tasks[$ReplySel].id
                    $reload = $true
                }
            }
            '^[sS]$'  { Start-HtbMachine $Id; $reload = $true }
            '^[fF]$'  { Submit-HtbMachineFlag $Id; $reload = $true }
            '^[tT]$'  { Invoke-HtbMachineAction "/vm/terminate" (T 'act_stop') (T 'confirm_stop'); $reload = $true }
            '^[rR]$'  { Invoke-HtbMachineAction "/vm/reset" (T 'act_reset') (T 'confirm_reset') }
            '^[eE]$'  { Invoke-HtbMachineAction "/vm/extend" (T 'act_extend') "" }
            '^[vV]$'  { Show-HtbVotes $m.feedbackForChart }
            '^[cC]$'  { Show-HtbCopied "$(HtbValue $m 'playInfo.ip')" }
        }
    }
}

function Start-HtbMachine($Id) {
    $active = (HtbV4Get "/machine/active").Data.info
    if ($active -and $active.name) {
        Write-Host ""
        if (-not (Confirm-Htb "$($active.name) — $(T 'confirm_replace')")) { return }
        HtbV4Post "/vm/terminate" @{ machine_id = $active.id } | Out-Null
        Start-Sleep -Seconds 2
    }
    Write-Host ""
    $result = HtbV4Post "/vm/spawn" @{ machine_id = [int]$Id }
    $message = "$(HtbValue $result.Data 'message')"
    if ((Read-HtbActionAnswer $message) -ne 'si') {
        Write-HtbFail $message
        Write-HtbPlanNote $message
        Wait-HtbEnter; return
    }
    Write-HtbOk $message
    Write-Host -NoNewline "  $Grey$(T 'spawn_waiting')$Reset"
    $ip = "-"
    for ($i = 0; $i -lt 10; $i++) {
        Start-Sleep -Seconds 3
        $ip = "$(HtbValue (HtbV4Get '/machine/active').Data 'info.ip')"
        if ($ip -and $ip -ne "-") { break }
        Write-Host -NoNewline "."
    }
    Write-Host ""
    if (-not $ip -or $ip -eq "-") { Write-HtbNote (T 'spawn_no_ip') }
    else { Write-HtbField (T 'f_ip') "$Cyan$ip$Reset" }
    Wait-HtbEnter
}

function Invoke-HtbMachineAction([string]$Endpoint, [string]$Label, [string]$Question) {
    $active = (HtbV4Get "/machine/active").Data.info
    if (-not $active -or -not $active.name) {
        Write-Host ""; Write-HtbNote (T 'no_active_machine'); Wait-HtbEnter; return
    }
    Write-Host ""
    if ($Question -and -not (Confirm-Htb $Question)) { return }
    $result = Invoke-HtbApi -Method POST -Url "$BaseV4$Endpoint" -Body @{ machine_id = $active.id }
    $message = "$(HtbValue $result.Data 'message')"
    if ((Read-HtbActionAnswer $message) -eq 'si') { Write-HtbOk "${Label}: $message" }
    else { Write-HtbFail $message; Write-HtbPlanNote $message }
    Wait-HtbEnter
}

function Submit-HtbMachineFlag($Id) {
    Write-Host ""
    $flag = Read-HtbFlagText (T 'ask_flag')
    if (-not $flag) { return }
    Select-HtbRating
    if ($ReplyRating -lt 1) { return }
    $rating = $ReplyRating
    $result = HtbV5Post "/machine/own" @{ flag = $flag; id = [int]$Id; difficulty = [int]$rating }
    $message = "$(HtbValue $result.Data 'message')"
    switch (Read-HtbFlagAnswer $message) {
        'si' { Write-Host "  $Green$Bold$(T 'flag_accepted')$Reset — $message" }
        'no' { Write-HtbFail $message }
        default { Write-HtbNote $message }
    }
    Wait-HtbEnter
}

function Show-HtbTarget {
    $at = 0; $reload = $true
    $header = @(); $tasks = @(); $rows = @(); $id = ""; $ip = ""
    while ($true) {
        if ($reload) {
            Write-Host "  $Grey$(T 'loading')$Reset"
            $active = (HtbV4Get "/machine/active").Data.info
            if (-not $active -or -not $active.name) {
                Clear-Host; Write-HtbTitle (T 'machines_target') $Green; Write-Host ""
                Write-HtbNote (T 'no_active_machine'); Wait-HtbEnter; return
            }
            $id = "$($active.id)"
            $ip = "$(HtbValue $active 'ip')"
            $tasks = @(Get-HtbTaskRows $id)
            $rows = @(Format-HtbTaskRows $tasks)
            $header = @(
                "  $Bold$White$($active.name)$Reset"
                ""
                ("  $Grey" + "ID".PadRight(18) + "$Reset$id")
                ("  $Grey" + (T 'f_ip').PadRight(18) + "$Reset$Cyan$ip$Reset")
                ("  $Grey" + (T 'f_expires').PadRight(18) + "$Reset$Yellow$(HtbValue $active 'expires_at')$Reset")
            )
            if ($tasks.Count -gt 0) { $header += (Get-HtbProgressLine $tasks) }
            $header += ""
            $reload = $false
        }
        $keys = @()
        if ($tasks.Count -gt 0) { $keys += "↵|$(T 'task_answer')" }
        $keys += @("f|$(T 'act_flag')", "t|$(T 'act_stop')", "r|$(T 'act_reset')",
                   "e|$(T 'act_extend')", "c|$(T 'act_copy')")
        $items = if ($tasks.Count -gt 0) { $rows } else { @("  $Faint$(T 'no_tasks')$Reset") }
        Select-HtbItem -Title (T 'machines_target') -Color $Green -Hint (Write-HtbKeys @keys) -Items @($items) -Start $at -Header $header
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' {
                if ($tasks.Count -gt 0) {
                    Submit-HtbMachineTask $id $tasks[$ReplySel].id
                    $reload = $true
                }
            }
            '^[fF]$'  { Submit-HtbMachineFlag $id; $reload = $true }
            '^[tT]$'  { Invoke-HtbMachineAction "/vm/terminate" (T 'act_stop') (T 'confirm_stop'); $reload = $true }
            '^[rR]$'  { Invoke-HtbMachineAction "/vm/reset" (T 'act_reset') (T 'confirm_reset') }
            '^[eE]$'  { Invoke-HtbMachineAction "/vm/extend" (T 'act_extend') "" }
            '^[cC]$'  { Show-HtbCopied $ip }
        }
    }
}
