use std::{
    fs::File,
    io::{BufReader, Read},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

pub enum CheckType {
    Json,
    Bson,
}

pub trait JsonChecker {
    /// Check if the Struct passed can be serialized and deserialized using a file
    /// as a reference
    fn check<T: Serialize + DeserializeOwned + PartialEq>(
        &self,
        value: &T,
        r#type: CheckType,
    ) -> bool;

    /// The location where the checker will look for the file. If the folder doesn't exist, it will
    /// be created. It is always in relation to the current directory.
    fn location(&self) -> String {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");

        let json_checks_dir = current_dir.join("tests").join("resource");

        if !json_checks_dir.exists() {
            std::fs::create_dir(&json_checks_dir).expect("Failed to create json_checks directory");
        }

        json_checks_dir
            .to_str()
            .expect("Failed to convert path to string")
            .to_string()
    }
}

#[derive(Debug, Clone, Default)]
pub struct JsonCheckerImpl {
    /// If true, the checker will override the file if it exists
    /// or create a new file if it doesn't exist
    r#override: bool,
}

impl JsonCheckerImpl {
    pub fn r#override(mut self) -> Self {
        self.r#override = true;
        self
    }
}

impl JsonChecker for JsonCheckerImpl {
    fn check<T: Serialize + DeserializeOwned + PartialEq>(
        &self,
        value: &T,
        r#type: CheckType,
    ) -> bool {
        let type_name = std::any::type_name::<T>().to_string();
        let file_path = match r#type {
            CheckType::Json => self.location() + &format!("/{}.json", type_name),
            CheckType::Bson => self.location() + &format!("/{}.bson", type_name),
        };

        match r#type {
            CheckType::Json => {
                let serialized =
                    serde_json::to_string_pretty(value).expect("Failed to serialize value");

                let file = File::open(file_path.clone());

                if self.r#override {
                    std::fs::write(file_path, serialized).expect("Failed to write to file");
                    panic!("Override flag is enabled, remember to disable and commit the changes");
                }

                if file.is_err() {
                    return false;
                }

                let file = file.expect("Failed to open file");
                let mut file = BufReader::new(file);

                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("Failed to read file contents");

                let expected = serde_json::from_str::<Value>(&contents)
                    .expect("Failed to deserialize expect value");

                let actual = serde_json::from_str::<Value>(&serialized)
                    .expect("Failed to deserialize actual value");

                expected == actual
            }
            CheckType::Bson => {
                let serialized = bson::to_vec(value).expect("Failed to serialize value");

                let file = File::open(file_path.clone());

                if self.r#override {
                    std::fs::write(file_path, serialized).expect("Failed to write to file");
                    panic!("Override flag is enabled, remember to disable and commit the changes");
                }

                if file.is_err() {
                    return false;
                }

                let file = file.expect("Failed to open file");
                let mut file = BufReader::new(file);

                let mut contents: Vec<u8> = vec![];
                file.read_to_end(&mut contents)
                    .expect("Failed to read file");

                let expected =
                    bson::from_slice::<T>(&contents).expect("Failed to deserialize expect value");

                let actual =
                    bson::from_slice::<T>(&serialized).expect("Failed to deserialize actual value");

                expected == actual
            }
        }
    }
}
