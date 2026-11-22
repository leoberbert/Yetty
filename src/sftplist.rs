//! SFTP `ls -l` listings in the layout of the original: no link count, "Sep 22, 2026 20:45" dates, sorted by name.

/// One parsed line of a long listing.
#[derive(Clone, Debug)]
pub struct Entry {
    pub perm: String,
    pub owner: String,
    pub group: String,
    pub size: String,
    pub month: String,
    pub day: u32,
    pub year: u32,
    pub time: Option<String>,
    pub name: String,
}

fn is_perm(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b"-dlcbps".contains(&b[0]) && b[1..].iter().all(|c| b"-rwxsStT".contains(c))
}

/// Could this beginning of a line still turn into a listing line? (A line cut in two by the network.)
pub fn could_be_listing(partial: &str) -> bool {
    let b = partial.as_bytes();
    if b.is_empty() {
        return true;
    }
    if !b"-dlcbps".contains(&b[0]) {
        return false;
    }
    b.iter().skip(1).take(9).all(|c| b"-rwxsStT".contains(c)) && (b.len() <= 10 || b[10] == b' ')
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Parse `-rw-r--r--    ? owner group    size Mon dd HH:MM name` (or `Mon dd  YYYY`). `now` = (year, month).
pub fn parse(line: &str, now_year: u32, now_month: u32) -> Option<Entry> {
    let mut rest = line.trim_end();
    let mut tok: Vec<&str> = Vec::new();
    for _ in 0..8 {
        rest = rest.trim_start();
        let end = rest.find(char::is_whitespace)?;
        tok.push(&rest[..end]);
        rest = &rest[end..];
    }
    let name = rest.trim_start();
    if name.is_empty() || !is_perm(tok[0]) {
        return None;
    }
    if !(tok[1] == "?" || tok[1].chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    if !tok[4].chars().all(|c| c.is_ascii_digit()) || tok[4].is_empty() {
        return None;
    }
    let mi = MONTHS.iter().position(|m| *m == tok[5])?;
    let day: u32 = tok[6].parse().ok()?;
    let (year, time) = if tok[7].contains(':') {
        // recent file: no year printed; it is this year unless that would be in the future
        let y = if (mi as u32 + 1) > now_month {
            now_year.saturating_sub(1)
        } else {
            now_year
        };
        (y, Some(tok[7].to_string()))
    } else {
        (tok[7].parse().ok()?, None)
    };
    Some(Entry {
        perm: tok[0].to_string(),
        owner: tok[2].to_string(),
        group: tok[3].to_string(),
        size: tok[4].to_string(),
        month: tok[5].to_string(),
        day,
        year,
        time,
        name: name.to_string(),
    })
}

/// The lines of a listing, sorted by name (ignoring case) and laid out like the original's.
pub fn format(mut entries: Vec<Entry>) -> Vec<String> {
    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.name.cmp(&b.name))
    });
    let ow = entries
        .iter()
        .map(|e| e.owner.chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    let gw = entries
        .iter()
        .map(|e| e.group.chars().count())
        .max()
        .unwrap_or(0);
    let sw = entries
        .iter()
        .map(|e| e.size.len())
        .max()
        .unwrap_or(0)
        .max(10);
    entries
        .iter()
        .map(|e| {
            let when = match &e.time {
                Some(t) => format!("{} {:02}, {} {}", e.month, e.day, e.year, t),
                None => format!("{} {:02}, {}", e.month, e.day, e.year),
            };
            format!(
                "{} {:<ow$} {:<gw$} {:>sw$} {} {}",
                e.perm,
                e.owner,
                e.group,
                e.size,
                when,
                e.name,
                ow = ow,
                gw = gw,
                sw = sw
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_is_reformatted_and_sorted() {
        let a = parse(
            "-rwxrwxr-x    ? ubuntu   ubuntu        920 Sep 22 20:45 1.sh",
            2026,
            11,
        )
        .unwrap();
        let b = parse(
            "drwxrwxr-x    ? ubuntu   ubuntu         88 Oct 21 12:13 Apps",
            2026,
            11,
        )
        .unwrap();
        let c = parse(
            "-rw-rw-r--    ? ubuntu   ubuntu       2387 Aug 22 01:09 acr-delete.log",
            2026,
            11,
        )
        .unwrap();
        let d = parse(
            "-rw-rw-r--    ? ubuntu   ubuntu      98135 May 19  2026 CONSUMER.zip",
            2026,
            11,
        )
        .unwrap();
        let lines = format(vec![d, b, c, a]);
        assert_eq!(
            lines[0],
            "-rwxrwxr-x ubuntu   ubuntu        920 Sep 22, 2026 20:45 1.sh"
        );
        assert!(lines[1].ends_with("Aug 22, 2026 01:09 acr-delete.log"));
        assert!(lines[2].ends_with("Oct 21, 2026 12:13 Apps"));
        assert!(lines[3].ends_with("May 19, 2026 CONSUMER.zip"));
        assert!(parse("sftp> ls -l", 2026, 11).is_none());
        assert!(parse("Connected to 138.2.244.44.", 2026, 11).is_none());
    }

    #[test]
    fn partial_lines() {
        assert!(could_be_listing("-rw-rw-r--  "));
        assert!(could_be_listing(""));
        assert!(!could_be_listing("sftp> "));
        assert!(!could_be_listing("sftp> ls -l"));
    }
}
