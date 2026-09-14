$Script:HfcMark = @(
    '██╗  ██╗███████╗ ██████╗'
    '██║  ██║██╔════╝██╔════╝'
    '███████║█████╗  ██║     '
    '██╔══██║██╔══╝  ██║     '
    '██║  ██║██║     ╚██████╗'
    '╚═╝  ╚═╝╚═╝      ╚═════╝'
)

$Script:Wordmark = @(
    '██╗  ██╗████████╗██████╗   ███████╗██╗   ██╗██╗████████╗███████╗'
    '██║  ██║╚══██╔══╝██╔══██╗  ██╔════╝██║   ██║██║╚══██╔══╝██╔════╝'
    '███████║   ██║   ██████╔╝  ███████╗██║   ██║██║   ██║   █████╗  '
    '██╔══██║   ██║   ██╔══██╗  ╚════██║██║   ██║██║   ██║   ██╔══╝  '
    '██║  ██║   ██║   ██████╔╝  ███████║╚██████╔╝██║   ██║   ███████╗'
    '╚═╝  ╚═╝   ╚═╝   ╚═════╝   ╚══════╝ ╚═════╝ ╚═╝   ╚═╝   ╚══════╝'
)

$Script:HtbAuthor          = 'Aztharot'
$Script:HtbAuthorInstagram = 'instagram.com/its_aztharot'
$Script:HtbAuthorGithub    = 'github.com/Aztharot-RO0t3d'
$Script:HtbCrew            = 'HFC — Hackers Fight Club'
$Script:HtbCrewInstagram   = 'instagram.com/hackersfightclub'
$Script:HtbCrewBlog        = 'blog.hackersfight.club'
$Script:HtbThanks          = 'Lorne · Ookami · Pablocro'

function Get-HtbIndent([int]$Width) {
    $pad = [Math]::Floor(((Get-HtbWidth) - $Width) / 2)
    if ($pad -lt 0) { $pad = 0 }
    return $pad
}

function Write-HtbBlock([string[]]$Block) {
    $pad = ' ' * (Get-HtbIndent $Block[0].Length)
    foreach ($line in $Block) { Write-Host "$pad$Green$line$Reset" }
}

function Show-HtbMark { Write-HtbBlock $HfcMark }

function Show-HtbFollow {
    $pad = ' ' * (Get-HtbIndent 58)
    Write-Host "$pad$White$HtbAuthor$Reset $Grey— $(T 'follow_author')$Reset $Cyan$HtbAuthorInstagram$Reset"
    Write-Host "$pad$Green$HtbCrew$Reset $Grey— $(T 'follow_crew')$Reset $Cyan$HtbCrewInstagram$Reset"
}

function Get-HtbFollowLine {
    return "$White$HtbAuthor$Reset $Grey$(T 'follow_author')$Reset $Cyan$HtbAuthorInstagram$Reset   $Green$HtbCrew$Reset $Grey$(T 'follow_crew')$Reset $Cyan$HtbCrewInstagram$Reset"
}

function Start-HtbBoot {
    if ($env:HTB_NO_ANIM -eq "1") { return }
    Clear-Host
    Write-Host ""
    Write-HtbBlock $HfcMark
    Write-Host ""
    Write-HtbBlock $Wordmark
    Write-Host ""
    Show-HtbFollow
    Write-Host ""
    $pad = ' ' * (Get-HtbIndent 50)
    Write-Host "$pad$Faint$(Get-HtbWelcome)$Reset"
    Start-Sleep -Milliseconds 900
}
