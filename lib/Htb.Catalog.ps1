function Show-HtbCatalogMenu {
    $items = @(
        (Write-HtbRow (T 'catalog_challenges') (T 'catalog_challenges_desc'))
        (Write-HtbRow (T 'catalog_prolabs')    (T 'catalog_prolabs_desc'))
        (Write-HtbRow (T 'catalog_fortress')   (T 'catalog_fortress_desc'))
        (Write-HtbRow (T 'catalog_sherlocks')  (T 'catalog_sherlocks_desc'))
        (Write-HtbRow (T 'catalog_tracks')     (T 'catalog_tracks_desc'))
        (Write-HtbRow (T 'catalog_seasons')    (T 'catalog_seasons_desc'))
    )
    $at = 0
    while ($true) {
        Select-HtbItem -Title (T 'catalog_title') -Color $Cyan -Items $items -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' {
                switch ($ReplySel) {
                    0 { Show-HtbChallenges }
                    1 { Show-HtbProlabs }
                    2 { Show-HtbFortress }
                    3 { Show-HtbSherlocks }
                    4 { Show-HtbTracks }
                    5 { Show-HtbSeasons }
                }
            }
        }
    }
}

$Script:ChallengeCategories = $null

function Import-HtbChallengeCategories {
    if ($Script:ChallengeCategories) { return }
    $Script:ChallengeCategories = @{}
    foreach ($c in (HtbV4Get "/challenge/categories/list").Data.info) {
        $Script:ChallengeCategories["$($c.id)"] = $c.name
    }
}

function Import-HtbAllChallenges {
    $all = @()
    $page = 1; $last = 1
    while ($true) {
        Clear-Host
        Write-Host ""
        Write-Host "  $Grey$(T 'loading') $page/$last$Reset"
        $response = HtbV4Get "/challenges?per_page=100&page=$page"
        $rows = @($response.Data.data)
        if ($rows.Count -eq 0) { break }
        $all += $rows
        if ($response.Data.meta.last_page) { $last = [int]$response.Data.meta.last_page }
        if ($page -ge $last -or $page -ge 40) { break }
        $page++
    }
    return $all
}

function Show-HtbChallenges {
    $all = @(Import-HtbAllChallenges)
    if ($all.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }
    Import-HtbChallengeCategories

    $category = ""; $difficulty = ""; $unsolved = $false; $at = 0
    $diffOptions = @(
        [pscustomobject]@{ Label = (T 'filter_any'); Value = "" }
        [pscustomobject]@{ Label = "Very Easy"; Value = "Very Easy" }
        [pscustomobject]@{ Label = "Easy";      Value = "Easy" }
        [pscustomobject]@{ Label = "Medium";    Value = "Medium" }
        [pscustomobject]@{ Label = "Hard";      Value = "Hard" }
        [pscustomobject]@{ Label = "Insane";    Value = "Insane" }
    )
    while ($true) {
        $view = @($all | Where-Object {
            ($category -eq "" -or "$($_.category_id)" -eq $category -or "$($_.challenge_category_id)" -eq $category) -and
            ($difficulty -eq "" -or "$($_.difficulty)" -eq $difficulty) -and
            (-not $unsolved -or -not ($_.is_owned -or $_.authUserSolve -or $_.isCompleted))
        })
        if ($view.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }

        $rows = @()
        foreach ($c in $view) {
            $solved = [bool]($c.is_owned -or $c.authUserSolve -or $c.isCompleted)
            $mark = if ($solved) { "✓" } else { " " }
            $markColor = if ($solved) { $Green } else { $Faint }
            $categoryName = "$($ChallengeCategories["$($c.category_id)"])"
            if (-not $categoryName) { $categoryName = "$($ChallengeCategories["$($c.challenge_category_id)"])" }
            if (-not $categoryName) { $categoryName = "$($c.category_name)" }
            $name = "$($c.name)"; if ($name.Length -gt 32) { $name = $name.Substring(0, 32) }
            if ($categoryName.Length -gt 16) { $categoryName = $categoryName.Substring(0, 16) }
            $rows += ("$markColor$mark$Reset $White" + $name.PadRight(32) + "$Reset $Purple" +
                      $categoryName.PadRight(16) + "$Reset $(Get-HtbDifficultyColor $c.difficulty)" +
                      "$($c.difficulty)".PadRight(11) + "$Reset $Faint$(T 'f_solves') $($c.solves)$Reset")
        }

        $shown = if ($category) { $ChallengeCategories[$category] } else { "—" }
        $diffShown = if ($difficulty) { $difficulty } else { T 'filter_any' }
        $toggle = if ($unsolved) { T 'yes' } else { T 'no' }
        $header = "$(T 'catalog_challenges') · $(T 'act_category'): $shown · $(T 'act_filter_challenge_difficulty'): $diffShown · $(T 'act_unsolved'): $toggle"
        $hint = Write-HtbKeys "c|$(T 'act_category')" "d|$(T 'act_filter_challenge_difficulty')" "u|$(T 'act_unsolved')" "x|$(T 'act_filter_clear')"

        Select-HtbItem -Title $header -Color $Cyan -Hint $hint -Items $rows -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' { Show-HtbChallengeDetail $view[$ReplySel].id }
            '^[cC]$'  { $category = Select-HtbChallengeCategory; $at = 0 }
            '^[dD]$'  {
                Select-HtbFilter (T 'act_filter_challenge_difficulty') $diffOptions
                if ($ReplyFilterSet) { $difficulty = $ReplyFilter; $at = 0 }
            }
            '^[uU]$'  { $unsolved = -not $unsolved; $at = 0 }
            '^[xX]$'  { $category = ""; $difficulty = ""; $unsolved = $false; $at = 0 }
        }
    }
}

function Select-HtbChallengeCategory {
    $keys = @("") + @($ChallengeCategories.Keys | Sort-Object { [int]$_ })
    $names = @((T 'challenge_all_categories')) + @($keys[1..($keys.Count - 1)] | ForEach-Object { $ChallengeCategories[$_] })
    Select-HtbItem -Title (T 'act_category') -Color $Cyan -Items $names
    if ($ReplyKey -eq "enter") { return $keys[$ReplySel] }
    return ""
}

function Show-HtbChallengeDetail($Id) {
    while ($true) {
        Write-Host "  $Grey$(T 'loading')$Reset"
        $raw = (HtbV4Get "/challenge/info/$Id").Data
        $c = if ($raw.challenge) { $raw.challenge } elseif ($raw.data) { $raw.data } else { $raw }
        $name = "$(HtbValue $c 'name')"
        $difficulty = "$(HtbValue $c 'difficulty')"
        Clear-Host; Write-HtbTitle "$(T 'catalog_challenges') · $name" $Cyan; Write-Host ""
        Write-Host "  $Bold$White$name$Reset"
        Write-Host ""
        Write-HtbField (T 'f_category') "$Purple$(HtbValue $c 'category_name')$Reset"
        Write-HtbField (T 'f_difficulty') "$(Get-HtbDifficultyColor $difficulty)$difficulty$Reset"
        Write-HtbField (T 'f_points') "$Yellow$(HtbValue $c 'points')$Reset"
        Write-HtbField (T 'f_solves') "$(HtbValue $c 'solves')"
        Write-HtbField (T 'f_author') "$(HtbValue $c 'creator_name')"
        Write-HtbField (T 'f_solved') $(if ($c.authUserSolve) { T 'yes' } else { T 'no' })
        if ($c.description) {
            Write-Host ""
            foreach ($line in (Format-HtbParagraph $c.description)) { Write-Host $line }
        }
        Write-Host ""
        if ($c.download) {
            Write-Host "    $Green·$Reset $(HtbValue $c 'file_name') — $(T 'zip_password')"
        } else {
            Write-Host "    $Faint· $(T 'no_files')$Reset"
        }
        $containerUp = [bool]($c.play_info -and $c.play_info.ip)
        $containerPorts = @($c.play_info.ports)
        $containerTarget = ""
        if ($containerUp) {
            $containerTarget = if ($containerPorts.Count -gt 0) { "$($c.play_info.ip):$($containerPorts[0])" } else { "$($c.play_info.ip)" }
        }
        if ($c.docker) {
            if ($containerUp) {
                Write-Host "    $Green·$Reset $(T 'f_container'): $Cyan$containerTarget$Reset $Faint$(T 'vm_ports') $($containerPorts -join ', ') · $(T 'vm_needs_vpn_labs')$Reset"
            } else {
                Write-Host "    $Faint· $(T 'f_container_off'): $(T 'vm_offline')$Reset"
            }
        }
        Write-Host ""; Write-HtbRule
        $keys = @()
        if ($c.download) { $keys += "d|$(T 'act_download')" }
        if ($c.docker) {
            if ($containerUp) { $keys += "t|$(T 'act_stop_lab')" } else { $keys += "s|$(T 'act_start_lab')" }
        }
        if ($containerUp) { $keys += "c|$(T 'act_copy')" }
        $keys += "f|$(T 'act_flag')"
        $keys += "←|$(T 'nav_back')"
        Write-Host (Write-HtbKeys @keys)
        $choice = Read-HtbKey
        switch -Regex ($choice) {
            '^CHAR:[sS]$' {
                if ($c.docker -and -not $containerUp) {
                    Write-Host ""
                    $result = HtbV4Post "/container/start" @{ containerable_id = [int]$Id }
                    $message = "$(HtbValue $result.Data 'message')"
                    if ($message -match '(?i)created|started') { Write-HtbOk $message }
                    else { Write-HtbFail $message; Write-HtbPlanNote $message }
                    Wait-HtbEnter
                }
            }
            '^CHAR:[tT]$' {
                if ($c.docker -and $containerUp) {
                    Write-Host ""
                    $result = HtbV4Post "/container/stop" @{ containerable_id = [int]$Id }
                    Write-HtbOk "$(HtbValue $result.Data 'message')"
                    Wait-HtbEnter
                }
            }
            '^CHAR:[dD]$' {
                if ($c.download) {
                    Save-HtbFile "$BaseV4/challenge/download/$Id" "challenge_${Id}_$(Get-HtbSafeName $name).zip" $true | Out-Null
                } else { Write-Host ""; Write-HtbNote (T 'no_files'); Wait-HtbEnter }
            }
            '^CHAR:[cC]$' {
                if ($containerUp) {
                    if ($containerPorts.Count -gt 0) { Show-HtbCopied "$containerTarget`nhttp://$containerTarget" }
                    else { Show-HtbCopied $containerTarget }
                }
            }
            '^CHAR:[fF]$' { Submit-HtbChallengeFlag $Id }
            '^(BACK|QUIT|CHAR:[qQ])$' { return }
        }
    }
}

function Submit-HtbChallengeFlag($Id) {
    Write-Host ""
    $flag = Read-HtbFlagText (T 'ask_flag')
    if (-not $flag) { return }
    Select-HtbRating
    if ($ReplyRating -lt 1) { return }
    $rating = $ReplyRating
    $result = HtbV4Post "/challenge/own" @{ flag = $flag; challenge_id = [int]$Id; difficulty = ([int]$rating * 10) }
    $message = "$(HtbValue $result.Data 'message')"
    if ((Read-HtbFlagAnswer $message) -eq 'si') {
        Write-Host "  $Green$Bold$(T 'flag_accepted')$Reset — $message"
    } else { Write-HtbFail $message }
    Wait-HtbEnter
}

function Show-HtbProlabs {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $response = HtbV4Get "/prolabs"
    $labs = @(if ($response.Data.data.labs) { $response.Data.data.labs } else { $response.Data.data })
    if ($labs.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }
    $rows = foreach ($l in $labs) {
        $name = "$($l.name)"; if ($name.Length -gt 26) { $name = $name.Substring(0, 26) }
        ("$White" + $name.PadRight(26) + "$Reset $(Get-HtbDifficultyColor $l.skill_level)" +
         "$($l.skill_level)".PadRight(13) + "$Reset " + "$($l.pro_machines_count)".PadLeft(3) +
         " $(T 'f_machines') $Green$($l.pro_flags_owns_count)/$($l.pro_flags_count) $(T 'f_flags')$Reset  $($l.ownership)%")
    }
    $at = 0
    while ($true) {
        Select-HtbItem -Title (T 'catalog_prolabs') -Color $Orange -Items @($rows) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' { Show-HtbProlabDetail $labs[$ReplySel].id $labs[$ReplySel].name }
        }
    }
}

function Show-HtbProlabDetail($Id, [string]$Name) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $info = (HtbV4Get "/prolab/$Id/info").Data.data
    $progress = (HtbV4Get "/prolab/$Id/progress").Data.data
    $entry = if ($info.entry_points) { $info.entry_points -join ", " } else { "-" }
    $lines = @(
        ""
        "  $Bold$White$Name$Reset"
        ""
        ("  $Grey" + (T 'f_version').PadRight(18) + "$Reset$(HtbValue $info 'version')")
        ("  $Grey" + (T 'f_progress').PadRight(18) + "$Reset$(HtbValue $progress 'ownership')%")
        ("  $Grey" + (T 'f_entry').PadRight(18) + "$Reset$entry")
        ""
        "  $Green── $(T 'f_machines')$Reset"
    )
    foreach ($m in (HtbV4Get "/prolab/$Id/machines").Data.data) { $lines += "    · $($m.name) ($($m.os))" }
    $lines += ""
    $lines += "  $Green── $(T 'f_flags')$Reset"
    foreach ($f in (HtbV4Get "/prolab/$Id/flags").Data.data) {
        $owned = if ($f.owned) { "✓" } else { "" }
        $lines += "    · $($f.title) — $($f.points) pts $owned"
    }
    Show-HtbLines $Name $Orange $lines
}

function Show-HtbFortress {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $raw = (HtbV4Get "/fortresses").Data.data
    $forts = if ($raw -is [array]) { @($raw) } else { @($raw.PSObject.Properties | ForEach-Object { $_.Value }) }
    if ($forts.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }
    $rows = foreach ($f in $forts) {
        $detail = (HtbV4Get "/fortress/$($f.id)").Data.data
        $owned = @($detail.flags | Where-Object { $_.owned }).Count
        $name = "$($f.name)"; if ($name.Length -gt 26) { $name = $name.Substring(0, 26) }
        ("$White" + $name.PadRight(26) + "$Reset $Green$owned/$($f.number_of_flags) $(T 'f_flags')$Reset  $Yellow$($detail.points) pts$Reset")
    }
    $at = 0
    while ($true) {
        Select-HtbItem -Title (T 'catalog_fortress') -Color $Red -Items @($rows) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' { Show-HtbFortressDetail $forts[$ReplySel].id }
        }
    }
}

function Show-HtbFortressDetail($Id) {
    while ($true) {
        Write-Host "  $Grey$(T 'loading')$Reset"
        $d = (HtbV4Get "/fortress/$Id").Data.data
        $owned = @($d.flags | Where-Object { $_.owned }).Count
        Clear-Host; Write-HtbTitle "$(T 'catalog_fortress') · $($d.name)" $Red; Write-Host ""
        Write-HtbField (T 'f_ip') "$Cyan$(HtbValue $d 'ip')$Reset"
        Write-HtbField (T 'f_points') "$Yellow$(HtbValue $d 'points')$Reset"
        Write-HtbField (T 'f_flags') "$Green$owned/$(@($d.flags).Count)$Reset"
        Write-HtbField (T 'f_progress') "$(HtbValue $d 'progress_percent')%"
        if ($d.description) {
            Write-Host ""
            foreach ($line in (Format-HtbParagraph $d.description)) { Write-Host $line }
        }
        Write-Host ""
        foreach ($f in $d.flags) {
            $ok = if ($f.owned) { "✓" } else { "" }
            Write-Host "    · $($f.title) — $($f.points) pts $ok"
        }
        Write-Host ""; Write-HtbRule
        Write-Host (Write-HtbKeys "f|$(T 'act_flag')" "c|$(T 'act_copy')" "←|$(T 'nav_back')")
        $choice = Read-HtbKey
        switch -Regex ($choice) {
            '^CHAR:[fF]$' {
                Write-Host ""
                $flag = Read-HtbFlagText (T 'ask_flag')
                if ($flag) {
                    $result = HtbV4Post "/fortress/$Id/flag" @{ flag = $flag }
                    $message = "$(HtbValue $result.Data 'message')"
                    if ((Read-HtbFlagAnswer $message) -eq 'si') {
                        Write-Host "  $Green$Bold$(T 'flag_accepted')$Reset — $message"
                    } else { Write-HtbFail $message }
                    Wait-HtbEnter
                }
            }
            '^CHAR:[cC]$' { Show-HtbCopied "$(HtbValue $d 'ip')" }
            '^(BACK|QUIT|CHAR:[qQ])$' { return }
        }
    }
}

function Show-HtbSherlocks {
    $page = 1; $reload = $true; $at = 0; $last = 1; $sherlocks = @()
    while ($true) {
        if ($reload) {
            Write-Host "  $Grey$(T 'loading')$Reset"
            $response = HtbV4Get "/sherlocks?page=$page"
            $sherlocks = @(if ($response.Data.data) { $response.Data.data } else { $response.Data.sherlocks })
            $last = if ($response.Data.meta.last_page) { [int]$response.Data.meta.last_page } else { 1 }
            $reload = $false
        }
        if ($sherlocks.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }
        $rows = foreach ($s in $sherlocks) {
            $mark = if ($s.is_owned) { "✓" } else { " " }
            $markColor = if ($s.is_owned) { $Green } else { $Faint }
            $name = "$($s.name)"; if ($name.Length -gt 32) { $name = $name.Substring(0, 32) }
            $category = "$($s.category_name)"; if ($category.Length -gt 18) { $category = $category.Substring(0, 18) }
            ("$markColor$mark$Reset $White" + $name.PadRight(32) + "$Reset $Purple" + $category.PadRight(18) +
             "$Reset $(Get-HtbDifficultyColor $s.difficulty)" + "$($s.difficulty)".PadRight(11) + "$Reset $Faint$($s.solves)$Reset")
        }
        Select-HtbItem -Title "$(T 'catalog_sherlocks') · $(T 'act_page') $page/$last" -Color $Cyan `
                       -Hint (Write-HtbKeys "n|$(T 'page_next')" "p|$(T 'page_previous')") -Items @($rows) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' { Show-HtbSherlockDetail $sherlocks[$ReplySel].id }
            '^[nN]$'  { if ($page -lt $last) { $page++; $reload = $true; $at = 0 } }
            '^[pP]$'  { if ($page -gt 1) { $page--; $reload = $true; $at = 0 } }
        }
    }
}

function Show-HtbSherlockDetail($Id) {
    $at = 0
    $reload = $true
    $name = ""; $password = ""; $downloadable = $false
    $header = @(); $tasks = @(); $rows = @()

    while ($true) {
        if ($reload) {
            Write-Host "  $Grey$(T 'loading')$Reset"
            $metaResponse = HtbV4Get "/sherlocks/$Id"
            if (-not (Test-HtbApi $metaResponse)) { return }
            $meta = $metaResponse.Data.data
            $info = (HtbV4Get "/sherlocks/$Id/info").Data.data
            $progress = (HtbV4Get "/sherlocks/$Id/progress").Data.data
            $tasks = @((HtbV4Get "/sherlocks/$Id/tasks").Data.data)
            $play = (HtbV4Get "/sherlocks/$Id/play").Data.data

            $name = "$(HtbValue $meta 'name')"
            $password = "$(HtbValue $meta 'file_password')"
            if (-not $password -or $password -eq "-") { $password = $SherlockZipPassword }
            $downloadable = @($meta.play_methods) -contains "download"
            $hasVm = @($meta.play_methods) -contains "vm"
            $vmStatus = "$(HtbValue $play 'play_info.status')"
            $vmIp = "$(HtbValue $play 'play_info.ip')"
            $vmUp = ($vmStatus -ne "-")
            $difficulty = "$(HtbValue $meta 'difficulty')"
            $released = Get-HtbDate $meta.release_at

            $answered = if ($null -ne $progress.tasks_answered) { $progress.tasks_answered }
                        else { @($tasks | Where-Object { $_.completed }).Count }
            $total = if ($null -ne $progress.total_tasks) { $progress.total_tasks } else { $tasks.Count }

            $material = if ($downloadable) {
                if ($password -and $password -ne "-") { "$(T 'zip_password_is') $password" } else { T 'zip_password' }
            } elseif (-not $hasVm) { T 'sherlock_no_material' } else { "" }

            $header = @(
                ("  $Grey" + (T 'f_category').PadRight(18) + "$Reset$Purple$(HtbValue $meta 'category_name')$Reset")
                ("  $Grey" + (T 'f_difficulty').PadRight(18) + "$Reset$(Get-HtbDifficultyColor $difficulty)$difficulty$Reset")
                ("  $Grey" + (T 'f_rating').PadRight(18) + "$Reset" + ("{0:N2}" -f [double](HtbValue $meta 'rating' 0)))
                ("  $Grey" + (T 'f_state').PadRight(18) + "$Reset$(HtbValue $meta 'state')")
                ("  $Grey" + (T 'f_released').PadRight(18) + "$Reset$released")
                ("  $Grey" + (T 'f_progress').PadRight(18) + "$Reset$answered/$total $(T 'sherlock_answered')")
            )
            if ($material) { $header += ("  $Grey" + (T 'sherlock_material').PadRight(18) + "$Reset$material") }
            if ($hasVm) {
                $vmLine = if (-not $vmUp) { "$Faint$(T 'vm_offline')$Reset" }
                          elseif ($vmIp -eq "-") { "$Yellow$(T 'vm_deploying')$Reset" }
                          else { "$Cyan$vmIp$Reset   $Faint$(T 'vm_needs_vpn')$Reset" }
                $header += ("  $Grey" + (T 'vm_label').PadRight(18) + "$Reset$vmLine")
                $vmUser = "$(HtbValue $play 'username')"
                $vmPass = "$(HtbValue $play 'password')"
                if ($vmUser -ne "-" -or $vmPass -ne "-") {
                    $header += ("  $Grey" + (T 'vm_credentials').PadRight(18) + "$Reset$(T 'vm_user') $vmUser   $(T 'vm_password') $vmPass")
                }
            }
            if ($info.description) {
                $header += ""
                $header += (Format-HtbParagraph $info.description 110)[0..([Math]::Min(2, (Format-HtbParagraph $info.description 110).Count - 1))]
            }

            $rows = foreach ($task in $tasks) {
                $mark = if ($task.completed) { "✓" } else { " " }
                $markColor = if ($task.completed) { $Green } else { $Faint }
                $question = "$($task.description)"
                if ($question.Length -gt 72) { $question = $question.Substring(0, 72) }
                $expected = "$($task.masked_flag)"
                if ($expected.Length -gt 22) { $expected = $expected.Substring(0, 22) }
                ("$markColor$mark$Reset $Grey" + "$($task.title)".PadRight(9) + "$Reset $White" +
                 $question.PadRight(72) + "$Reset $Faint$expected$Reset")
            }
            $reload = $false
        }

        if ($tasks.Count -eq 0) {
            Show-HtbLines $name $Cyan $header
            return
        }

        $keys = @("↵|$(T 'action_answer')")
        if ($downloadable) { $keys += "d|$(T 'act_download')" }
        if ($hasVm) {
            if ($vmUp) { $keys += "t|$(T 'act_stop_lab')"; $keys += "r|$(T 'act_reset')"
                         if ($vmIp -ne "-") { $keys += "c|$(T 'act_copy')" } }
            else { $keys += "s|$(T 'act_start_vm')" }
        }
        $keys += "←|$(T 'nav_back')"
        $hint = Write-HtbKeys @keys
        Select-HtbItem -Title "$(T 'catalog_sherlocks') · $name" -Color $Cyan -Hint $hint -Items @($rows) -Start $at -Header $header
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' { Submit-HtbSherlockTask $Id $tasks[$ReplySel].id; $reload = $true }
            '^[sS]$'  {
                if ($hasVm -and -not $vmUp) {
                    Write-Host ""
                    $result = HtbV4Post "/challenge/start/vm/$Id"
                    $message = "$(HtbValue $result.Data 'message')"
                    if ($result.Data.success) {
                        Write-HtbOk $message
                        Write-HtbField (T 'vm_user') "$(HtbValue $result.Data 'username')"
                        Write-HtbField (T 'vm_password') "$(HtbValue $result.Data 'password')"
                    } else { Write-HtbFail $message; Write-HtbPlanNote $message }
                    Wait-HtbEnter; $reload = $true
                }
            }
            '^[tT]$'  {
                if ($hasVm -and $vmUp) {
                    Write-Host ""
                    $result = HtbV4Post "/vm/terminate" @{ machine_id = [int]$Id }
                    Write-HtbOk "$(HtbValue $result.Data 'message')"
                    Wait-HtbEnter; $reload = $true
                }
            }
            '^[rR]$'  {
                if ($hasVm -and $vmUp) {
                    Write-Host ""
                    $result = HtbV4Post "/challenge/reset/vm/$Id"
                    Write-HtbOk "$(HtbValue $result.Data 'message')"
                    Wait-HtbEnter; $reload = $true
                }
            }
            '^[cC]$'  {
                if ($hasVm -and $vmUp -and $vmIp -ne "-") {
                    Show-HtbCopied "$vmIp`nxfreerdp /v:$vmIp /u:LetsDefend /p:'' /cert:ignore /dynamic-resolution"
                }
            }
            '^[dD]$'  {
                if (-not $downloadable) { Write-Host ""; Write-HtbNote (T 'sherlock_no_material'); Wait-HtbEnter }
                else {
                    $link = "$(HtbValue (HtbV4Get "/sherlocks/$Id/download_link").Data 'url')"
                    if (-not $link -or $link -eq "-") { Write-Host ""; Write-HtbNote (T 'no_link'); Wait-HtbEnter }
                    else { Save-HtbFile $link "sherlock_${Id}_$(Get-HtbSafeName $name).zip" $true $password | Out-Null }
                }
            }
        }
    }
}

function Submit-HtbSherlockTask($SherlockId, $TaskId) {
    Write-Host ""
    $answer = Read-HtbFlagText (T 'ask_answer')
    if (-not $answer) { return }
    $result = HtbV4Post "/sherlocks/$SherlockId/tasks/$TaskId/flag" @{ flag = $answer }
    $message = "$(HtbValue $result.Data 'message')"
    if ((Read-HtbFlagAnswer $message) -eq 'si') {
        Write-Host "  $Green$Bold$(T 'task_accepted')$Reset — $message"
    } else {
        Write-HtbFail $message
    }
    Wait-HtbEnter
}

function Show-HtbTracks {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $response = HtbV4Get "/tracks"
    $list = @(if ($response.Data -is [array]) { $response.Data } else { $response.Data.data })
    if ($list.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }
    $rows = foreach ($track in $list) {
        $name = "$($track.name)"; if ($name.Length -gt 38) { $name = $name.Substring(0, 38) }
        ("$White" + $name.PadRight(38) + "$Reset $(Get-HtbDifficultyColor $track.difficulty)" +
         "$($track.difficulty)".PadRight(11) + "$Reset $Faint$($track.likes) $(T 'f_likes')$Reset")
    }
    $at = 0
    while ($true) {
        Select-HtbItem -Title (T 'catalog_tracks') -Color $Cyan -Items @($rows) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' { Show-HtbTrackDetail $list[$ReplySel].id $list[$ReplySel].name }
        }
    }
}

function Show-HtbTrackDetail($Id, [string]$Name) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $raw = (HtbV4Get "/tracks/$Id").Data
    $track = if ($raw.data) { $raw.data } else { $raw }
    $difficulty = "$(HtbValue $track 'difficulty')"
    $lines = @(
        ""
        "  $Bold$White$Name$Reset"
        ""
        ("  $Grey" + (T 'f_difficulty').PadRight(18) + "$Reset$(Get-HtbDifficultyColor $difficulty)$difficulty$Reset")
        ("  $Grey" + (T 'f_likes').PadRight(18) + "$Reset$(HtbValue $track 'likes')")
        ("  $Grey" + (T 'f_enrolled').PadRight(18) + "$Reset" + $(if ($track.enrolled) { T 'yes' } else { T 'no' }))
    )
    if ($track.description) {
        $lines += ""
        $lines += (Format-HtbParagraph $track.description)
    }
    $lines += ""
    $lines += "  $Cyan── $(T 'f_content')$Reset"
    foreach ($item in $track.items) { $lines += "    · [$($item.type)] $($item.name) ($($item.difficulty))" }
    Show-HtbLines $Name $Cyan $lines
}

function Show-HtbSeasons {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $list = (HtbV4Get "/season/list").Data.data
    $active = $list | Where-Object { $_.active -eq $true } | Select-Object -First 1
    $lines = @("")
    if (-not $active) {
        $lines += "  $Yellow$(T 'season_none')$Reset"
    } else {
        $rank = (HtbV4Get "/season/user/rank/$($active.id)").Data.data
        $league = "$(HtbValue $rank 'league')"
        if ($league -eq "-") { $league = "→ $(HtbValue $rank 'next_rank.title')" }
        $lines += "  $Bold$White$($active.name)$Reset  $Faint($($active.state))$Reset"
        $lines += ""
        $lines += ("  $Grey" + (T 'season_tier').PadRight(18) + "$Reset$Purple$league$Reset")
        if ("$(HtbValue $rank 'rank')" -ne "-") {
            $lines += ("  $Grey" + (T 'season_rank').PadRight(18) + "$Reset$(HtbValue $rank 'rank')/$(HtbValue $rank 'total_ranks')")
        }
        $lines += ("  $Grey" + (T 'season_points').PadRight(18) + "$Reset$(HtbValue $rank 'total_season_points')")
        $lines += ("  $Grey" + (T 'season_flags').PadRight(18) + "$Reset$(HtbValue $rank 'total_season_flags.obtained')/$(HtbValue $rank 'total_season_flags.total')")
        $lines += ("  $Grey" + (T 'season_bloods').PadRight(18) + "$Reset$(HtbValue $rank 'total_season_bloods')")
        $lines += ""
        $lines += "  $Purple── $(T 'season_machines')$Reset"
        $seasonMachines = (HtbV4Get "/season/machines").Data
        $seasonMachines = @(if ($seasonMachines -is [array]) { $seasonMachines } else { $seasonMachines.data })
        foreach ($m in $seasonMachines) {
            $mark = if ($m.is_owned_root) { "✓✓" } elseif ($m.is_owned_user) { "✓ " } else { "  " }
            $lines += "    $mark $($m.name) ($($m.os) · $($m.difficulty_text))"
        }
        $lines += ""
        $lines += "  $Faint$(T 'season_spawn_note')$Reset"
    }
    Show-HtbLines (T 'catalog_seasons') $Purple $lines
}
