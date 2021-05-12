use crate::util::Time;
use std::collections::HashMap;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Cap {
    pub id: String, // id of the parent
    pub start: Time,
    pub end: Option<Time>,
}

pub struct CapsBuilder {
    value: HashMap<String, Cap>,
}
impl CapsBuilder {
    pub fn new() -> CapsBuilder {
        CapsBuilder {
            value: HashMap::new(),
        }
    }

    pub fn add<T>(mut self, other: T) -> CapsBuilder
    where
        T: Iterator<Item = Cap>,
    {
        for cap in other {
            self.value.insert(cap.id.clone(), cap);
        }

        self
    }

    pub fn add_some<T>(self, other: Option<T>) -> CapsBuilder
    where
        T: Iterator<Item = Cap>,
    {
        if let Some(other) = other {
            self.add(other)
        } else {
            self
        }
    }

    pub fn done(self) -> Vec<Cap> {
        self.value.into_iter().map(|e| e.1).collect()
    }
}
