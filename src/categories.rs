use lazy_regex::{Lazy, regex};
pub type CategoryMatchers = Vec<CategoryMatcher>;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct CategoryMatcher {
    pub(crate) name: String,
    // #[serde(with = "serde_regex")]
    pub(crate) matchers: Vec<regex_helper::RegexProxy>,
}

impl CategoryMatcher {
    pub fn new(name: String, matchers: Vec<regex_helper::RegexProxy>) -> Self {
        Self { name, matchers }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn matchers(&self) -> &[regex_helper::RegexProxy] {
        &self.matchers
    }

    pub fn set_matchers(&mut self, matchers: Vec<regex_helper::RegexProxy>) {
        self.matchers = matchers;
    }
}

pub fn default_matchers() -> CategoryMatchers {
    let matchers: CategoryMatchers = [
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
        ("Magik", [regex!("magik|darkchylde"i)]),
        ("Magneto", [regex!("mag(neto)?"i)]),
        ("Mantis", [regex!("mantis"i)]),
        ("Mister Fantastic", [regex!("((mister|mr\\.?) ?fantastic)|reed"i)]),
        ("Moon Knight", [regex!("(moon knight)|mk"i)]),
        ("Namor", [regex!("namor"i)]),
        ("Peni Parker", [regex!("peni"i)]),
        ("Phoenix", [regex!("phoenix|jean"i)]),
        ("Psylocke", [regex!("psy(locke)?"i)]),
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
        matchers: matchers.into_iter().map(|x| Lazy::force(x).clone().into()).collect(),
    })
    .collect();

    matchers
}

pub fn match_category(categories: &[CategoryMatcher], name: &str) -> String {
    categories
        .iter()
        .find(|c| c.matchers.iter().any(|m| m.is_match(name)))
        .map(|x| x.name.clone())
        .unwrap_or("Uncategorized".into())
}

mod regex_helper {
    use std::ops::{Deref, DerefMut};

    use regex::{Regex, RegexBuilder};
    use serde::{Deserialize, Serialize};

    // serde_regex does not do case-insensitive, so we use this wrapper instead
    #[derive(Debug, Clone)]
    pub struct RegexProxy(regex::Regex);
    impl Serialize for RegexProxy {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(self.0.as_str())
        }
    }

    impl<'de> Deserialize<'de> for RegexProxy {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            RegexBuilder::new(&s)
                .case_insensitive(true)
                .build()
                .map(|x| Self(x))
                .map_err(serde::de::Error::custom)
        }
    }

    impl Deref for RegexProxy {
        type Target = Regex;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl DerefMut for RegexProxy {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl From<Regex> for RegexProxy {
        fn from(value: Regex) -> Self {
            Self(value)
        }
    }
}
