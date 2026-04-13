use lazy_regex::regex;
use lazy_regex::regex::Regex;
pub type CategoryMatchers = Vec<CategoryMatcher>;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct CategoryMatcher {
    name: String,
    #[serde(with = "serde_regex")]
    matchers: Vec<Regex>,
}

impl CategoryMatcher {
    pub fn new(name: String, matchers: Vec<Regex>) -> Self {
        Self { name, matchers }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn matchers(&self) -> &[Regex] {
        &self.matchers
    }

    pub fn set_matchers(&mut self, matchers: Vec<Regex>) {
        self.matchers = matchers;
    }
}

pub fn default_matchers() -> CategoryMatchers {
    [
        ("Adam Warlock", [regex!("adam|warlock"i)]),
        ("Angela", [regex!("angela"i)]),
        ("Black Panther", [regex!("(black )?panther"i)]),
        ("Black Widow", [regex!("(black )?widow"i)]),
        ("Blade", [regex!("blade"i)]),
        ("Hulk", [regex!("hulk|banner"i)]),
        ("Captain America", [regex!("cap(tain)( america)?"i)]),
        ("Cloak & Dagger", [regex!("cloak|dagger"i)]),
        ("Daredevil", [regex!("daredevil|dd"i)]),
        ("Deadpool", [regex!("deadpool|dp"i)]),
        ("Doctor Strange", [regex!("(doctor|dr|steven) ?strange"i)]),
        ("Elsa Bloodstone", [regex!("elsa"i)]),
        ("Emma Frost", [regex!("emma"i)]),
        ("Gambit", [regex!("gambit"i)]),
        ("Groot", [regex!("groot"i)]),
        ("Hawkeye", [regex!("hawkeye"i)]),
        ("Hela", [regex!("hela"i)]),
        ("Human Torch", [regex!("(human )?torch"i)]),
        ("Invisible Woman", [regex!("(invis(ible)?( woman)?)|((sue|susan) ?(storm)?)"i)]),
        ("Iron Fist", [regex!("(iron )?fist"i)]),
        ("Iron Man", [regex!("(iron man)|tony|stark"i)]),
        ("Jeff The Land Shark", [regex!("jeff|(land shark)"i)]),
        ("Loki", [regex!("loki"i)]),
        ("Luna Snow", [regex!("luna( snow)?"i)]),
        ("Magik", [regex!("magik"i)]),
        ("Magneto", [regex!("mag(neto)?"i)]),
        ("Mantis", [regex!("mantis"i)]),
        ("Mister Fantastic", [regex!("((mister|mr\\.?) ?fantastic)|reed"i)]),
        ("Moon Knight", [regex!("(moon knight)|mk"i)]),
        ("Namor", [regex!("namor"i)]),
        ("Peni Parker", [regex!("peni"i)]),
        ("Phoenix", [regex!("phoenix|jean"i)]),
        ("Psylocke", [regex!("psy(locke)"i)]),
        ("Rocket Raccoon", [regex!("rocket"i)]),
        ("Rogue", [regex!("rogue"i)]),
        ("Scarlet Witch", [regex!("(scarlet witch)|wanda|sw"i)]),
        ("Spider-man", [regex!("spider-?man"i)]),
        ("Squirrel Girl", [regex!("squirrel girl"i)]),
        ("Star-lord", [regex!("(star-?lord)|(quill)"i)]),
        ("Storm", [regex!("storm"i)]),
        ("The Punisher", [regex!("punisher"i)]),
        ("The Thing", [regex!("thing"i)]),
        ("Thor", [regex!("thor"i)]),
        ("Ultron", [regex!("ultron"i)]),
        ("Venom", [regex!("venom"i)]),
        ("White Fox", [regex!("white fox"i)]),
        ("Winter Soldier", [regex!("(winter soldier)|bucky"i)]),
        ("Wolverine", [regex!("wolverine"i)]),
    ]
    .into_iter()
    .map(|(name, matchers)| CategoryMatcher {
        name: name.to_string(),
        // need to get the actual regex out
        matchers: matchers.into_iter().map(|x| (*x).clone()).collect(),
    })
    .collect()
}

pub fn match_category(categories: &[CategoryMatcher], name: &str) -> String {
    categories
        .iter()
        .find(|c| c.matchers.iter().any(|m| m.is_match(name)))
        .map(|x| x.name.clone())
        .unwrap_or("Uncategorized".into())
}
