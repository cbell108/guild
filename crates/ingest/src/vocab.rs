//! Maps KSU calendar categories to the interest vocabulary the app's users pick from
//! (`seed::interest_vocabulary`), so official events line up with what students say they like.
//!
//! KSU's own tags ("lectures-guest-speakers", "athletics", department names) rarely equal
//! interest tags ("talks", "sports", "coding"). Each [`Rule`] adds one interest tag when any of
//! its signals matches: a normalized KSU tag / event type, a substring of the department name,
//! or a whole word of the title. The original KSU tags are kept alongside.

/// One mapping from KSU signals to an interest tag.
struct Rule {
    /// The interest tag to add.
    interest: &'static str,
    /// Normalized KSU tags / event types / keywords that imply it (exact match).
    ksu_tags: &'static [&'static str],
    /// Lowercase substrings of the organizing department's name.
    departments: &'static [&'static str],
    /// Lowercase whole words of the title.
    title_words: &'static [&'static str],
}

const fn rule(
    interest: &'static str,
    ksu_tags: &'static [&'static str],
    departments: &'static [&'static str],
    title_words: &'static [&'static str],
) -> Rule {
    Rule {
        interest,
        ksu_tags,
        departments,
        title_words,
    }
}

const RULES: &[Rule] = &[
    rule(
        "sports",
        &["athletics"],
        &["athletics"],
        &[
            "basketball",
            "soccer",
            "volleyball",
            "football",
            "baseball",
            "softball",
            "tennis",
            "golf",
            "swim",
            "lacrosse",
        ],
    ),
    rule("basketball", &[], &[], &["basketball"]),
    rule("soccer", &[], &[], &["soccer"]),
    rule("volleyball", &[], &[], &["volleyball"]),
    rule(
        "fitness",
        &["recreation", "wellness"],
        &["recreation", "well-being", "wellbeing"],
        &["fitness", "yoga", "workout", "zumba", "wellness"],
    ),
    rule("running", &[], &[], &["5k", "run", "marathon"]),
    rule(
        "music",
        &[],
        &["music"],
        &[
            "concert",
            "recital",
            "orchestra",
            "philharmonic",
            "ensemble",
            "choir",
            "symphony",
            "band",
            "choral",
            "music",
        ],
    ),
    rule(
        "concerts",
        &[],
        &[],
        &[
            "concert",
            "recital",
            "orchestra",
            "philharmonic",
            "symphony",
        ],
    ),
    rule("jazz", &[], &[], &["jazz"]),
    rule(
        "art",
        &[],
        &["arts"],
        &[
            "art",
            "exhibit",
            "exhibition",
            "gallery",
            "artist",
            "painting",
            "sculpture",
        ],
    ),
    rule("design", &[], &[], &["design", "designer"]),
    rule("photography", &[], &[], &["photography", "photo"]),
    rule(
        "theater",
        &[],
        &[],
        &["theatre", "theater", "musical", "dance"],
    ),
    rule("film", &[], &[], &["film", "movie", "screening", "cinema"]),
    rule(
        "career",
        &["career-services"],
        &["career"],
        &[
            "career",
            "careers",
            "job",
            "jobs",
            "internship",
            "internships",
            "employer",
            "employers",
            "resume",
        ],
    ),
    rule("internships", &[], &[], &["internship", "internships"]),
    rule("resume", &[], &[], &["resume", "resumes"]),
    rule(
        "networking",
        &["conferences-fairs"],
        &[],
        &["networking", "fair", "mixer"],
    ),
    rule("leadership", &[], &[], &["leadership", "sga"]),
    rule(
        "coding",
        &[],
        &[
            "computing",
            "software",
            "data science",
            "information technology",
            "engineering",
        ],
        &[
            "coding",
            "programming",
            "python",
            "software",
            "developer",
            "java",
        ],
    ),
    rule("ai", &[], &[], &["ai", "artificial", "analytics"]),
    rule("cybersecurity", &[], &[], &["cyber", "cybersecurity"]),
    rule("robotics", &[], &[], &["robot", "robots", "robotics"]),
    rule("hackathons", &[], &[], &["hackathon", "hackathons"]),
    rule(
        "research",
        &["research", "undergraduate-research", "undergrad-research"],
        &["research"],
        &["research", "dissertation", "thesis"],
    ),
    rule(
        "talks",
        &["lectures-guest-speakers"],
        &[],
        &[
            "lecture",
            "speaker",
            "keynote",
            "panel",
            "colloquium",
            "seminar",
            "talk",
        ],
    ),
    rule(
        "workshops",
        &["training-workshops", "training"],
        &[],
        &["workshop", "workshops", "training"],
    ),
    rule(
        "food",
        &[],
        &[],
        &[
            "lunch",
            "dinner",
            "pizza",
            "cookout",
            "tasting",
            "breakfast",
            "food",
            "bake",
        ],
    ),
    rule("free-food", &[], &[], &["pizza", "refreshments", "snacks"]),
    rule("coffee", &[], &[], &["coffee", "latte", "tea"]),
    rule(
        "outdoors",
        &[],
        &[],
        &[
            "hike", "hiking", "trail", "outdoor", "outdoors", "garden", "nature",
        ],
    ),
    rule(
        "gaming",
        &[],
        &[],
        &["gaming", "esports", "smash", "mario", "videogame"],
    ),
    rule("board-games", &[], &[], &["tabletop", "dnd", "catan"]),
    rule(
        "anime",
        &[],
        &[],
        &["anime", "manga", "cosplay", "k-pop", "kpop"],
    ),
];

/// The interest tags implied by an event's KSU tags (already normalized), department and
/// title, in rule order and without duplicates.
pub fn interest_tags(
    ksu_tags: &[String],
    department: Option<&str>,
    title: &str,
) -> Vec<&'static str> {
    let department = department.map(str::to_lowercase).unwrap_or_default();
    let title = title.to_lowercase();
    let words: Vec<&str> = title
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    RULES
        .iter()
        .filter(|r| {
            r.ksu_tags.iter().any(|t| ksu_tags.iter().any(|k| k == t))
                || (!department.is_empty() && r.departments.iter().any(|d| department.contains(d)))
                || r.title_words.iter().any(|w| words.contains(w))
        })
        .map(|r| r.interest)
        .collect()
}

/// Every interest tag the rules can produce (for checking against the seeded vocabulary).
pub fn mapped_interests() -> impl Iterator<Item = &'static str> {
    RULES.iter().map(|r| r.interest)
}
