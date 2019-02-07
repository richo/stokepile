use time::Duration;

/// Convert an integer number of bytes to something a human being might reasonably intepret. Allows
/// for one place of decimal precision for low quanta of a given denomination.
/// ```rust
/// # use archiver::formatting::human_readable_size;
/// assert_eq!(human_readable_size(12), "12".to_string());
/// assert_eq!(human_readable_size(2055), "2k".to_string());
/// assert_eq!(human_readable_size(36700244), "35m".to_string());
/// assert_eq!(human_readable_size(3650722201), "3.4g".to_string());
/// ```
pub fn human_readable_size(bytes: usize) -> String {
    let mut multiplier = 1;

    if bytes < 1024 {
        return format!("{}", bytes);
    }
    for unit in &['k', 'm', 'g', 't'] {
        multiplier *= 1024;
        if *unit != 'k' && bytes < multiplier * 10 {
            return format!("{:.1}{}", bytes as f32 / multiplier as f32, unit);
        }
        if bytes < 1024 * multiplier {
            return format!("{:.0}{}", bytes as f32 / multiplier as f32, unit);
        }
    }
    return format!("{}t", bytes as f32 / multiplier as f32);
}

/// Format a given `Duration` as a formatted amount of time a human might reasonably interpret.
/// ```rust
/// # use archiver::formatting::human_readable_time;
/// use time::Duration;
///
/// assert_eq!(human_readable_time(Duration::seconds(45)), "45s".to_string());
/// assert_eq!(human_readable_time(Duration::seconds(45311)), "12h35m11s".to_string());
/// ```
pub fn human_readable_time(time: Duration) -> String {
    let mut secs = time.num_seconds();
    let mut out = "".to_string();

    if secs > 60 {
        let mut mins = secs / 60;
        secs %= 60;

        if mins > 60 {
            let hours = mins / 60;
            mins %= 60;

            out = format!("{}h", hours);
        }
        out = format!("{}{}m", out, mins);
    }
    format!("{}{}s", out, secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_flysight_manager_test_suite_for_human_readable_size() {
        assert_eq!(human_readable_size(1000), "1000".to_string());
        assert_eq!(human_readable_size(1024), "1k".to_string());
        // TODO(richo) This would be way better as 1m I think
        assert_eq!(human_readable_size(1024 * 1024), "1.0m".to_string());
        assert_eq!(human_readable_size(1144 * 1024), "1.1m".to_string());
        assert_eq!(human_readable_size((5.5 * 1024f64 * 1024f64) as usize), "5.5m".to_string());
        assert_eq!(human_readable_size(1024 * 100 + 100), "100k".to_string());
        assert_eq!(human_readable_size(5), "5".to_string());
        assert_eq!(human_readable_size(2000), "2k".to_string());
        assert_eq!(human_readable_size(1024 * 1024 * 1024 * 1024 * 1024), "1024t".to_string());
    }
}
