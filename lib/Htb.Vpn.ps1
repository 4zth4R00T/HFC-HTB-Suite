$Script:VpnProducts = @("labs", "starting_point", "sherlocks", "fortresses", "release_arena", "competitive")

function Show-HtbVpnMenu {
    $at = 0
    while ($true) {
        Select-HtbItem -Title "$(T 'vpn_title') · $(T 'vpn_product')" -Color $Purple `
                       -Items @($VpnProducts | ForEach-Object { "$White$_$Reset" }) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { return }
            'help' { Show-HtbHelp }
            'enter' { Show-HtbVpnServers $VpnProducts[$ReplySel] }
        }
    }
}

function Get-HtbVpnServers($Options) {
    $servers = @()
    foreach ($region in $Options.PSObject.Properties) {
        foreach ($kind in $region.Value.PSObject.Properties) {
            foreach ($entry in $kind.Value.servers.PSObject.Properties) { $servers += $entry.Value }
        }
    }
    return @($servers | Sort-Object location, friendly_name)
}

function Show-HtbVpnServers([string]$Product) {
    Write-Host "  $Grey$(T 'loading')$Reset"
    $response = HtbV4Get "/connections/servers?product=$Product"
    if (-not $response.Ok -or -not $response.Data.status) {
        Clear-Host; Write-HtbTitle "$(T 'vpn_title') · $Product" $Purple; Write-Host ""
        Write-HtbNote (T 'vpn_unavailable')
        Write-HtbField "" "$(HtbValue $response.Data 'message')"
        Write-HtbPlanNote "$(HtbValue $response.Data 'message')"
        Wait-HtbEnter; return
    }
    $assigned = "$(HtbValue $response.Data 'data.assigned.id')"
    $assignedName = "$(HtbValue $response.Data 'data.assigned.friendly_name')"
    $assignedLocation = "$(HtbValue $response.Data 'data.assigned.location')"
    $servers = Get-HtbVpnServers $response.Data.data.options
    if ($servers.Count -eq 0) { Write-HtbFail (T 'no_results'); Wait-HtbEnter; return }

    $at = 0
    while ($true) {
        $rows = foreach ($s in $servers) {
            $here = ("$($s.id)" -eq $assigned)
            $mark = if ($here) { "▶" } else { " " }
            $markColor = if ($here) { $Green } else { $Faint }
            $full = if ($s.full) { " $Red$(T 'vpn_full')$Reset" } else { "" }
            $name = "$($s.friendly_name)"; if ($name.Length -gt 30) { $name = $name.Substring(0, 30) }
            ("$markColor$mark$Reset $White" + $name.PadRight(30) + "$Reset $Cyan" +
             "$($s.location)".PadRight(11) + "$Reset $Faint$($s.current_clients) $(T 'vpn_clients')$Reset$full")
        }
        $header = "$(T 'vpn_title') · $Product · $(T 'vpn_assigned'): $assignedName ($assigned, $assignedLocation)"
        Select-HtbItem -Title $header -Color $Purple -Hint (Write-HtbKeys "↵|$(T 'act_download') UDP" "t|$(T 'act_download') TCP" "s|$(T 'act_switch')") -Items @($rows) -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        $target = if ($ReplySel -ge 0) { "$($servers[$ReplySel].id)" } else { $null }
        switch -Regex ($ReplyKey) {
            '^quit$'  { return }
            '^help$'  { Show-HtbHelp }
            '^enter$' { if (Save-HtbVpnFile $Product $target 0 $assigned) { $assigned = $target; $assignedName = "$($servers[$ReplySel].friendly_name)" } }
            '^[tT]$'  { if (Save-HtbVpnFile $Product $target 1 $assigned) { $assigned = $target; $assignedName = "$($servers[$ReplySel].friendly_name)" } }
            '^[sS]$'  { if (Switch-HtbVpnServer $target) { $assigned = $target; $assignedName = "$($servers[$ReplySel].friendly_name)" } }
        }
    }
}

function Switch-HtbVpnServer([string]$ServerId) {
    Write-Host ""
    Write-Host "  $Grey$(T 'vpn_switching')$Reset"
    $result = HtbV4Post "/connections/servers/switch/$ServerId"
    if ($result.Data.status -eq $true) {
        Write-HtbOk "$(HtbValue $result.Data 'message') → $(HtbValue $result.Data 'data.friendly_name')"
        Wait-HtbEnter
        return $true
    }
    Write-HtbFail "$(HtbValue $result.Data 'message')"
    Wait-HtbEnter
    return $false
}

function Save-HtbVpnFile([string]$Product, [string]$ServerId, [int]$Tcp, [string]$Assigned) {
    if ($ServerId -ne $Assigned) {
        Write-Host ""
        if (-not (Confirm-Htb (T 'confirm_switch'))) { return $false }
        $switch = HtbV4Post "/connections/servers/switch/$ServerId"
        if ($switch.Data.status -ne $true) {
            Write-HtbFail "$(HtbValue $switch.Data 'message')"; Wait-HtbEnter; return $false
        }
    }
    $proto = if ($Tcp -eq 1) { "tcp" } else { "udp" }
    $out = Join-Path $DlDir "htb_${Product}_${ServerId}_${proto}.ovpn"
    try {
        Invoke-WebRequest -Uri "$BaseV4/access/ovpnfile/$ServerId/$Tcp" -Headers @{ Authorization = "Bearer $ApiToken" } -OutFile $out -TimeoutSec 60 -ErrorAction Stop
        Protect-HtbFile $out
        $size = (Get-Item $out).Length
        Write-Host ""
        if ($size -gt 500) {
            $current = Join-Path $VpnDir "current.ovpn"
            Copy-Item $out $current -Force
            Protect-HtbFile $current
            Write-HtbOk (T 'vpn_saved')
            Write-HtbField (T 'saved_to') $out
            Write-HtbField (T 'vpn_copy') $VpnDir
            Write-HtbField (T 'vpn_connect') $out
            Wait-HtbEnter
            return $true
        }
        Remove-Item $out -ErrorAction SilentlyContinue
        Write-HtbFail (T 'download_failed')
    } catch {
        Write-Host ""
        Write-HtbFail "$(T 'download_failed'): $($_.Exception.Message)"
    }
    Wait-HtbEnter
    return $false
}
