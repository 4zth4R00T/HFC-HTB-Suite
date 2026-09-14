function Format-HtbPoints($Raw) {
    $value = 0.0
    if ([double]::TryParse("$Raw", [ref]$value)) {
        if ([Math]::Abs($value - [Math]::Round($value)) -lt 0.0001) { return "{0:N0}" -f $value }
        return "{0:N2}" -f $value
    }
    return "$Raw"
}

function Show-HtbProfile {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $info = HtbV4Get "/user/info"
    if (-not $info.Ok -or -not $info.Data.info.id) {
        Clear-Host; Write-HtbTitle (T 'profile_title') $Green; Write-Host ""
        Write-HtbFail (T 'token_invalid'); Wait-HtbEnter; return
    }
    $user = $info.Data.info
    $p = (HtbV4Get "/user/profile/basic/$($user.id)").Data.profile
    $joined = Get-HtbDate $p.joined_date

    $lines = @(
        ""
        "  $Bold$White$(HtbValue $p 'name')$Reset"
        ""
        "  $Green── $(T 'p_account')$Reset"
        ("  $Grey" + (T 'p_rank').PadRight(18) + "$Reset$Yellow$(HtbValue $p 'rank')$Reset  $Faint(#$(HtbValue $p 'ranking'))$Reset")
        ("  $Grey" + (T 'p_next_rank').PadRight(18) + "$Reset$(HtbValue $p 'next_rank') — $(HtbValue $p 'current_rank_progress')% ($(Format-HtbPoints (HtbValue $p 'next_rank_points')) pts)")
        ("  $Grey" + (T 'p_points').PadRight(18) + "$Reset$Green$(HtbValue $p 'points')$Reset")
        ("  $Grey" + (T 'p_owns').PadRight(18) + "$Reset" + "user $(HtbValue $p 'user_owns') · system $(HtbValue $p 'system_owns')")
        ("  $Grey" + (T 'p_bloods').PadRight(18) + "$Reset" + "user $(HtbValue $p 'user_bloods') · system $(HtbValue $p 'system_bloods') · challenge $(HtbValue $p 'challenge_bloods')")
        ("  $Grey" + (T 'p_respects').PadRight(18) + "$Reset$(HtbValue $p 'respects')")
        ("  $Grey" + (T 'p_country').PadRight(18) + "$Reset$(HtbValue $p 'country_name')")
        ("  $Grey" + (T 'p_server').PadRight(18) + "$Reset$Cyan$(HtbValue $p 'server')$Reset")
        ("  $Grey" + (T 'p_since').PadRight(18) + "$Reset$joined")
        ("  $Grey" + (T 'p_team').PadRight(18) + "$Reset$(HtbValue $p 'team')")
        ""
        "  $Green── $(T 'p_subscription')$Reset"
        ("  $Grey" + (T 'p_plan').PadRight(18) + "$Reset$Yellow$(HtbValue $user 'subscriptionType')$Reset")
        ("  $Grey" + (T 'p_vip').PadRight(18) + "$Reset" + $(if ($user.canAccessVIP) { T 'yes' } else { T 'no' }))
    )

    $subs = (HtbV4Get "/user/subscriptions").Data.subscriptions
    if ($subs -and @($subs).Count -gt 0) {
        foreach ($s in $subs) { $lines += "    · $($s.name) ($($s.status))" }
    } else {
        $lines += "    $Faint· $(T 'sub_none')$Reset"
        $lines += "    $Faint· $StoreUrl$Reset"
    }

    $lines += ""
    $lines += "  $Green── $(T 'p_season')$Reset"
    $season = (HtbV4Get "/season/list").Data.data | Where-Object { $_.active -eq $true } | Select-Object -First 1
    if (-not $season) {
        $lines += "    $Yellow· $(T 'season_none')$Reset"
    } else {
        $rank = (HtbV4Get "/season/user/rank/$($season.id)").Data.data
        $league = "$(HtbValue $rank 'league')"
        if ($league -eq "-") { $league = "→ $(HtbValue $rank 'next_rank.title')" }
        $lines += ("  $Grey" + (T 'p_season').PadRight(18) + "$Reset$($season.name)  $Faint($($season.state))$Reset")
        $lines += ("  $Grey" + (T 'season_tier').PadRight(18) + "$Reset$Purple$league$Reset")
        $lines += ("  $Grey" + (T 'season_points').PadRight(18) + "$Reset$(HtbValue $rank 'total_season_points')")
        $lines += ("  $Grey" + (T 'season_flags').PadRight(18) + "$Reset$(HtbValue $rank 'total_season_flags.obtained')/$(HtbValue $rank 'total_season_flags.total')")
        $lines += ("  $Grey" + (T 'season_bloods').PadRight(18) + "$Reset$(HtbValue $rank 'total_season_bloods')")
    }

    Show-HtbLines (T 'profile_title') $Green $lines
}

function Show-HtbDiagnostics {
    Clear-Host; Write-HtbTitle (T 'diag_title') $Cyan; Write-Host ""
    Write-Host "  $Grey$(T 'diag_running')$Reset"
    Write-Host ""
    Write-Host ("  $White" + (T 'diag_status').PadRight(8) + (T 'diag_endpoint').PadRight(30) + "HTTP".PadRight(10) + (T 'diag_latency') + $Reset)
    Write-Host ""

    $uid = "$(HtbValue (HtbV4Get '/user/info').Data 'info.id')"
    $season = (HtbV4Get "/season/list").Data.data | Where-Object { $_.active -eq $true } | Select-Object -First 1
    $seasonId = if ($season) { $season.id } else { 1 }

    $probes = @(
        [pscustomobject]@{ Label = "v4 /user/info";            Url = "$BaseV4/user/info" }
        [pscustomobject]@{ Label = "v4 /user/profile/basic";   Url = "$BaseV4/user/profile/basic/$uid" }
        [pscustomobject]@{ Label = "v4 /user/subscriptions";   Url = "$BaseV4/user/subscriptions" }
        [pscustomobject]@{ Label = "v5 /machines";             Url = "$BaseV5/machines?per_page=1" }
        [pscustomobject]@{ Label = "v4 /machine/active";       Url = "$BaseV4/machine/active" }
        [pscustomobject]@{ Label = "v4 /connections/servers";  Url = "$BaseV4/connections/servers?product=labs" }
        [pscustomobject]@{ Label = "v4 /challenge/list";       Url = "$BaseV4/challenge/list" }
        [pscustomobject]@{ Label = "v4 /challenge/categories"; Url = "$BaseV4/challenge/categories/list" }
        [pscustomobject]@{ Label = "v4 /prolabs";              Url = "$BaseV4/prolabs" }
        [pscustomobject]@{ Label = "v4 /fortresses";           Url = "$BaseV4/fortresses" }
        [pscustomobject]@{ Label = "v4 /sherlocks";            Url = "$BaseV4/sherlocks?page=1" }
        [pscustomobject]@{ Label = "v4 /tracks";               Url = "$BaseV4/tracks" }
        [pscustomobject]@{ Label = "v4 /season/list";          Url = "$BaseV4/season/list" }
        [pscustomobject]@{ Label = "v4 /season/user/rank";     Url = "$BaseV4/season/user/rank/$seasonId" }
    )

    $healthy = 0
    foreach ($probe in $probes) {
        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        try {
            Invoke-WebRequest -Uri $probe.Url -Headers @{ Authorization = "Bearer $ApiToken" } -TimeoutSec 12 -ErrorAction Stop | Out-Null
            $code = 200
        } catch {
            $code = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 0 }
        }
        $watch.Stop()
        if ($code -ge 200 -and $code -lt 300) { $mark = "OK"; $color = $Green; $healthy++ }
        elseif ($code -eq 401 -or $code -eq 403) { $mark = "AUTH"; $color = $Yellow }
        else { $mark = "FAIL"; $color = $Red }
        Write-Host ("  $color" + $mark.PadRight(8) + "$Reset" + $probe.Label.PadRight(30) + "$code".PadRight(10) + "$Faint$($watch.ElapsedMilliseconds) ms$Reset")
    }

    Write-Host ""
    Write-Host "  $White$healthy/$($probes.Count) $(T 'diag_summary')$Reset"
    Write-Host ""
    Write-HtbField (T 'diag_token') $(if ($ApiToken) { "$Green$(T 'diag_configured')$Reset" } else { "$Red$(T 'diag_missing')$Reset" })
    Write-HtbField (T 'diag_theme') $HtbTheme
    Write-HtbField (T 'diag_language') $HtbLang
    Write-HtbField (T 'diag_config') $ConfigDir
    Write-HtbField (T 'diag_downloads') $DlDir
    Wait-HtbEnter
}

function Show-HtbCredits {
    Clear-Host; Write-HtbTitle (T 'credits_title') $Green; Write-Host ""
    Show-HtbMark
    Write-Host ""
    Write-HtbField (T 'credits_author') "$White$HtbAuthor$Reset"
    Write-HtbField "" "$Cyan$HtbAuthorInstagram$Reset"
    Write-HtbField "" "$Faint$HtbAuthorGithub$Reset"
    Write-Host ""
    Write-HtbField "" "$Green$HtbCrew$Reset"
    Write-HtbField "" "$Cyan$HtbCrewInstagram$Reset"
    Write-HtbField "" "$Faint$HtbCrewBlog$Reset"
    Write-Host ""
    Write-Host "  $White$(T 'credits_belongs')$Reset"
    Write-Host ""
    Write-Host "  $Grey$(T 'credits_thanks')$Reset $Orange$HtbThanks$Reset"
    Write-Host "  $Faint$(T 'credits_thanks_note')$Reset"
    Write-Host ""
    Write-Host "  $Faint$(T 'credits_license')$Reset"
    Wait-HtbEnter
}

function Show-HtbSettings {
    $at = 0
    while ($true) {
        $items = @(
            (Write-HtbRow (T 'set_theme')    $HtbTheme)
            (Write-HtbRow (T 'set_language') $HtbLang)
            (Write-HtbRow (T 'set_token')      (T 'set_token_desc'))
            (Write-HtbRow (T 'set_passphrase') (T 'set_passphrase_desc'))
            (Write-HtbRow (T 'set_forget')     (T 'set_forget_desc'))
            (Write-HtbRow (T 'set_paths')      $ConfigDir)
        )
        Select-HtbItem -Title (T 'settings_title') -Color $White -Items $items -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' {
                switch ($ReplySel) {
                    0 { Set-HtbTheme $(if ($HtbTheme -eq "dark") { "light" } else { "dark" }) }
                    1 { Show-HtbLanguage }
                    2 {
                        Clear-Host; Write-HtbTitle (T 'set_token') $White; Write-Host ""
                        $token = Read-HtbPassphrase (T 'ask_token')
                        if ($token) {
                            Write-Host ""
                            $pass = Read-HtbNewPassphrase
                            if ($pass -and (Write-HtbVault $token $pass)) {
                                $Script:ApiToken = $token
                                Clear-HtbPlaintextToken
                                Write-Host ""
                                if ((HtbV4Get "/user/info").Data.info.id) { Write-HtbOk (T 'token_updated') }
                                else { Write-HtbFail (T 'token_invalid') }
                            } else {
                                Write-HtbFail (T 'vault_cancelled')
                            }
                            Wait-HtbEnter
                        }
                    }
                    3 {
                        Clear-Host; Write-HtbTitle (T 'set_passphrase') $White; Write-Host ""
                        $pass = Read-HtbNewPassphrase
                        if ($pass -and (Write-HtbVault $ApiToken $pass)) {
                            Write-HtbOk (T 'passphrase_updated')
                        } else {
                            Write-HtbFail (T 'vault_cancelled')
                        }
                        Wait-HtbEnter
                    }
                    4 {
                        Clear-Host; Write-HtbTitle (T 'set_forget') $Yellow; Write-Host ""
                        if (Confirm-Htb (T 'confirm_forget')) {
                            Clear-HtbVault
                            Write-HtbOk (T 'token_removed')
                            Wait-HtbEnter
                        }
                    }
                    5 {
                        $paths = @(
                            ""
                            ("  $Grey" + (T 'diag_config').PadRight(18) + "$Reset$ConfigDir")
                            ("  $Grey" + (T 'diag_downloads').PadRight(18) + "$Reset$DlDir")
                            ("  $Grey" + "VPN".PadRight(18) + "$Reset$VpnDir")
                            ("  $Grey" + (T 'diag_token').PadRight(18) + "$Reset$VaultFile")
                            ("  $Grey" + (T 'vault_cipher').PadRight(18) + "$Reset" + (Get-HtbVaultLabel))
                        )
                        Show-HtbLines (T 'set_paths') $White $paths
                    }
                }
            }
        }
    }
}

function Show-HtbLanguage {
    Select-HtbItem -Title (T 'lang_title') -Color $White -Hint (T 'lang_prompt') -Items @("Español", "English", "Русский")
    if ($ReplyKey -ne "enter") { return }
    switch ($ReplySel) {
        0 { Set-HtbLang "es" }
        1 { Set-HtbLang "en" }
        2 { Set-HtbLang "ru" }
    }
}

function Write-HtbBar([int]$Done, [int]$Total, [string]$Color) {
    $width = 28
    $filled = 0
    if ($Total -gt 0) { $filled = [Math]::Min($width, [Math]::Floor($Done * $width / $Total)) }
    $percent = 0
    if ($Total -gt 0) { $percent = [Math]::Floor($Done * 100 / $Total) }
    return ("$Color" + ("█" * $filled) + "$Reset$Faint" + ("░" * ($width - $filled)) + "$Reset  $Done/$Total  $Grey$percent%$Reset")
}

function Write-HtbBarRow([string]$Label, [int]$Done, [int]$Total, [string]$Color) {
    return ("  $Grey" + $Label.PadRight(18) + "$Reset" + (Write-HtbBar $Done $Total $Color))
}

function Get-HtbInt($Value, $Fallback = 0) {
    if ($null -eq $Value) { return $Fallback }
    $result = 0
    if ([int]::TryParse("$Value", [ref]$result)) { return $result }
    return $Fallback
}

function Show-HtbStats {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $infoResponse = HtbV4Get "/user/info"
    if (-not (Test-HtbApi $infoResponse)) { return }
    $uid = $infoResponse.Data.info.id
    $basic = (HtbV4Get "/user/profile/basic/$uid").Data.profile
    $machines = (HtbV4Get "/user/profile/progress/machines/$uid").Data.profile
    $challenges = (HtbV4Get "/user/profile/progress/challenges/$uid").Data.profile
    $sherlocks = (HtbV4Get "/user/profile/progress/sherlocks/$uid").Data.profile
    $fortresses = (HtbV4Get "/user/profile/progress/fortress/$uid").Data.profile
    $prolabs = (HtbV4Get "/user/profile/progress/prolab/$uid").Data.profile
    $dashboard = (HtbV4Get "/user/dashboard").Data
    $board = (HtbV4Get "/rankings/users").Data.data

    $lines = @(
        ""
        "  $Bold$White$($basic.name)$Reset"
        ""
        ("  $Grey" + (T 'p_rank').PadRight(18) + "$Reset$Yellow$($basic.rank)$Reset")
        ("  $Grey" + (T 'stats_global_rank').PadRight(18) + "$Reset#$($basic.ranking)")
        ("  $Grey" + (T 'p_points').PadRight(18) + "$Reset$($basic.points)")
        (Write-HtbBarRow "$($basic.next_rank)" (Get-HtbInt $basic.current_rank_progress) 100 $Green)
        ""
        "  $Green── $(T 'stats_machines')$Reset"
        (Write-HtbBarRow (T 'stats_solved') (Get-HtbInt $machines.machine_owns.solved) (Get-HtbInt $machines.machine_owns.total) $Green)
    )
    foreach ($entry in $machines.machine_difficulties) {
        $lines += Write-HtbBarRow "$($entry.name)" (Get-HtbInt $entry.owned_machines) (Get-HtbInt $entry.total_machines) (Get-HtbDifficultyColor $entry.name)
    }

    $lines += ""
    $lines += "  $Purple── $(T 'stats_challenges')$Reset"
    $lines += Write-HtbBarRow (T 'stats_solved') (Get-HtbInt $challenges.challenge_owns.solved) (Get-HtbInt $challenges.challenge_owns.total) $Purple
    $top = @($challenges.challenge_categories |
        Where-Object { (Get-HtbInt $_.owned_flags) -gt 0 -or (Get-HtbInt $_.owns) -gt 0 } |
        Sort-Object { -([Math]::Max((Get-HtbInt $_.owned_flags), (Get-HtbInt $_.owns))) } |
        Select-Object -First 8)
    foreach ($entry in $top) {
        $done = [Math]::Max((Get-HtbInt $entry.owned_flags), (Get-HtbInt $entry.owns))
        $total = [Math]::Max((Get-HtbInt $entry.total_flags), (Get-HtbInt $entry.total))
        $lines += Write-HtbBarRow "$($entry.name)" $done $total $Purple
    }

    $lines += ""
    $lines += "  $Cyan── $(T 'stats_sherlocks')$Reset"
    $lines += Write-HtbBarRow (T 'stats_solved') (Get-HtbInt $sherlocks.challenge_owns.solved) (Get-HtbInt $sherlocks.challenge_owns.total) $Cyan
    foreach ($entry in ($sherlocks.challenge_categories | Where-Object { (Get-HtbInt $_.owned_flags) -gt 0 })) {
        $lines += Write-HtbBarRow "$($entry.name)" (Get-HtbInt $entry.owned_flags) (Get-HtbInt $entry.total_flags) $Cyan
    }

    $startedForts = @($fortresses.fortresses | Where-Object { (Get-HtbInt $_.owned_flags) -gt 0 })
    if ($startedForts.Count -gt 0) {
        $lines += ""
        $lines += "  $Red── $(T 'stats_fortress')$Reset"
        foreach ($entry in $startedForts) {
            $lines += Write-HtbBarRow "$($entry.name)" (Get-HtbInt $entry.owned_flags) (Get-HtbInt $entry.total_flags) $Red
        }
    }

    $startedLabs = @($prolabs.prolabs | Where-Object { (Get-HtbInt $_.owned_flags) -gt 0 })
    if ($startedLabs.Count -gt 0) {
        $lines += ""
        $lines += "  $Orange── $(T 'stats_prolabs')$Reset"
        foreach ($entry in $startedLabs) {
            $lines += Write-HtbBarRow "$($entry.name)" (Get-HtbInt $entry.owned_flags) (Get-HtbInt $entry.total_flags) $Orange
        }
    }

    $season = (HtbV4Get "/season/list").Data.data | Where-Object { $_.active -eq $true } | Select-Object -First 1
    if ($season) {
        $rank = (HtbV4Get "/season/user/rank/$($season.id)").Data.data
        $league = "$(HtbValue $rank 'league')"
        if ($league -eq "-") { $league = "→ $(HtbValue $rank 'next_rank.title')" }
        $obtained = Get-HtbInt $rank.total_season_flags.obtained
        $totalFlags = Get-HtbInt $rank.total_season_flags.total
        $lines += ""
        $lines += "  $Purple── $(T 'p_season')$Reset"
        $lines += ("  $Grey" + (T 'season_tier').PadRight(18) + "$Reset$Purple$league$Reset")
        $lines += Write-HtbBarRow (T 'season_flags') $obtained $totalFlags $Green
        $lines += ("  $Grey" + (T 'stats_streak').PadRight(18) + "$Reset$Green" + ("▮" * $obtained) + "$Reset  $obtained")
        $lines += ("  $Faint" + "".PadRight(18) + "$(T 'stats_streak_note')$Reset")
    }

    $lines += ""
    $lines += "  $White── $(T 'stats_leaderboard')$Reset"
    $lines += ("  $Grey" + (T 'stats_online').PadRight(18) + "$Reset$(HtbValue $dashboard 'dashboard_players.online_players')")
    foreach ($entry in ($board | Select-Object -First 5)) {
        $name = "$($entry.name)"
        if ($name.Length -gt 24) { $name = $name.Substring(0, 24) }
        $lines += ("    $Yellow#" + "$($entry.rank)".PadRight(4) + "$Reset $White" + $name.PadRight(24) + "$Reset $Grey$($entry.points) pts$Reset")
    }
    $lines += ("    $Green#" + "$($basic.ranking)".PadRight(4) + "$Reset $Green" + "$($basic.name)".PadRight(24) + "$Reset $Grey$($basic.points) pts$Reset")

    Show-HtbLines (T 'stats_title') $Green $lines
}
