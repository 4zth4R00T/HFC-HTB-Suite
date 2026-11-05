fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn epoch_of_iso(iso: &str) -> Option<i64> {
    let s = iso.trim();
    if s.len() < 10 {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: u32 = s[5..7].parse().ok()?;
    let d: u32 = s[8..10].parse().ok()?;
    let mut secs = days_from_civil(y, m, d) * 86400;
    if s.len() >= 19 {
        let hh: i64 = s[11..13].parse().unwrap_or(0);
        let mm: i64 = s[14..16].parse().unwrap_or(0);
        let ss: i64 = s[17..19].parse().unwrap_or(0);
        secs += hh * 3600 + mm * 60 + ss;
    }
    Some(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fechas() {
        assert_eq!(epoch_of_iso("1970-01-02"), Some(86400));
        assert_eq!(epoch_of_iso("2026-11-03T00:00:10"), Some(1793664010));
        assert!(epoch_of_iso("x").is_none());
    }
}
