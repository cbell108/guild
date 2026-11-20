//! Static demo content: personas, names, crews, and event templates.

/// An interest cluster that drives a seeded student's tags and behaviour.
pub(crate) struct Persona {
    /// Interest tags (normalized form).
    pub tags: &'static [&'static str],
    /// Bio lines to pick from.
    pub bios: &'static [&'static str],
}

pub(crate) const PERSONAS: &[Persona] = &[
    Persona {
        // gamer
        tags: &["gaming", "smash-bros", "esports", "nintendo", "mario-kart"],
        bios: &[
            "Main Kirby, no shame.",
            "Always down for one more set.",
            "Bringing my Switch everywhere.",
        ],
    },
    Persona {
        // board-gamer
        tags: &["board-games", "tabletop", "dnd", "card-games", "puzzles"],
        bios: &[
            "DM for a Tuesday campaign.",
            "Catan strategist, Codenames enjoyer.",
            "Will teach you any board game.",
        ],
    },
    Persona {
        // coder
        tags: &["coding", "ai", "hackathons", "robotics", "cybersecurity"],
        bios: &[
            "CS junior, likes shipping things at 3am.",
            "Building a robot that makes coffee.",
            "CTF player, ask me about pwn.",
        ],
    },
    Persona {
        // athlete
        tags: &[
            "basketball",
            "soccer",
            "fitness",
            "intramurals",
            "volleyball",
        ],
        bios: &[
            "Pickup basketball most evenings.",
            "Intramural soccer captain.",
            "Rec center regular.",
        ],
    },
    Persona {
        // artist
        tags: &["art", "drawing", "photography", "film", "design"],
        bios: &[
            "Sketchbook always in my bag.",
            "Film photography nerd.",
            "Graphic design major.",
        ],
    },
    Persona {
        // musician
        tags: &["music", "concerts", "jazz", "open-mic", "guitar"],
        bios: &[
            "Plays bass in two bands.",
            "Open mic regular.",
            "Jazz ensemble trumpet.",
        ],
    },
    Persona {
        // foodie
        tags: &["food", "cooking", "coffee", "baking", "free-food"],
        bios: &[
            "Will go anywhere for free pizza.",
            "Trying every coffee spot near campus.",
            "Weekend baker.",
        ],
    },
    Persona {
        // outdoors
        tags: &["hiking", "outdoors", "climbing", "running", "nature"],
        bios: &[
            "Kennesaw Mountain every Sunday.",
            "Learning to boulder.",
            "Training for a half marathon.",
        ],
    },
    Persona {
        // career
        tags: &[
            "career",
            "networking",
            "resume",
            "internships",
            "leadership",
        ],
        bios: &[
            "Looking for a summer internship.",
            "Student government senator.",
            "Business major, coffee chats welcome.",
        ],
    },
    Persona {
        // anime
        tags: &["anime", "manga", "cosplay", "k-pop", "japanese"],
        bios: &[
            "Currently rewatching Frieren.",
            "Cosplay in progress.",
            "Manga library is open to borrow.",
        ],
    },
];

/// Interests no seeded persona has, but that real users can pick and that ingest maps KSU
/// categories to (talks, workshops, sports, research, theater). Kept out of the personas so the
/// seeded world, and therefore `docs/EVAL.md`, does not change.
pub(crate) const EXTRA_INTERESTS: &[&str] =
    &["sports", "research", "theater", "talks", "workshops"];

pub(crate) const FIRST_NAMES: &[&str] = &[
    "Jordan", "Maya", "Ethan", "Priya", "Marcus", "Sofia", "Liam", "Aaliyah", "Noah", "Chloe",
    "Diego", "Hannah", "Jamal", "Grace", "Kenji", "Olivia", "Mateo", "Zoe", "Andre", "Lily",
    "Tyler", "Fatima", "Caleb", "Isabel", "Darius", "Emma", "Ravi", "Naomi", "Owen", "Jasmine",
    "Lucas", "Ava", "Malik", "Elena", "Sean", "Mia", "Hassan", "Riley", "Gabriel", "Nora", "Isaac",
    "Leah", "Tariq", "Ruby", "Evan", "Camila", "Brandon", "Aisha", "Wesley", "Ivy", "Xavier",
    "Sara", "Kofi", "Megan", "Julian", "Tessa", "Omar", "Paige", "Cole", "Yuki",
];

pub(crate) const LAST_INITIALS: &str = "ABCDEFGHJKLMNPRSTVWY";

/// (name, description, persona index).
pub(crate) const CREWS: &[(&str, &str, usize)] = &[
    ("Smash Saturdays", "Weekly Smash Bros in the game room.", 0),
    (
        "Tabletop Tuesdays",
        "Board games and a long-running D&D table.",
        1,
    ),
    (
        "Owl Hacks",
        "Build nights, hackathon teams, AI side projects.",
        2,
    ),
    (
        "Pickup Hoopers",
        "Evening pickup basketball at the rec center.",
        3,
    ),
    ("Sketch Club", "Draw together, critique gently.", 4),
    ("Basement Band", "Jam sessions and open mic prep.", 5),
    ("Snack Squad", "Free food radar and cooking nights.", 6),
    ("Trail Owls", "Hikes, runs, and climbing trips.", 7),
    ("Coffee Chats", "Career prep, resumes, and networking.", 8),
    ("Anime Night", "Watch parties and cosplay builds.", 9),
];

/// A weekly or one-off event shape.
pub(crate) struct Template {
    pub title: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
    pub venue: &'static str,
    pub room: Option<&'static str>,
    /// Persona this mostly appeals to (index into [`PERSONAS`]); also picks the crew for hangouts.
    pub persona: usize,
    /// Official (KSU) or student hangout.
    pub official: bool,
    /// Day offset from the week's Monday (0..=6).
    pub weekday: u32,
    /// Local start hour.
    pub hour: u32,
    /// Length in hours.
    pub hours: i64,
    /// Capacity for capped hangouts.
    pub capacity: Option<u32>,
}

#[allow(clippy::too_many_arguments)] // table constructor; positional rows read best
const fn t(
    title: &'static str,
    description: &'static str,
    tags: &'static [&'static str],
    venue: (&'static str, Option<&'static str>),
    persona: usize,
    official: bool,
    when: (u32, u32, i64),
    capacity: Option<u32>,
) -> Template {
    Template {
        title,
        description,
        tags,
        venue: venue.0,
        room: venue.1,
        persona,
        official,
        weekday: when.0,
        hour: when.1,
        hours: when.2,
        capacity,
    }
}

const SC: &str = "Carmichael Student Center";
const REC: &str = "Recreation & Wellness Center";
const LIB: &str = "Sturgis Library";

/// Happen every week.
pub(crate) const WEEKLY: &[Template] = &[
    t(
        "Smash Saturdays",
        "Bring a controller. Bracket at 8, friendlies all night.",
        &["smash-bros", "gaming", "nintendo"],
        (SC, Some("Game Room 274")),
        0,
        false,
        (5, 19, 3),
        Some(16),
    ),
    t(
        "Mario Kart Monday",
        "Casual races, chaos items on.",
        &["mario-kart", "gaming", "nintendo"],
        (SC, Some("Game Room 274")),
        0,
        false,
        (0, 20, 2),
        Some(12),
    ),
    t(
        "Tabletop Tuesday",
        "Catan, Codenames, Wingspan. New players welcome.",
        &["board-games", "tabletop", "card-games"],
        (LIB, Some("Room 240")),
        1,
        false,
        (1, 18, 3),
        Some(20),
    ),
    t(
        "D&D One-Shot Night",
        "Pre-made characters provided. Dice optional.",
        &["dnd", "tabletop"],
        (LIB, Some("Room 240")),
        1,
        false,
        (3, 19, 3),
        Some(6),
    ),
    t(
        "Owl Hacks Build Night",
        "Bring a project, get unstuck, eat snacks.",
        &["coding", "hackathons", "ai"],
        ("Atrium Building", Some("Lab 120")),
        2,
        false,
        (2, 18, 3),
        None,
    ),
    t(
        "Pickup Basketball",
        "Run fives on court 2.",
        &["basketball", "fitness"],
        (REC, Some("Court 2")),
        3,
        false,
        (1, 19, 2),
        Some(15),
    ),
    t(
        "Intramural Soccer",
        "Rec league games, all skill levels.",
        &["soccer", "intramurals"],
        ("Campus Green", None),
        3,
        true,
        (3, 17, 2),
        None,
    ),
    t(
        "Figure Drawing Session",
        "Untimed and timed poses; bring paper.",
        &["drawing", "art"],
        ("Visual Arts Building", Some("Studio 201")),
        4,
        false,
        (2, 17, 2),
        Some(14),
    ),
    t(
        "Open Mic at The Commons",
        "Sign up at the door. Five minutes each.",
        &["open-mic", "music"],
        ("The Commons", None),
        5,
        true,
        (4, 19, 2),
        None,
    ),
    t(
        "Free Pancake Friday",
        "Free pancakes while they last.",
        &["free-food", "food"],
        (SC, Some("Lobby")),
        6,
        true,
        (4, 9, 2),
        None,
    ),
    t(
        "Saturday Trail Run",
        "5 miles on the Kennesaw Mountain trails.",
        &["running", "hiking", "outdoors"],
        ("Kennesaw Mountain", None),
        7,
        false,
        (5, 8, 2),
        None,
    ),
    t(
        "Anime Watch Party",
        "Voted-on double feature with snacks.",
        &["anime", "manga"],
        (SC, Some("Room 300")),
        9,
        false,
        (4, 20, 3),
        Some(30),
    ),
];

/// A few of these are picked each week.
pub(crate) const ONE_OFF: &[Template] = &[
    t(
        "Valorant Scrims",
        "5v5 customs; sign up as a team or solo.",
        &["esports", "gaming"],
        (SC, Some("Esports Arena")),
        0,
        false,
        (3, 20, 3),
        Some(10),
    ),
    t(
        "KSU Esports Showmatch",
        "Varsity esports exhibition match.",
        &["esports", "gaming"],
        (SC, Some("Esports Arena")),
        0,
        true,
        (2, 19, 2),
        None,
    ),
    t(
        "Puzzle Hunt",
        "Teams of four solve puzzles across campus.",
        &["puzzles", "board-games"],
        (LIB, None),
        1,
        false,
        (5, 14, 3),
        Some(24),
    ),
    t(
        "Magic: The Gathering Draft",
        "Draft night; packs provided.",
        &["card-games", "tabletop"],
        (LIB, Some("Room 240")),
        1,
        false,
        (2, 19, 3),
        Some(8),
    ),
    t(
        "AI Club Workshop: Local LLMs",
        "Run a language model on your own laptop.",
        &["ai", "coding"],
        ("Atrium Building", Some("Lab 120")),
        2,
        true,
        (1, 17, 2),
        None,
    ),
    t(
        "Cybersecurity CTF Practice",
        "Beginner-friendly capture the flag.",
        &["cybersecurity", "coding"],
        ("Atrium Building", Some("Lab 120")),
        2,
        false,
        (3, 18, 3),
        Some(20),
    ),
    t(
        "Robotics Open Lab",
        "Drop in and help build the competition bot.",
        &["robotics", "coding"],
        ("Engineering Lab", None),
        2,
        true,
        (5, 13, 3),
        None,
    ),
    t(
        "Volleyball Pickup",
        "Sand courts, all levels.",
        &["volleyball", "fitness"],
        (REC, Some("Sand Courts")),
        3,
        false,
        (6, 16, 2),
        Some(12),
    ),
    t(
        "Rec Center HIIT Class",
        "45 minutes, bring water.",
        &["fitness"],
        (REC, Some("Studio B")),
        3,
        true,
        (0, 17, 1),
        None,
    ),
    t(
        "Photo Walk",
        "Golden hour walk around campus.",
        &["photography", "art"],
        ("Campus Green", None),
        4,
        false,
        (6, 17, 2),
        None,
    ),
    t(
        "Student Film Screening",
        "Shorts from the film program.",
        &["film", "art"],
        ("Bailey Performance Center", None),
        4,
        true,
        (3, 19, 2),
        None,
    ),
    t(
        "Design Critique Night",
        "Bring a piece, get feedback.",
        &["design", "art"],
        ("Visual Arts Building", Some("Studio 201")),
        4,
        false,
        (1, 18, 2),
        Some(15),
    ),
    t(
        "Jazz Ensemble Concert",
        "Fall concert in Morgan Concert Hall.",
        &["jazz", "concerts", "music"],
        ("Bailey Performance Center", Some("Morgan Concert Hall")),
        5,
        true,
        (4, 20, 2),
        None,
    ),
    t(
        "Guitar Jam Circle",
        "Acoustic jam, any level.",
        &["guitar", "music"],
        ("The Commons", None),
        5,
        false,
        (6, 15, 2),
        None,
    ),
    t(
        "Cooking Night: Dumplings",
        "We fold, we steam, we eat.",
        &["cooking", "food"],
        ("Prillaman Hall", Some("Kitchen 105")),
        6,
        false,
        (2, 18, 2),
        Some(10),
    ),
    t(
        "Coffee Crawl",
        "Three coffee shops in one afternoon.",
        &["coffee", "food"],
        ("Downtown Kennesaw", None),
        6,
        false,
        (6, 13, 3),
        Some(8),
    ),
    t(
        "Bake Sale for Charity",
        "Cookies, brownies, good cause.",
        &["baking", "food"],
        (SC, Some("Lobby")),
        6,
        true,
        (2, 11, 3),
        None,
    ),
    t(
        "Climbing Gym Trip",
        "Carpool to the bouldering gym.",
        &["climbing", "outdoors"],
        (REC, Some("Climbing Wall")),
        7,
        false,
        (3, 18, 2),
        Some(10),
    ),
    t(
        "Sunrise Hike",
        "Summit Kennesaw Mountain for sunrise.",
        &["hiking", "nature", "outdoors"],
        ("Kennesaw Mountain", None),
        7,
        false,
        (6, 6, 3),
        None,
    ),
    t(
        "Fall Career Fair",
        "Employers hiring for internships and full-time roles.",
        &["career", "internships", "networking"],
        ("Convocation Center", None),
        8,
        true,
        (2, 10, 4),
        None,
    ),
    t(
        "Resume Review Drop-In",
        "Career services reviews your resume in 10 minutes.",
        &["resume", "career"],
        ("Career Services Center", None),
        8,
        true,
        (1, 13, 3),
        None,
    ),
    t(
        "Leadership Summit",
        "Student org leaders panel and workshop.",
        &["leadership", "networking"],
        (SC, Some("Ballroom")),
        8,
        true,
        (5, 10, 4),
        None,
    ),
    t(
        "Cosplay Workshop",
        "Foam armor basics.",
        &["cosplay", "anime"],
        ("Visual Arts Building", Some("Studio 105")),
        9,
        false,
        (6, 14, 3),
        Some(12),
    ),
    t(
        "K-Pop Dance Practice",
        "Learn a routine in an hour.",
        &["k-pop", "music"],
        (REC, Some("Studio A")),
        9,
        false,
        (2, 20, 1),
        Some(20),
    ),
    t(
        "Japanese Conversation Table",
        "Practice with native speakers.",
        &["japanese", "anime"],
        (LIB, Some("Room 110")),
        9,
        true,
        (3, 12, 1),
        None,
    ),
    t(
        "Homecoming Tailgate",
        "Food trucks and games before kickoff.",
        &["free-food", "food", "music"],
        ("Fifth Third Stadium", None),
        6,
        true,
        (5, 15, 4),
        None,
    ),
];
