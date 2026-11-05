
$Script:VaultFile      = Join-Path $ConfigDir "token.vault"
$Script:VaultMagic     = [Text.Encoding]::ASCII.GetBytes("HTBHFC")
$Script:VaultVersion   = 1
$Script:VaultKdfArgon2 = 1
$Script:VaultKdfPbkdf2 = 2
$Script:VaultIters     = 600000
$Script:VaultMinPass   = 12
$Script:VaultPrefixLen = 20
$Script:VaultSaltLen   = 8
$Script:VaultTagLen    = 32
$Script:VaultPadTo     = 64

function ConvertTo-HtbBE32([int]$Value) {
    $bytes = [BitConverter]::GetBytes([uint32]$Value)
    if ([BitConverter]::IsLittleEndian) { [array]::Reverse($bytes) }
    return ,$bytes
}

function Get-HtbSlice([byte[]]$Bytes, [int]$Offset, [int]$Length) {
    $out = [byte[]]::new($Length)
    [array]::Copy($Bytes, $Offset, $out, 0, $Length)
    return ,$out
}

function Test-HtbSameBytes([byte[]]$Left, [byte[]]$Right) {
    if ($Left.Length -ne $Right.Length) { return $false }
    $diff = 0
    for ($i = 0; $i -lt $Left.Length; $i++) {
        $diff = $diff -bor ($Left[$i] -bxor $Right[$i])
    }
    return ($diff -eq 0)
}

function ConvertFrom-HtbBE32([byte[]]$Bytes, [int]$Offset) {
    $slice = Get-HtbSlice $Bytes $Offset 4
    if ([BitConverter]::IsLittleEndian) { [array]::Reverse($slice) }
    return [BitConverter]::ToUInt32($slice, 0)
}

function Get-HtbVaultKeys([string]$Passphrase, [byte[]]$Salt, [int]$Iterations) {
    $kdf = [Security.Cryptography.Rfc2898DeriveBytes]::new(
        [Text.Encoding]::UTF8.GetBytes($Passphrase),
        $Salt,
        $Iterations,
        [Security.Cryptography.HashAlgorithmName]::SHA512
    )
    try { return ,$kdf.GetBytes(80) } finally { $kdf.Dispose() }
}

function Get-HtbVaultPlaintext([string]$Token) {
    $raw = [Text.Encoding]::UTF8.GetBytes($Token)
    if ($raw.Length -eq 0 -or $raw.Length -gt 0xFFFF) { return $null }
    $body  = 2 + $raw.Length
    $total = [math]::Ceiling($body / $VaultPadTo) * $VaultPadTo
    $plain = [byte[]]::new($total)
    $len   = [BitConverter]::GetBytes([uint16]$raw.Length)
    if ([BitConverter]::IsLittleEndian) { [array]::Reverse($len) }
    [array]::Copy($len, 0, $plain, 0, 2)
    [array]::Copy($raw, 0, $plain, 2, $raw.Length)
    if ($total -gt $body) {
        $pad = [byte[]]::new($total - $body)
        [Security.Cryptography.RandomNumberGenerator]::Fill($pad)
        [array]::Copy($pad, 0, $plain, $body, $pad.Length)
    }
    [array]::Clear($raw, 0, $raw.Length)
    return ,$plain
}

function Clear-HtbBytes([byte[]]$Bytes) {
    if ($Bytes) { [array]::Clear($Bytes, 0, $Bytes.Length) }
}

function Protect-HtbFile([string]$Path) {
    if ($IsWindows) { return }
    try {
        [IO.File]::SetUnixFileMode($Path, 'UserRead, UserWrite')
    } catch {
        & chmod 600 $Path 2>$null
    }
}

function Test-HtbVault { Test-Path $VaultFile }

function Write-HtbVault([string]$Token, [string]$Passphrase) {
    $salt = [byte[]]::new($VaultSaltLen)
    [Security.Cryptography.RandomNumberGenerator]::Fill($salt)
    $okm = $null; $plain = $null; $cipher = $null
    try {
        $okm   = Get-HtbVaultKeys $Passphrase $salt $VaultIters
        $plain = Get-HtbVaultPlaintext $Token
        if (-not $plain) { return $false }

        $aes = [Security.Cryptography.Aes]::Create()
        try {
            $aes.KeySize = 256
            $aes.Mode    = [Security.Cryptography.CipherMode]::CBC
            $aes.Padding = [Security.Cryptography.PaddingMode]::PKCS7
            $aes.Key     = Get-HtbSlice $okm 0 32
            $aes.IV      = Get-HtbSlice $okm 32 16
            $enc = $aes.CreateEncryptor()
            try { $cipher = $enc.TransformFinalBlock($plain, 0, $plain.Length) } finally { $enc.Dispose() }
        } finally { $aes.Dispose() }

        $body = [Collections.Generic.List[byte]]::new()
        $body.AddRange($VaultMagic)
        $body.Add([byte]$VaultVersion)
        $body.Add([byte]$VaultKdfPbkdf2)
        $body.AddRange((ConvertTo-HtbBE32 $VaultIters))
        $body.AddRange((ConvertTo-HtbBE32 0))
        $body.AddRange((ConvertTo-HtbBE32 0))
        $body.AddRange($salt)
        $body.AddRange((ConvertTo-HtbBE32 $cipher.Length))
        $body.AddRange($cipher)

        $hmac = [Security.Cryptography.HMACSHA256]::new((Get-HtbSlice $okm 48 32))
        try { $tag = $hmac.ComputeHash($body.ToArray()) } finally { $hmac.Dispose() }
        $body.AddRange($tag)

        Set-Content -Path $VaultFile -Value ([Convert]::ToBase64String($body.ToArray())) -NoNewline
        Protect-HtbFile $VaultFile
        return $true
    } catch {
        return $false
    } finally {
        Clear-HtbBytes $okm
        Clear-HtbBytes $plain
    }
}

function Read-HtbVault([string]$Passphrase) {
    $fail = { param($reason) [pscustomobject]@{ Ok = $false; Token = $null; Reason = $reason } }
    try {
        $blob = [Convert]::FromBase64String((Get-Content $VaultFile -Raw).Trim())
    } catch {
        return (& $fail "corrupt")
    }
    if ($blob.Length -lt ($VaultPrefixLen + 4 + $VaultTagLen)) { return (& $fail "corrupt") }
    for ($i = 0; $i -lt 6; $i++) {
        if ($blob[$i] -ne $VaultMagic[$i]) { return (& $fail "corrupt") }
    }
    if ($blob[6] -ne $VaultVersion) { return (& $fail "corrupt") }
    if ($blob[7] -eq $VaultKdfArgon2) { return (& $fail "argon2") }
    if ($blob[7] -ne $VaultKdfPbkdf2) { return (& $fail "corrupt") }

    $iters = [int](ConvertFrom-HtbBE32 $blob 8)
    if ($iters -lt 100000) { return (& $fail "corrupt") }

    $salt      = Get-HtbSlice $blob $VaultPrefixLen $VaultSaltLen
    $headerLen = $VaultPrefixLen + $VaultSaltLen + 4
    $ctLen     = [int](ConvertFrom-HtbBE32 $blob ($VaultPrefixLen + $VaultSaltLen))
    if ($ctLen -le 0 -or ($ctLen % 16) -ne 0 -or $blob.Length -ne ($headerLen + $ctLen + $VaultTagLen)) {
        return (& $fail "corrupt")
    }

    $okm = $null; $plain = $null
    try {
        $okm = Get-HtbVaultKeys $Passphrase $salt $iters

        $hmac = [Security.Cryptography.HMACSHA256]::new((Get-HtbSlice $okm 48 32))
        try { $tag = $hmac.ComputeHash($blob, 0, $headerLen + $ctLen) } finally { $hmac.Dispose() }
        $stored = Get-HtbSlice $blob ($headerLen + $ctLen) $VaultTagLen
        if (-not (Test-HtbSameBytes $tag $stored)) {
            return (& $fail "passphrase")
        }

        $aes = [Security.Cryptography.Aes]::Create()
        try {
            $aes.KeySize = 256
            $aes.Mode    = [Security.Cryptography.CipherMode]::CBC
            $aes.Padding = [Security.Cryptography.PaddingMode]::PKCS7
            $aes.Key     = Get-HtbSlice $okm 0 32
            $aes.IV      = Get-HtbSlice $okm 32 16
            $dec = $aes.CreateDecryptor()
            try { $plain = $dec.TransformFinalBlock($blob, $headerLen, $ctLen) } finally { $dec.Dispose() }
        } finally { $aes.Dispose() }

        $size = ([int]$plain[0] -shl 8) -bor [int]$plain[1]
        if ($size -le 0 -or (2 + $size) -gt $plain.Length) { return (& $fail "corrupt") }
        $token = [Text.Encoding]::UTF8.GetString($plain, 2, $size)
        return [pscustomobject]@{ Ok = $true; Token = $token; Reason = $null }
    } catch {
        return (& $fail "corrupt")
    } finally {
        Clear-HtbBytes $okm
        Clear-HtbBytes $plain
    }
}

function Get-HtbVaultLabel {
    if (-not (Test-HtbVault)) { return "-" }
    try {
        $blob = [Convert]::FromBase64String((Get-Content $VaultFile -Raw).Trim())
        $p1 = [int](ConvertFrom-HtbBE32 $blob 8)
        switch ($blob[7]) {
            1 { return "Argon2id m=$([int]($p1 / 1024))MiB" }
            2 { return "PBKDF2-HMAC-SHA512 $p1 $(T 'vault_rounds')" }
            default { return "-" }
        }
    } catch { return "-" }
}

function Read-HtbPassphrase([string]$Label) {
    if ([Console]::IsInputRedirected) { return $null }
    $secure = Read-Host "  $Label" -AsSecureString
    if (-not $secure -or $secure.Length -eq 0) { return $null }
    $ptr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    try {
        return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($ptr)
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr)
        $secure.Dispose()
    }
}

function Read-HtbNewPassphrase {
    foreach ($attempt in 1..3) {
        $first = Read-HtbPassphrase (T 'vault_new_passphrase')
        if (-not $first) { return $null }
        if ($first.Length -lt $VaultMinPass) {
            Write-HtbFail (T 'vault_passphrase_short')
            continue
        }
        $second = Read-HtbPassphrase (T 'vault_repeat_passphrase')
        if (-not $second) { return $null }
        if ($first -cne $second) {
            Write-HtbFail (T 'vault_passphrase_mismatch')
            continue
        }
        return $first
    }
    return $null
}

function Clear-HtbVault {
    foreach ($path in @($VaultFile, $TokenFile, (Join-Path $LegacyDir "token"))) {
        if (-not (Test-Path $path)) { continue }
        try {
            $size = (Get-Item $path).Length
            if ($size -gt 0) {
                [IO.File]::WriteAllBytes($path, [byte[]]::new($size))
            }
        } catch { }
        Remove-Item $path -Force -ErrorAction SilentlyContinue
    }
}
