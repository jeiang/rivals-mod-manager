pub type CategoryMatchers = Vec<CategoryMatcher>;

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Plain,
    Regex,
}

impl MatchType {
    pub fn name(&self) -> &'static str {
        match self {
            MatchType::Plain => "Plain",
            MatchType::Regex => "Regex",
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct Matcher {
    value: String,
    matcher_type: MatchType,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct CategoryMatcher {
    name: String,
    matchers: Vec<Matcher>,
}

impl From<(String, Vec<Matcher>)> for CategoryMatcher {
    fn from(value: (String, Vec<Matcher>)) -> Self {
        Self { name: value.0, matchers: value.1 }
    }
}

impl From<(&str, Vec<Matcher>)> for CategoryMatcher {
    fn from(value: (&str, Vec<Matcher>)) -> Self {
        Self { name: value.0.to_string(), matchers: value.1 }
    }
}

impl CategoryMatcher {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn matchers(&self) -> &[Matcher] {
        &self.matchers
    }

    pub fn set_matchers(&mut self, matchers: Vec<Matcher>) {
        self.matchers = matchers;
    }
}

impl PartialOrd for CategoryMatcher {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Eq for CategoryMatcher {}

impl Ord for CategoryMatcher {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Matcher {
    pub fn new(value: String, matcher_type: MatchType) -> Self {
        Self { value, matcher_type }
    }

    pub fn matcher_type(&self) -> &MatchType {
        &self.matcher_type
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut String {
        &mut self.value
    }

    pub fn matcher_type_mut(&mut self) -> &mut MatchType {
        &mut self.matcher_type
    }
}

pub fn default_matchers() -> CategoryMatchers {
    use MatchType::*;
    vec![
        ("Adam Warlock", vec![Matcher::new("adam( warlock)?".into(), Regex)]),
        ("Angela", vec![Matcher::new("angela".into(), Plain)]),
        (
            "Black Panther",
            vec![
                Matcher::new("black panther".into(), Plain),
                Matcher::new("panther".into(), Plain),
                Matcher::new("bp".into(), Plain),
            ],
        ),
        (
            "Black Widow",
            vec![Matcher::new("black widow".into(), Plain), Matcher::new("widow".into(), Plain)],
        ),
        ("Blade", vec![Matcher::new("blade".into(), Plain)]),
        (
            "Hulk",
            vec![Matcher::new("bruce( banner)?".into(), Regex), Matcher::new("hulk".into(), Plain)],
        ),
        ("Captain America", vec![Matcher::new("cap(tain)?( america)?".into(), Regex)]),
        (
            "Cloak & Dagger",
            vec![
                Matcher::new("cloak( ?(&|and) ?dagger)?".into(), Regex),
                Matcher::new("dagger".into(), Plain),
            ],
        ),
        (
            "Daredevil",
            vec![Matcher::new("daredevil".into(), Plain), Matcher::new("dd".into(), Plain)],
        ),
        (
            "Deadpool",
            vec![
                Matcher::new("deadpool".into(), Plain),
                Matcher::new("pool".into(), Plain),
                Matcher::new("dp".into(), Plain),
            ],
        ),
        (
            "Doctor Strange",
            vec![
                Matcher::new("(doc(tor)?|dr)( strange)?".into(), Regex),
                Matcher::new("strange".into(), Plain),
            ],
        ),
        ("Elsa Bloodstone", vec![Matcher::new("elsa( bloodstone)?".into(), Regex)]),
        (
            "Emma Frost",
            vec![Matcher::new("emma( frost)?".into(), Regex), Matcher::new("frost".into(), Plain)],
        ),
        ("Gambit", vec![Matcher::new("gambit".into(), Plain)]),
        ("Groot", vec![Matcher::new("groot".into(), Plain)]),
        (
            "Hawkeye",
            vec![Matcher::new("hawkeye".into(), Plain), Matcher::new("hawk eye".into(), Plain)],
        ),
        ("Hela", vec![Matcher::new("hela".into(), Plain)]),
        (
            "Human Torch",
            vec![
                Matcher::new("(human )?torch".into(), Regex),
                Matcher::new("johnny( storm)?".into(), Regex),
            ],
        ),
        (
            "Invisible Woman",
            vec![
                Matcher::new("(sue|susan)( storm)?".into(), Regex),
                Matcher::new("invis(ible)?( woman)?".into(), Regex),
            ],
        ),
        (
            "Iron Fist",
            vec![
                Matcher::new("iron fist".into(), Plain),
                Matcher::new("fist".into(), Plain),
                Matcher::new("danny( rand)?".into(), Regex),
            ],
        ),
        (
            "Iron Man",
            vec![
                Matcher::new("iron man".into(), Plain),
                Matcher::new("tony( stark)?".into(), Regex),
            ],
        ),
        (
            "Jeff The Land Shark",
            vec![
                Matcher::new("jeff( the land shark)?".into(), Regex),
                Matcher::new("land shark".into(), Plain),
            ],
        ),
        ("Loki", vec![Matcher::new("loki".into(), Plain)]),
        ("Luna Snow", vec![Matcher::new("luna( snow)?".into(), Regex)]),
        ("Magik", vec![Matcher::new("magik".into(), Plain)]),
        ("Magneto", vec![Matcher::new("magneto".into(), Plain), Matcher::new("mag".into(), Plain)]),
        ("Mantis", vec![Matcher::new("mantis".into(), Plain)]),
        (
            "Mister Fantastic",
            vec![
                Matcher::new("m(r|is|ister)\\.?( fantastic)?".into(), Regex),
                Matcher::new("reed( richards)?".into(), Regex),
            ],
        ),
        (
            "Moon Knight",
            vec![
                Matcher::new("moon knight".into(), Plain),
                Matcher::new("mk".into(), Plain),
                Matcher::new("marc( spector)?".into(), Regex),
            ],
        ),
        ("Namor", vec![Matcher::new("namor".into(), Plain)]),
        ("Peni Parker", vec![Matcher::new("peni( parker)?".into(), Regex)]),
        (
            "Phoenix",
            vec![Matcher::new("phoenix".into(), Plain), Matcher::new("jean( grey)?".into(), Regex)],
        ),
        (
            "Psylocke",
            vec![Matcher::new("psylocke".into(), Plain), Matcher::new("psy".into(), Plain)],
        ),
        ("Rocket Raccoon", vec![Matcher::new("rocket( raccoon)?".into(), Regex)]),
        ("Rogue", vec![Matcher::new("rogue".into(), Plain)]),
        (
            "Scarlet Witch",
            vec![
                Matcher::new("scarlet witch".into(), Plain),
                Matcher::new("wanda( maximoff)?".into(), Regex),
                Matcher::new("sw".into(), Plain),
            ],
        ),
        (
            "Spider-man",
            vec![
                Matcher::new("spider-?man".into(), Regex),
                Matcher::new("spidey".into(), Plain),
                Matcher::new("peter( parker)?".into(), Regex),
            ],
        ),
        (
            "Squirrel Girl",
            vec![Matcher::new("squirrel girl".into(), Plain), Matcher::new("sg".into(), Plain)],
        ),
        (
            "Star-lord",
            vec![
                Matcher::new("star-?lord".into(), Regex),
                Matcher::new("peter( quill)?".into(), Regex),
            ],
        ),
        (
            "Storm",
            vec![
                Matcher::new("storm".into(), Plain),
                Matcher::new("ororo".into(), Plain),
                Matcher::new("munroe".into(), Plain),
            ],
        ),
        (
            "The Punisher",
            vec![
                Matcher::new("(the )?punisher".into(), Regex),
                Matcher::new("frank( castle)?".into(), Regex),
            ],
        ),
        (
            "The Thing",
            vec![
                Matcher::new("(the )?thing".into(), Regex),
                Matcher::new("ben( grimm)?".into(), Regex),
            ],
        ),
        ("Thor", vec![Matcher::new("thor".into(), Plain)]),
        ("Ultron", vec![Matcher::new("ultron".into(), Plain)]),
        ("Venom", vec![Matcher::new("venom".into(), Plain)]),
        (
            "White Fox",
            vec![Matcher::new("white fox".into(), Plain), Matcher::new("ami han".into(), Plain)],
        ),
        (
            "Winter Soldier",
            vec![Matcher::new("winter soldier".into(), Plain), Matcher::new("bucky".into(), Plain)],
        ),
        (
            "Wolverine",
            vec![Matcher::new("wolverine".into(), Plain), Matcher::new("logan".into(), Plain)],
        ),
    ]
    .into_iter()
    .map(|e| e.into())
    .collect()
}
