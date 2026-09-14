pub const AUTHOR: &str = "Aztharot";
pub const AUTHOR_INSTAGRAM: &str = "instagram.com/its_aztharot";
pub const AUTHOR_GITHUB: &str = "github.com/Aztharot-RO0t3d";
pub const CREW: &str = "HFC — Hackers Fight Club";
pub const CREW_INSTAGRAM: &str = "instagram.com/hackersfightclub";
pub const CREW_BLOG: &str = "blog.hackersfight.club";
pub const THANKS: [&str; 3] = ["Lorne", "Ookami", "Pablocro"];

pub const HFC: [&str; 6] = [
    "██╗  ██╗███████╗ ██████╗",
    "██║  ██║██╔════╝██╔════╝",
    "███████║█████╗  ██║     ",
    "██╔══██║██╔══╝  ██║     ",
    "██║  ██║██║     ╚██████╗",
    "╚═╝  ╚═╝╚═╝      ╚═════╝",
];

pub const WORDMARK: [&str; 6] = [
    "██╗  ██╗████████╗██████╗   ███████╗██╗   ██╗██╗████████╗███████╗",
    "██║  ██║╚══██╔══╝██╔══██╗  ██╔════╝██║   ██║██║╚══██╔══╝██╔════╝",
    "███████║   ██║   ██████╔╝  ███████╗██║   ██║██║   ██║   █████╗  ",
    "██╔══██║   ██║   ██╔══██╗  ╚════██║██║   ██║██║   ██║   ██╔══╝  ",
    "██║  ██║   ██║   ██████╔╝  ███████║╚██████╔╝██║   ██║   ███████╗",
    "╚═╝  ╚═╝   ╚═╝   ╚═════╝   ╚══════╝ ╚═════╝ ╚═╝   ╚═╝   ╚══════╝",
];

pub fn thanks_line() -> String {
    THANKS.join(" · ")
}
