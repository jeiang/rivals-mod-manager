use regex::Regex;

pub enum MatchType {
    Plain,
    Regex,
}

pub struct Matcher {
    matcher_type: MatchType,
    case_sensitive: bool,
}

pub struct Category {
    name: String,
    matchers: Vec<Matcher>,
}
