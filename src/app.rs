#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CttApp {
    search_query: String,
    counter: i32,
    status: String,
}

impl Default for CttApp {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            counter: 0,
            status: "Ready".to_string(),
        }
    }
}

#[allow(dead_code)]
impl CttApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn counter(&self) -> i32 {
        self.counter
    }

    pub fn increment(&mut self) -> i32 {
        self.counter += 1;
        self.counter
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_increment() {
        let mut app = CttApp::new();
        assert_eq!(app.counter(), 0);
        assert_eq!(app.increment(), 1);
        assert_eq!(app.increment(), 2);
    }

    #[test]
    fn test_search_query() {
        let mut app = CttApp::new();
        assert_eq!(app.search_query(), "");
        app.set_search_query("macro_1".to_string());
        assert_eq!(app.search_query(), "macro_1");
    }
}
