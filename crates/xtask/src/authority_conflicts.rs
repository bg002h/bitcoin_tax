//! **The statute/regulation conflict register, enforced.**
//!
//! Only **26 USC** is law. A Treasury regulation is the executive's *interpretation* of it — binding in
//! practice, capable of being wrong, and regularly held invalid for exceeding or contradicting the
//! statute. So if we believe the statute disagrees with a regulation, the filer has a **duty to push
//! back**, and the system supplies the instrument (**Form 8275-R**).
//!
//! ★ That duty is routinely neglected **because challenging is expensive**. That is a legitimate choice —
//! but it must be *a choice*, made on a date, by a person, and revisited. This module makes it impossible
//! for it to be an omission nobody ever decided: every entry in `AUTHORITY_CONFLICTS.md` carries a
//! `review-by`, and an overdue entry **fails the test suite**.
//!
//! ★★ **The suite going red on a date is the point, not a bug.** It is the only reminder in this project
//! that cannot be ignored. Extending a date to silence it, without actually revisiting the question, is
//! the one misuse that defeats the whole mechanism.
//!
//! ★ **Scope bound** (same as `FOLLOWUPS.md` §G-11/§G-12): this does **not** identify conflicts. That is
//! legal judgement, out of scope exactly as *intent* is. A human writes the entry; this does bookkeeping.

use std::fs;
use std::path::{Path, PathBuf};

/// Where the filer currently stands on a believed conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Posture {
    /// No decision yet. **Must** carry a future `review-by` — an undecided conflict that nobody revisits
    /// is precisely the neglected duty this register exists to prevent.
    Undecided,
    /// We follow the regulation despite believing the statute differs — usually because challenging
    /// costs more than it is worth. Legitimate, and still re-reviewed: cost/benefit moves.
    ComplyWithReg,
    /// We take the statute's position and disclose it on **Form 8275-R**.
    StatutePositionWith8275R,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub id: String,
    pub summary: String,
    pub statute: String,
    pub regulation: String,
    pub disagreement: String,
    /// ★ Load-bearing, not bookkeeping: a reg that UNDERSTATES tax relative to the statute is the
    /// dangerous one — complying is both the cheap path and the path that risks an understatement on a
    /// return signed under §6065.
    pub direction: String,
    pub posture: Posture,
    pub decided: Option<String>,
    /// `YYYY-MM-DD`. Overdue ⇒ the suite fails.
    pub review_by: String,
    pub why: String,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/xtask has a grandparent")
        .to_path_buf()
}

fn field(block: &str, key: &str) -> Option<String> {
    block.lines().find_map(|l| {
        let l = l.trim();
        let p = format!("- **{key}:**");
        l.strip_prefix(&p).map(|v| v.trim().to_string())
    })
}

/// Parse every `### AC-<n>` entry. A malformed entry is an **error**, never a skip — silently ignoring a
/// block that does not parse would let a real conflict sit unenforced while everything looked green.
pub fn parse(md: &str) -> Result<Vec<Conflict>, String> {
    let mut out = Vec::new();
    for block in md.split("\n### ").skip(1) {
        let head = block.lines().next().unwrap_or_default().trim().to_string();
        if !head.starts_with("AC-") {
            continue; // a prose heading, not an entry
        }
        let (id, summary) = head
            .split_once(" — ")
            .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
            .unwrap_or((head.clone(), String::new()));
        let need =
            |k: &str| field(block, k).ok_or_else(|| format!("{id}: missing required field `{k}`"));
        let posture = match need("posture")?.as_str() {
            "undecided" => Posture::Undecided,
            "comply-with-reg" => Posture::ComplyWithReg,
            "statute-position-with-8275R" => Posture::StatutePositionWith8275R,
            other => return Err(format!("{id}: unknown posture `{other}`")),
        };
        let decided = match need("decided")?.as_str() {
            "—" | "-" | "" => None,
            d => Some(d.to_string()),
        };
        if posture != Posture::Undecided && decided.is_none() {
            return Err(format!(
                "{id}: posture is decided but `decided:` is blank — a decision has a DATE and an author, \
                 or it is not a decision"
            ));
        }
        let (statute, regulation) = (need("statute")?, need("regulation")?);
        let (disagreement, direction) = (need("disagreement")?, need("direction")?);
        let (review_by, why) = (need("review-by")?, need("why")?);
        out.push(Conflict {
            id,
            summary,
            statute,
            regulation,
            disagreement,
            direction,
            posture,
            decided,
            review_by,
            why,
        });
    }
    Ok(out)
}

/// Today as `YYYY-MM-DD`, from the system clock. Deliberately real: this mechanism's whole value is that
/// it fires on a date nobody chose to be reminded on.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs / 86_400;
    // Civil-from-days (Howard Hinnant's algorithm) — no chrono dependency in dev-only tooling.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Entries whose `review-by` has passed. `YYYY-MM-DD` sorts lexicographically, so a string compare is
/// exact here and needs no date library.
pub fn overdue<'a>(conflicts: &'a [Conflict], today: &str) -> Vec<&'a Conflict> {
    conflicts
        .iter()
        .filter(|c| c.review_by.as_str() < today)
        .collect()
}

/// `cargo run -p xtask -- authority-conflicts` — the same check the test runs, as a command.
pub fn run() -> Result<(), String> {
    let path = repo_root().join("AUTHORITY_CONFLICTS.md");
    let md =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let conflicts = parse(&md)?;
    let now = today();
    let late = overdue(&conflicts, &now);
    println!(
        "authority-conflicts: {} entr{} recorded, {} undecided, {} overdue (today {now})",
        conflicts.len(),
        if conflicts.len() == 1 { "y" } else { "ies" },
        conflicts
            .iter()
            .filter(|c| c.posture == Posture::Undecided)
            .count(),
        late.len()
    );
    for c in &conflicts {
        // ★ Print the WHOLE entry, not just an id. Someone running this is deciding whether to do the
        // duty, and the two facts that decide it are the DIRECTION (a reg that UNDERSTATES tax is the
        // dangerous one — complying is both the cheap path and the one that risks an understatement on a
        // signed return) and the WHY (usually: challenging costs more than it is worth, which is a
        // legitimate answer whose cost/benefit moves over time).
        let overdue_mark = if c.review_by.as_str() < now.as_str() {
            "   ★ OVERDUE"
        } else {
            ""
        };
        let decided = c
            .decided
            .as_deref()
            .map(|d| format!(" (decided {d})"))
            .unwrap_or_default();
        println!("\n  {} — {}", c.id, c.summary);
        println!("    statute      {}", c.statute);
        println!("    regulation   {}", c.regulation);
        println!("    disagreement {}", c.disagreement);
        println!("    direction    {}", c.direction);
        println!("    posture      {:?}{decided}", c.posture);
        println!("    review-by    {}{overdue_mark}", c.review_by);
        println!("    why          {}", c.why);
    }
    if late.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{} authority-conflict entr{} PAST review-by: {}.\n\
         Re-decide each and set a new review-by. Do NOT simply push the date out — the whole value of \
         this register is that a duty we chose to neglect stays visible, and silencing it without \
         revisiting the question is the one misuse that defeats it. Only 26 USC is law; a regulation is \
         the agency's interpretation and can be wrong. Form 8275-R is the instrument for saying so.",
        late.len(),
        if late.len() == 1 { "y is" } else { "ies are" },
        late.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "
### AC-9 — a believed conflict, for testing

- **statute:** 26 USC §1
- **regulation:** 26 CFR §1.1-1
- **disagreement:** the reg reads a limit into the statute
- **direction:** the regulation UNDERSTATES tax relative to the statute — it allows a deduction §1 does not
- **posture:** comply-with-reg
- **decided:** 2026-01-01
- **review-by:** 2026-06-30
- **why:** challenging costs more than the amount at stake
";

    /// ★★ **THE MECHANISM.** An entry past its `review-by` must FAIL. Without this the register is a
    /// notes file, and a duty we chose to neglect quietly stops being visible — which is the exact
    /// outcome it exists to prevent.
    #[test]
    fn an_overdue_entry_fails_and_a_current_one_does_not() {
        let c = parse(SAMPLE).expect("sample parses");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].posture, Posture::ComplyWithReg);
        assert_eq!(overdue(&c, "2026-06-29").len(), 0, "not yet due");
        assert_eq!(
            overdue(&c, "2026-06-30").len(),
            0,
            "due today is not overdue"
        );
        assert_eq!(
            overdue(&c, "2026-07-01").len(),
            1,
            "PAST review-by must fire"
        );
    }

    /// A decided posture with no date is not a decision. And an unparseable entry is an error, never a
    /// skip — skipping would let a real conflict sit unenforced behind a green suite.
    #[test]
    fn a_decision_needs_a_date_and_a_malformed_entry_is_an_error() {
        let undated = SAMPLE.replace("- **decided:** 2026-01-01", "- **decided:** —");
        assert!(
            parse(&undated).is_err(),
            "decided posture with no date must be rejected"
        );
        let missing = SAMPLE.replace(
            "- **direction:** the regulation UNDERSTATES",
            "- **dir:** x",
        );
        assert!(
            parse(&missing).is_err(),
            "a missing required field must be an error"
        );
        let bad = SAMPLE.replace(
            "- **posture:** comply-with-reg",
            "- **posture:** probably-fine",
        );
        assert!(parse(&bad).is_err(), "an unknown posture must be rejected");
        // ★ `undecided` is legitimate — it just cannot be left to rot, which `overdue` enforces.
        let un = SAMPLE
            .replace("- **posture:** comply-with-reg", "- **posture:** undecided")
            .replace("- **decided:** 2026-01-01", "- **decided:** —");
        assert_eq!(parse(&un).unwrap()[0].posture, Posture::Undecided);
    }

    /// The committed register itself: parses, and nothing is overdue.
    #[test]
    fn the_committed_register_parses_and_is_current() {
        let md = fs::read_to_string(repo_root().join("AUTHORITY_CONFLICTS.md")).expect(
            "AUTHORITY_CONFLICTS.md must exist — it is a standing register, not an artifact",
        );
        let c = parse(&md).expect("the register must parse");
        let now = today();
        let late = overdue(&c, &now);
        assert!(
            late.is_empty(),
            "overdue: {:?}",
            late.iter().map(|x| &x.id).collect::<Vec<_>>()
        );
        // ★ Guard against a vacuous pass: prove `today()` produces a real date, so an empty register is
        // "nothing recorded" rather than "the clock returned garbage and nothing could ever be overdue".
        assert!(
            now.starts_with("20") && now.len() == 10,
            "today() looks wrong: {now}"
        );
        assert!(
            now.as_str() > "2026-01-01",
            "today() is implausibly early: {now}"
        );
    }
}
