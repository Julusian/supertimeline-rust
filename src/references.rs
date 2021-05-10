use std::collections::HashSet;

pub struct ReferencesBuilder {
    value: HashSet<String>,
}
impl ReferencesBuilder {
    pub fn new() -> ReferencesBuilder {
        ReferencesBuilder {
            value: HashSet::new(),
        }
    }

    pub fn add(mut self, other: &HashSet<String>) -> ReferencesBuilder {
        self.value.extend(other.iter().cloned());
        self
    }

    // pub fn add2(mut self, other: HashSet<String>) -> ReferencesBuilder {
    //     self.value.extend(other.into_iter());
    //     self
    // }

    pub fn add_some(mut self, other: Option<&HashSet<String>>) -> ReferencesBuilder {
        if let Some(other) = other {
            self.value.extend(other.iter().cloned());
        }
        self
    }

    pub fn add_some2(mut self, other: Option<HashSet<String>>) -> ReferencesBuilder {
        if let Some(other) = other {
            self.value.extend(other.into_iter());
        }
        self
    }

    pub fn add_id(mut self, id: &str) -> ReferencesBuilder {
        self.value.insert(id.to_string());
        self
    }

    pub fn done(self) -> HashSet<String> {
        self.value
    }
}
