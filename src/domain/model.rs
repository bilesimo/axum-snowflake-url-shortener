#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortUrl {
    pub id: i64,
    pub short_code: String,
    pub long_url: String,
    pub normalized_long_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewShortUrl {
    pub id: i64,
    pub short_code: String,
    pub long_url: String,
    pub normalized_long_url: String,
}
