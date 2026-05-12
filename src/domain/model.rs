#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortUrl {
    pub id: i64,
    pub short_code: String,
    pub long_url: String,
}
