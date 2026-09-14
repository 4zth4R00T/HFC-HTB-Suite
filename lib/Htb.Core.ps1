function Assert-HtbNotRoot {
    if ($IsWindows) { return }
    $uid = 0
    try { $uid = [int](& id -u) } catch { return }
    if ($uid -ne 0) { return }
    if ($env:HTB_ALLOW_ROOT -eq "1") {
        Write-Host "  Ejecutando como root por HTB_ALLOW_ROOT=1." -ForegroundColor Yellow
        return
    }
    Write-Host "  No me ejecuto como root." -ForegroundColor Red
    Write-Host "  La suite solo necesita tu usuario. Si de verdad lo quieres:"
    Write-Host "  HTB_ALLOW_ROOT=1 pwsh ./htb.ps1"
    exit 1
}
Assert-HtbNotRoot

$Script:ConfigDir = Join-Path $HOME ".config/htb-hfc"
$Script:LegacyDir = Join-Path $HOME ".config/htb-suite"
$Script:TokenFile = Join-Path $ConfigDir "token"
$Script:ThemeFile = Join-Path $ConfigDir "theme"
$Script:LangFile  = Join-Path $ConfigDir "lang"
$Script:VpnDir    = if ($env:HTB_VPN_DIR) { $env:HTB_VPN_DIR } else { Join-Path $ConfigDir "vpn" }
$Script:DlDir     = if ($env:HTB_DL_DIR)  { $env:HTB_DL_DIR }  else { Join-Path $HOME "HTB" }

New-Item -ItemType Directory -Force -Path $ConfigDir, $VpnDir, $DlDir -ErrorAction SilentlyContinue | Out-Null

function Protect-HtbConfigDir {
    if ($IsWindows) { return }
    $item = Get-Item $ConfigDir -Force -ErrorAction SilentlyContinue
    if ($item -and $item.LinkType) {
        Write-Host "  $ConfigDir es un enlace simbolico. No guardo secretos ahi." -ForegroundColor Red
        exit 1
    }
    foreach ($dir in @($ConfigDir, $VpnDir)) {
        try {
            [IO.File]::SetUnixFileMode($dir, 'UserRead, UserWrite, UserExecute')
        } catch {
            & chmod 700 $dir 2>$null
        }
    }
}
Protect-HtbConfigDir

$Script:BaseV4   = "https://labs.hackthebox.com/api/v4"
$Script:BaseV5   = "https://labs.hackthebox.com/api/v5"
$Script:StoreUrl = "https://www.hackthebox.com/hacker/subscriptions"
$Script:TokenUrl = "https://app.hackthebox.com/profile/settings"
$Script:ApiToken = $null

$Script:Esc = [char]27
$Script:Reset = "$([char]27)[0m"
$Script:Bold  = "$([char]27)[1m"

function Import-HtbTheme {
    $Script:HtbTheme = "dark"
    if (Test-Path $ThemeFile) {
        $value = (Get-Content $ThemeFile -Raw -ErrorAction SilentlyContinue)
        if ($value -and $value.Trim() -eq "light") { $Script:HtbTheme = "light" }
    }
    if ($HtbTheme -eq "light") {
        $Script:Green   = "$Esc[1;38;2;27;69;58m"
        $Script:Red     = "$Esc[1;38;2;140;51;46m"
        $Script:Cyan    = "$Esc[1;38;2;47;111;140m"
        $Script:Yellow  = "$Esc[1;38;2;150;115;26m"
        $Script:Grey    = "$Esc[0;38;2;41;94;73m"
        $Script:White   = "$Esc[1;38;2;12;26;22m"
        $Script:Purple  = "$Esc[1;38;2;107;74;140m"
        $Script:Orange  = "$Esc[0;38;2;138;85;39m"
        $Script:Faint   = "$Esc[2;38;2;91;122;112m"
    } else {
        $Script:Green   = "$Esc[1;38;2;71;131;107m"
        $Script:Red     = "$Esc[1;38;2;176;69;63m"
        $Script:Cyan    = "$Esc[1;38;2;79;158;196m"
        $Script:Yellow  = "$Esc[1;38;2;201;162;39m"
        $Script:Grey    = "$Esc[0;38;2;122;154;144m"
        $Script:White   = "$Esc[1;38;2;201;214;210m"
        $Script:Purple  = "$Esc[1;38;2;166;126;196m"
        $Script:Orange  = "$Esc[0;38;2;193;122;63m"
        $Script:Faint   = "$Esc[2;38;2;61;94;84m"
    }
}

function Set-HtbTheme([string]$Name) {
    Set-Content -Path $ThemeFile -Value $Name -NoNewline
    Import-HtbTheme
}

function Import-HtbLang {
    $Script:HtbLang = "es"
    if (Test-Path $LangFile) {
        $value = (Get-Content $LangFile -Raw -ErrorAction SilentlyContinue)
        if ($value) {
            $value = $value.Trim()
            if ($value -in @("en", "ru")) { $Script:HtbLang = $value }
        }
    }
    Import-HtbStrings $HtbLang
}

function Set-HtbLang([string]$Code) {
    Set-Content -Path $LangFile -Value $Code -NoNewline
    Import-HtbLang
}

function T([string]$Key) {
    if ($Script:HtbStrings -and $Script:HtbStrings.ContainsKey($Key)) { $Script:HtbStrings[$Key] } else { $Key }
}

function Import-HtbToken {
    if ($env:HTB_TOKEN) {
        $Script:ApiToken = $env:HTB_TOKEN
        Write-HtbNote (T 'token_env_warning')
        return
    }

    if (Test-HtbVault) { Unlock-HtbVault; return }

    $legacy = $null
    foreach ($path in @($TokenFile, (Join-Path $LegacyDir "token"))) {
        if (-not (Test-Path $path)) { continue }
        $value = (Get-Content $path -Raw -ErrorAction SilentlyContinue)
        if ($value -and $value.Trim()) { $legacy = $value.Trim(); break }
    }
    if ($legacy) {
        Clear-Host; Show-HtbMark; Write-Host ""
        Write-HtbNote (T 'vault_migrating')
        Write-Host ""
        $pass = Read-HtbNewPassphrase
        if ($pass -and (Write-HtbVault $legacy $pass)) {
            $Script:ApiToken = $legacy
            Clear-HtbPlaintextToken
            Write-HtbOk (T 'vault_migrated')
            Start-Sleep -Milliseconds 900
            return
        }
        Write-HtbFail (T 'vault_migration_failed')
        exit 1
    }

    New-HtbToken
}

function Clear-HtbPlaintextToken {
    foreach ($path in @($TokenFile, (Join-Path $LegacyDir "token"))) {
        if (-not (Test-Path $path)) { continue }
        try {
            $size = (Get-Item $path).Length
            if ($size -gt 0) { [IO.File]::WriteAllBytes($path, [byte[]]::new($size)) }
        } catch { }
        Remove-Item $path -Force -ErrorAction SilentlyContinue
    }
}

function New-HtbToken {
    Clear-Host
    Show-HtbMark
    Write-Host ""
    Write-HtbLine "  $Cyan$(T 'token_ask')$Reset"
    Write-HtbLine "  $Grey$(T 'token_where'): $TokenUrl$Reset"
    Write-HtbLine "  $Grey$(T 'token_stored'): $VaultFile$Reset"
    Write-Host ""
    $token = Read-HtbPassphrase "token"
    if (-not $token) {
        Write-HtbLine "  $Red$(T 'token_empty')$Reset"
        exit 1
    }
    Write-Host ""
    Write-HtbNote (T 'vault_choose_passphrase')
    $pass = Read-HtbNewPassphrase
    if (-not $pass) {
        Write-HtbFail (T 'vault_cancelled')
        exit 1
    }
    if (-not (Write-HtbVault $token $pass)) {
        Write-HtbFail (T 'vault_write_failed')
        exit 1
    }
    $Script:ApiToken = $token
    Write-HtbOk (T 'vault_sealed')
    Start-Sleep -Milliseconds 900
}

function Unlock-HtbVault {
    Clear-Host
    Show-HtbMark
    Write-Host ""
    Write-HtbField (T 'vault_cipher') (Get-HtbVaultLabel)
    Write-Host ""
    foreach ($attempt in 1..3) {
        $pass = Read-HtbPassphrase (T 'vault_passphrase')
        if (-not $pass) { Write-HtbFail (T 'vault_cancelled'); exit 1 }
        $result = Read-HtbVault $pass
        if ($result.Ok) { $Script:ApiToken = $result.Token; return }
        switch ($result.Reason) {
            "passphrase" { if ($attempt -lt 3) { Write-HtbFail (T 'vault_wrong_passphrase') } }
            "argon2"     { Write-HtbFail (T 'vault_argon2_only'); exit 1 }
            default      { Write-HtbFail (T 'vault_unreadable'); exit 1 }
        }
    }
    Write-HtbFail (T 'vault_wrong_passphrase')
    exit 1
}

function Invoke-HtbApi {
    param([string]$Method = "GET", [string]$Url, [object]$Body)
    $headers = @{ Authorization = "Bearer $ApiToken"; Accept = "application/json" }
    try {
        $params = @{ Method = $Method; Uri = $Url; Headers = $headers; TimeoutSec = 25; ErrorAction = "Stop" }
        if ($null -ne $Body) {
            $params.Body = ($Body | ConvertTo-Json -Depth 10 -Compress)
            $params.ContentType = "application/json"
        }
        return [pscustomobject]@{ Ok = $true; Data = (Invoke-RestMethod @params); Code = 200 }
    } catch {
        $code = 0
        $data = $null
        if ($_.Exception.Response) { $code = [int]$_.Exception.Response.StatusCode }
        if ($_.ErrorDetails.Message) {
            try { $data = $_.ErrorDetails.Message | ConvertFrom-Json -ErrorAction Stop } catch { $data = $null }
        }
        return [pscustomobject]@{ Ok = $false; Data = $data; Code = $code; Message = $_.Exception.Message }
    }
}

function HtbV4Get([string]$Path)  { Invoke-HtbApi -Method GET  -Url "$BaseV4$Path" }
function HtbV5Get([string]$Path)  { Invoke-HtbApi -Method GET  -Url "$BaseV5$Path" }
function HtbV4Post([string]$Path, [object]$Body = @{}) { Invoke-HtbApi -Method POST -Url "$BaseV4$Path" -Body $Body }
function HtbV5Post([string]$Path, [object]$Body = @{}) { Invoke-HtbApi -Method POST -Url "$BaseV5$Path" -Body $Body }

function HtbValue($Object, [string]$Path, $Fallback = "-") {
    if ($null -eq $Object) { return $Fallback }
    $current = $Object
    foreach ($part in $Path.Split('.')) {
        if ($null -eq $current) { return $Fallback }
        $current = $current.$part
    }
    if ($null -eq $current -or "$current" -eq "") { return $Fallback }
    return $current
}

function Get-HtbDate($Value) {
    if ($null -eq $Value) { return "-" }
    if ($Value -is [datetime]) { return $Value.ToString("yyyy-MM-dd") }
    $text = "$Value"
    if ($text.Length -ge 10) { return $text.Substring(0, 10) }
    return $text
}

function Test-HtbApi($Response) {
    if ($Response.Code -eq 429 -or "$($Response.Data.message)" -eq "Too Many Attempts.") {
        Write-Host ""
        Write-HtbNote (T 'api_rate_limited')
        Wait-HtbEnter
        return $false
    }
    if (-not $Response.Ok -and -not $Response.Data) {
        Write-Host ""
        Write-HtbFail (T 'api_bad_response')
        Wait-HtbEnter
        return $false
    }
    return $true
}

function Write-HtbKeys {
    $parts = @()
    foreach ($pair in $args) {
        $key, $label = $pair -split '\|', 2
        $parts += "$Green$key$Reset $Grey$label$Reset"
    }
    return "  " + ($parts -join "   ")
}

function Read-HtbFlagAnswer([string]$Message) {
    $text = "$Message".ToLower()
    foreach ($word in @('incorrect', 'not correct', 'wrong', 'invalid', 'already', 'failed', 'error')) {
        if ($text.Contains($word)) { return 'no' }
    }
    foreach ($word in @('correct', 'owned', 'congrat', 'obtained', 'accepted', 'solved', 'well done')) {
        if ($text.Contains($word)) { return 'si' }
    }
    return 'duda'
}

function Test-HtbPlanMessage([string]$Message) {
    $text = "$Message".ToLower()
    foreach ($word in @('vip', 'upgrade', 'subscription', 'free server', 'requires a', 'premium')) {
        if ($text.Contains($word)) { return $true }
    }
    return $false
}

function Write-HtbPlanNote([string]$Message) {
    if (-not (Test-HtbPlanMessage $Message)) { return }
    Write-Host ""
    Write-HtbNote (T 'vip_required')
    Write-HtbField (T 'your_plan') "$Yellow$Script:HtbPlan$Reset"
    Write-HtbField (T 'plan_upgrade') "$Cyan$StoreUrl$Reset"
}

function Read-HtbActionAnswer([string]$Message) {
    $text = "$Message".ToLower()
    foreach ($word in @('cannot', 'can not', "can't", 'could not', "couldn't", 'not find', 'no active',
                        'please switch', 'not allowed', 'unauthorized', 'forbidden', 'already have',
                        'error', 'invalid', 'failed', 'requires', 'upgrade', 'vip')) {
        if ($text.Contains($word)) { return 'no' }
    }
    foreach ($word in @('spawn', 'started', 'starting', 'deployed', 'playing', 'being', 'terminat',
                        'reset', 'extend', 'success', 'switched')) {
        if ($text.Contains($word)) { return 'si' }
    }
    return 'duda'
}

function Get-HtbWidth {
    try { $w = [Console]::WindowWidth } catch { $w = 100 }
    if (-not $w -or $w -lt 20) { $w = 100 }
    return $w
}

function Get-HtbHeight {
    try { $h = [Console]::WindowHeight } catch { $h = 30 }
    if (-not $h -or $h -lt 10) { $h = 30 }
    return $h
}

function Write-HtbLine([string]$Text) { Write-Host $Text }

function Write-HtbRule([string]$Char = "─") {
    Write-Host ("$Grey" + ($Char * (Get-HtbWidth)) + $Reset)
}

function Write-HtbTitle([string]$Text, [string]$Color = $null) {
    if (-not $Color) { $Color = $Green }
    $width = Get-HtbWidth
    if ($Text.Length -gt ($width - 6)) { $Text = $Text.Substring(0, $width - 6) }
    $fill = "═" * ($width - 2)
    Write-Host "$Color╔$fill╗$Reset"
    Write-Host ("$Color║$Reset$Bold  " + $Text.PadRight($width - 4) + "$Reset$Color║$Reset")
    Write-Host "$Color╚$fill╝$Reset"
}

function Write-HtbOk([string]$Text)   { Write-Host "  $Green✓$Reset $Text" }
function Write-HtbFail([string]$Text) { Write-Host "  $Red✗$Reset $Text" }
function Write-HtbNote([string]$Text) { Write-Host "  $Yellow$Text$Reset" }

function Write-HtbField([string]$Label, [string]$Value, [int]$Width = 18) {
    Write-Host ("  $Grey" + $Label.PadRight($Width) + "$Reset$Value")
}

function Write-HtbRow([string]$Title, [string]$Description, [int]$Width = 20) {
    return "$White" + $Title.PadRight($Width) + "$Reset$Faint$Description$Reset"
}

function Wait-HtbEnter {
    Write-Host ""
    Write-Host "  $Grey$(T 'press_enter')$Reset"
    if ([Console]::IsInputRedirected) { return }
    while ([Console]::KeyAvailable) { [Console]::ReadKey($true) | Out-Null }
    [Console]::ReadKey($true) | Out-Null
}

function Confirm-Htb([string]$Question) {
    $answer = Read-Host "  $Yellow$Question $Grey[$(T 'yn')]$Reset"
    return ($answer -match '^[sSyYдД]$')
}

function Read-HtbText([string]$Label) {
    return (Read-Host "  $Cyan$Label$Reset")
}

function Get-HtbDifficultyColor([string]$Difficulty) {
    switch -Regex ("$Difficulty".ToLower()) {
        '^(very easy|easy)$' { return $Green }
        '^medium$'           { return $Yellow }
        '^hard$'             { return $Red }
        '^insane$'           { return $Purple }
        default              { return $White }
    }
}

function Get-HtbCleanText([string]$Text) {
    $clean = [regex]::Replace("$Text", '<[^>]*>', ' ')
    $clean = $clean -replace '&nbsp;', ' ' -replace '&amp;', '&' -replace '&lt;', '<' -replace '&gt;', '>' -replace '&quot;', '"' -replace '&#039;', "'"
    return ($clean -replace '\s+', ' ').Trim()
}

function Format-HtbParagraph([string]$Text, [int]$Width = 92) {
    $clean = Get-HtbCleanText $Text
    $lines = @()
    $current = ""
    foreach ($word in $clean.Split(' ')) {
        if (($current.Length + $word.Length + 1) -gt $Width) {
            $lines += "  $Faint$current$Reset"
            $current = $word
        } elseif ($current -eq "") {
            $current = $word
        } else {
            $current += " $word"
        }
    }
    if ($current) { $lines += "  $Faint$current$Reset" }
    return $lines
}

$Script:HtbWelcome = @{
    es = @(
        "la enumeración callada rinde más que el ruido"
        "cada flag es una prueba, no una casualidad"
        "el acceso es el principio del trabajo, no el final"
        "quien entiende el sistema termina mandando en él"
        "la paciencia abre más cajas que cualquier exploit"
        "hoy también hay algo nuevo que aprender"
    )
    en = @(
        "quiet enumeration pays better than noise"
        "every flag is proof, never luck"
        "access is where the work starts, not where it ends"
        "whoever understands the system ends up running it"
        "patience opens more boxes than any exploit"
        "there is something new to learn today too"
    )
    ru = @(
        "тихая разведка даёт больше, чем шум"
        "каждый флаг — доказательство, а не везение"
        "доступ — это начало работы, а не её конец"
        "кто разобрался в системе, тот ей и управляет"
        "терпение вскрывает больше боксов, чем любой эксплойт"
        "сегодня тоже есть чему научиться"
    )
}

function Get-HtbWelcome {
    $pool = $HtbWelcome[$HtbLang]
    if (-not $pool) { $pool = $HtbWelcome['es'] }
    return $pool[(Get-Random -Maximum $pool.Count)]
}

$Script:SherlockZipPassword = "hacktheblue"

function Copy-HtbText([string]$Text) {
    if (-not $Text -or $Text -eq "-") { return $false }
    if (Get-Command Set-Clipboard -ErrorAction SilentlyContinue) {
        try { Set-Clipboard -Value $Text -ErrorAction Stop; return $true } catch { }
    }
    foreach ($tool in @(
        @{ Name = "wl-copy";  Args = @() }
        @{ Name = "xclip";    Args = @("-selection", "clipboard") }
        @{ Name = "xsel";     Args = @("--clipboard", "--input") }
        @{ Name = "pbcopy";   Args = @() }
    )) {
        if (-not (Get-Command $tool.Name -ErrorAction SilentlyContinue)) { continue }
        try {
            $psi = [Diagnostics.ProcessStartInfo]::new()
            $psi.FileName = $tool.Name
            foreach ($arg in $tool.Args) { $psi.ArgumentList.Add($arg) }
            $psi.RedirectStandardInput = $true
            $psi.UseShellExecute = $false
            $proc = [Diagnostics.Process]::Start($psi)
            $proc.StandardInput.Write($Text)
            $proc.StandardInput.Close()
            $proc.WaitForExit(2000) | Out-Null
            return $true
        } catch { }
    }
    return $false
}

function Get-HtbClipboard {
    if (Get-Command Get-Clipboard -ErrorAction SilentlyContinue) {
        try { return (Get-Clipboard -Raw -ErrorAction Stop) } catch { }
    }
    foreach ($tool in @(
        @{ Name = "wl-paste"; Args = @("--no-newline") }
        @{ Name = "xclip";    Args = @("-selection", "clipboard", "-o") }
        @{ Name = "xsel";     Args = @("--clipboard", "--output") }
        @{ Name = "pbpaste";  Args = @() }
    )) {
        if (-not (Get-Command $tool.Name -ErrorAction SilentlyContinue)) { continue }
        try { return (& $tool.Name @($tool.Args) 2>$null | Select-Object -First 1) } catch { }
    }
    return $null
}

function Show-HtbCopied([string]$Text) {
    Write-Host ""
    if (Copy-HtbText $Text) {
        Write-HtbOk (T 'copied')
        foreach ($line in ($Text -split "`n")) { Write-Host "  $Cyan$line$Reset" }
    } else {
        Write-HtbNote (T 'nothing_to_copy')
    }
    Wait-HtbEnter
}

function Read-HtbFlagText([string]$Label) {
    $pasted = "$(Get-HtbClipboard)".Trim()
    if ($pasted -match '^HTB\{.*\}$' -or $pasted -match '^[0-9a-fA-F]{32}$') {
        Write-Host "  $Faint$(T 'clipboard_flag'): $pasted$Reset"
        $typed = Read-Host "  $Cyan$Label$Reset $Grey[Enter = $(T 'clipboard_flag')]$Reset"
        if (-not $typed) { return $pasted }
        return $typed
    }
    return (Read-Host "  $Cyan$Label$Reset")
}

function Enable-HtbMouse  { [Console]::Write("$Esc[?1000h$Esc[?1006h") }
function Disable-HtbMouse { [Console]::Write("$Esc[?1006l$Esc[?1000l") }

function Read-HtbKey {
    if ([Console]::IsInputRedirected) { return "QUIT" }
    $key = [Console]::ReadKey($true)
    if ($key.KeyChar -eq [char]27) {
        $next = if ([Console]::KeyAvailable) { [Console]::ReadKey($true) } else { $null }
        if ($null -ne $next -and $next.KeyChar -eq '[') {
            $third = if ([Console]::KeyAvailable) { [Console]::ReadKey($true) } else { $null }
            if ($null -ne $third -and $third.KeyChar -eq '<') {
                $buffer = ""
                $final = ""
                while ([Console]::KeyAvailable) {
                    $c = [Console]::ReadKey($true).KeyChar
                    if ($c -ceq 'M' -or $c -ceq 'm') { $final = "$c"; break }
                    $buffer += $c
                }
                $parts = $buffer -split ';'
                if ($parts.Count -ge 3) {
                    switch ($parts[0]) {
                        '64' { return "UP" }
                        '65' { return "DOWN" }
                        '0'  {
                            if ($final -ceq 'M') {
                                $Script:ReplyMouseRow = [int]$parts[2]
                                $Script:ReplyMouseCol = [int]$parts[1]
                                return "CLICK"
                            }
                            return "NONE"
                        }
                        '2'  { if ($final -ceq 'M') { return "BACK" } ; return "NONE" }
                    }
                }
                return "NONE"
            }
            return "BACK"
        }
        return "BACK"
    }
    switch ($key.Key) {
        'UpArrow'    { return "UP" }
        'DownArrow'  { return "DOWN" }
        'RightArrow' { return "SELECT" }
        'LeftArrow'  { return "BACK" }
        'Enter'      { return "SELECT" }
        'Escape'     { return "BACK" }
        'Backspace'  { return "BACK" }
        'Home'       { return "HOME" }
        'End'        { return "END" }
        'PageUp'     { return "PGUP" }
        'PageDown'   { return "PGDN" }
        'Spacebar'   { return "PGDN" }
    }
    if ($key.KeyChar) {
        switch ($key.KeyChar) {
            'j' { return "DOWN" }
            'k' { return "UP" }
            'q' { return "BACK" }
            'Q' { return "BACK" }
            '?' { return "HELP" }
        }
        if ([int][char]$key.KeyChar -ge 32) { return "CHAR:$($key.KeyChar)" }
    }
    return "NONE"
}

function Select-HtbItem {
    param(
        [string]$Title,
        [string]$Color,
        [string]$Hint,
        [string[]]$Items,
        [int]$Start = 0,
        [string[]]$Header = @()
    )
    $Script:ReplySel = -1
    $Script:ReplyKey = "quit"
    $count = $Items.Count
    if ($count -eq 0) { return }
    if (-not $Color) { $Color = $Green }

    if ([Console]::IsInputRedirected) {
        Clear-Host; Write-HtbTitle $Title $Color; Write-Host ""
        for ($i = 0; $i -lt $count; $i++) { Write-Host ("  {0,3}) {1}" -f ($i + 1), $Items[$i]) }
        if ($Hint) { Write-Host "  $Grey$Hint$Reset" }
        $answer = Read-Host "  $(T 'pick_number')"
        if ($answer -match '^\d+$' -and [int]$answer -ge 1 -and [int]$answer -le $count) {
            $Script:ReplySel = [int]$answer - 1
            $Script:ReplyKey = "enter"
        }
        return
    }

    $cur = $Start
    if ($cur -lt 0 -or $cur -ge $count) { $cur = 0 }
    while ($true) {
        $window = [Math]::Max(3, (Get-HtbHeight) - 9 - $Header.Count)
        $first = 0
        if ($cur -ge $window) { $first = $cur - $window + 1 }
        if ($first -gt $count - $window) { $first = $count - $window }
        if ($first -lt 0) { $first = 0 }

        Enable-HtbMouse
        Clear-Host; Write-HtbTitle $Title $Color
        foreach ($line in $Header) { Write-Host $line }
        Write-Host ""
        for ($i = $first; $i -lt ($first + $window) -and $i -lt $count; $i++) {
            if ($i -eq $cur) { Write-Host "  $Color▌$Reset $($Items[$i])" }
            else { Write-Host "    $($Items[$i])" }
        }
        Write-Host ""
        if ($Hint) { Write-Host $Hint }
        Write-Host ("  $Grey$($cur + 1)/$count$Reset   $Green↑↓$Reset $Grey$(T 'nav_move')$Reset   $Green↵$Reset $Grey$(T 'nav_open')$Reset   $Green←$Reset $Grey$(T 'nav_back')$Reset   ${Green}?$Reset $Grey$(T 'nav_help')$Reset")

        $pressed = Read-HtbKey
        if ($pressed -eq "CLICK") {
            $firstRow = 5 + $Header.Count
            $hit = $Script:ReplyMouseRow - $firstRow + $first
            if ($Script:ReplyMouseRow -ge $firstRow -and $hit -ge 0 -and $hit -lt $count) {
                if ($hit -eq $cur) {
                    Disable-HtbMouse
                    $Script:ReplySel = $cur; $Script:ReplyKey = "enter"; return
                }
                $cur = $hit
            }
            continue
        }
        switch -Regex ($pressed) {
            '^UP$'     { if ($cur -gt 0) { $cur-- } else { $cur = $count - 1 }; continue }
            '^DOWN$'   { if ($cur -lt $count - 1) { $cur++ } else { $cur = 0 }; continue }
            '^PGUP$'   { $cur = [Math]::Max(0, $cur - $window); continue }
            '^PGDN$'   { $cur = [Math]::Min($count - 1, $cur + $window); continue }
            '^HOME$'   { $cur = 0; continue }
            '^END$'    { $cur = $count - 1; continue }
            '^SELECT$' { Disable-HtbMouse; $Script:ReplySel = $cur; $Script:ReplyKey = "enter"; return }
            '^BACK$'   { Disable-HtbMouse; $Script:ReplySel = -1;  $Script:ReplyKey = "quit";  return }
            '^QUIT$'   { Disable-HtbMouse; $Script:ReplySel = -1;  $Script:ReplyKey = "quit";  return }
            '^HELP$'   { Disable-HtbMouse; $Script:ReplySel = $cur; $Script:ReplyKey = "help"; return }
            '^CHAR:'   { Disable-HtbMouse; $Script:ReplySel = $cur; $Script:ReplyKey = $pressed.Substring(5); return }
        }
    }
}

function Write-HtbHelpRow([string]$Keys, [string]$Text) {
    Write-Host ("  $Green" + $Keys.PadRight(22) + "$Reset$Grey$Text$Reset")
}

function Show-HtbHelp {
    Clear-Host; Write-HtbTitle (T 'help_title') $Cyan; Write-Host ""
    Write-HtbHelpRow "↑ ↓        k j"      (T 'help_move')
    Write-HtbHelpRow "↵          →"        (T 'help_select')
    Write-HtbHelpRow "←          Esc  q"   (T 'help_back')
    Write-HtbHelpRow "PageUp  PageDown"    (T 'help_page')
    Write-HtbHelpRow "Home  End"          (T 'help_edges')
    Write-HtbHelpRow (T 'help_keys')       (T 'help_actions')
    Write-HtbHelpRow "?"                   (T 'help_show')
    Write-HtbHelpRow "c"                   (T 'help_copy')
    Write-HtbHelpRow (T 'help_mouse_key')  (T 'help_mouse')
    Write-HtbHelpRow "Ctrl+C"              (T 'help_quit')
    Wait-HtbEnter
}

function Show-HtbLines([string]$Title, [string]$Color, [string[]]$Lines) {
    Clear-Host; Write-HtbTitle $Title $Color; Write-Host ""
    foreach ($line in $Lines) { Write-Host $line }
    Wait-HtbEnter
}

function Save-HtbFile([string]$Url, [string]$FileName, [bool]$Encrypted = $false, [string]$Password = "") {
    $out = Join-Path $DlDir $FileName
    try {
        Invoke-WebRequest -Uri $Url -Headers @{ Authorization = "Bearer $ApiToken" } -OutFile $out -TimeoutSec 120 -ErrorAction Stop
        Protect-HtbFile $out
        $size = (Get-Item $out).Length
        Write-Host ""
        if ($size -gt 100) {
            Write-HtbOk "$(T 'saved_to') $out ($size B)"
            if ($Encrypted) {
                if ($Password -and $Password -ne "-") { Write-HtbNote "$(T 'zip_password_is') $Password" }
                else { Write-HtbNote (T 'zip_password') }
            }
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

function Get-HtbSafeName([string]$Name) {
    return ([regex]::Replace("$Name", '[^A-Za-z0-9._-]', '_'))
}

function Select-HtbFilter([string]$Title, $Options) {
    $Script:ReplyFilterSet = $false
    $Script:ReplyFilter = ""
    Select-HtbItem -Title $Title -Color $Cyan -Items @($Options | ForEach-Object { $_.Label })
    if ($ReplyKey -eq "enter") {
        $Script:ReplyFilter = $Options[$ReplySel].Value
        $Script:ReplyFilterSet = $true
    }
}

function Get-HtbFilterLabel([string]$Value, $Options) {
    $match = $Options | Where-Object { $_.Value -eq $Value } | Select-Object -First 1
    if ($match) { return $match.Label }
    return $Value
}
