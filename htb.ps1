#Requires -Version 7.0

$ScriptDir = $PSScriptRoot
. (Join-Path $ScriptDir "lib/Htb.Core.ps1")
. (Join-Path $ScriptDir "lib/Htb.Vault.ps1")
. (Join-Path $ScriptDir "lib/Htb.I18n.ps1")
. (Join-Path $ScriptDir "lib/Htb.Brand.ps1")
. (Join-Path $ScriptDir "lib/Htb.Machines.ps1")
. (Join-Path $ScriptDir "lib/Htb.Catalog.ps1")
. (Join-Path $ScriptDir "lib/Htb.Vpn.ps1")
. (Join-Path $ScriptDir "lib/Htb.Profile.ps1")

function Get-HtbStatusLine {
    $info = (HtbV4Get "/user/info").Data.info
    if (-not $info -or -not $info.name) { return (T 'main_prompt') }
    $Script:HtbPlan = "$($info.subscriptionType)"
    $Script:HtbVip = [bool]$info.canAccessVIP
    $active = (HtbV5Get "/virtual_machine/active").Data.info
    if (-not $active -or -not $active.name) { return "$($info.name)" }
    $label = "$($active.name)"
    if ($active.type -and $active.type -ne "machine") { $label += " ($($active.type))" }
    $ip = (HtbV4Get "/machine/active").Data.info.ip
    if ($ip) { return "$($info.name)  ·  $label $ip" }
    return "$($info.name)  ·  $label"
}

function Start-HtbMainMenu {
    $at = 0
    $status = Get-HtbStatusLine
    while ($true) {
        $items = @(
            (Write-HtbRow (T 'menu_machines')    (T 'menu_machines_desc'))
            (Write-HtbRow (T 'menu_catalog')     (T 'menu_catalog_desc'))
            (Write-HtbRow (T 'menu_vpn')         (T 'menu_vpn_desc'))
            (Write-HtbRow (T 'menu_profile')     (T 'menu_profile_desc'))
            (Write-HtbRow (T 'menu_stats')       (T 'menu_stats_desc'))
            (Write-HtbRow (T 'menu_diagnostics') (T 'menu_diagnostics_desc'))
            (Write-HtbRow (T 'menu_settings')    (T 'menu_settings_desc'))
            (Write-HtbRow (T 'menu_credits')     (T 'menu_credits_desc'))
            "$Yellow$(T 'menu_quit')$Reset"
        )
        Select-HtbItem -Title $status -Color $Green -Hint (Get-HtbFollowLine) -Items $items -Start $at
        if ($ReplySel -ge 0) { $at = $ReplySel }
        switch ($ReplyKey) {
            'quit' { Clear-Host; Write-Host ""; Write-Host "  $Grey$(T 'session_ended')$Reset"; Write-Host ""; return }
            'help' { Show-HtbHelp }
            'enter' {
                switch ($ReplySel) {
                    0 { Show-HtbMachinesMenu }
                    1 { Show-HtbCatalogMenu }
                    2 { Show-HtbVpnMenu }
                    3 { Show-HtbProfile }
                    4 { Show-HtbStats }
                    5 { Show-HtbDiagnostics }
                    6 { Show-HtbSettings }
                    7 { Show-HtbCredits }
                    8 { Clear-Host; Write-Host ""; Write-Host "  $Grey$(T 'session_ended')$Reset"; Write-Host ""; return }
                }
                $status = Get-HtbStatusLine
            }
        }
    }
}

try {
    Import-HtbTheme
    Import-HtbLang
    if (-not (Test-Path $LangFile)) { Show-HtbLanguage }
    Start-HtbBoot
    Import-HtbToken
    Start-HtbMainMenu
} finally {
    Disable-HtbMouse
    [Console]::Write("$Esc[?25h")
}
