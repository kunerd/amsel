#[derive(Debug, Clone)]
pub struct Video {
    id: String,
}

impl Video {
    pub async fn search(query: String) -> Vec<Self> {
        Vec::new()
    }
}
