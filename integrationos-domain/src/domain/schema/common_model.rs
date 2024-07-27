use crate::{
    api_model_config::Lang,
    id::{prefix::IdPrefix, Id},
    prelude::{shared::record_metadata::RecordMetadata, MongoStore, StringExt},
    IntegrationOSError, InternalError,
};
use async_recursion::async_recursion;
use bson::doc;
use indexmap::IndexMap;
use openapiv3::*;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Deref,
};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct CommonModel {
    #[serde(rename = "_id")]
    pub id: Id,
    pub name: String,
    pub fields: Vec<Field>,
    #[serde(default)]
    pub sample: Value,
    #[serde(default)]
    pub primary: bool,
    pub category: String,
    #[serde(default)]
    pub interface: HashMap<Lang, String>,
    #[serde(flatten, default)]
    pub record_metadata: RecordMetadata,
}

impl Hash for CommonModel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct UnsavedCommonModel {
    pub name: String,
    pub fields: Vec<Field<UnsavedCommonModel>>,
    pub category: String,
    #[serde(default)]
    pub sample: Value,
    #[serde(default)]
    pub interface: HashMap<Lang, String>,
    #[serde(default)]
    pub primary: bool,
}

impl Default for CommonModel {
    fn default() -> Self {
        Self {
            id: Id::new(IdPrefix::CommonModel, chrono::Utc::now()),
            name: Default::default(),
            fields: Default::default(),
            sample: Default::default(),
            primary: Default::default(),
            category: Default::default(),
            interface: Default::default(),
            record_metadata: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq, Copy)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub enum SchemaType {
    Lax,
    Strict,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct Field<T = CommonModel> {
    pub name: String,
    #[serde(flatten)]
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub datatype: DataType<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

impl Field {
    fn is_expandable(&self) -> bool {
        self.datatype.is_expandable()
    }

    fn is_primitive(&self) -> bool {
        self.datatype.is_primitive()
    }

    fn is_enum_reference(&self) -> bool {
        self.datatype.is_enum_reference()
    }

    fn is_enum_field(&self) -> bool {
        self.datatype.is_enum_field()
    }

    fn as_rust_ref(&self) -> String {
        format!(
            "pub {}: Option<{}>",
            replace_reserved_keyword(&self.name, Lang::Rust).snake_case(),
            self.datatype.as_rust_ref(self.name.clone())
        )
    }

    fn as_typescript_ref(&self) -> String {
        format!(
            "{}?: {}",
            replace_reserved_keyword(&self.name, Lang::TypeScript).camel_case(),
            self.datatype.as_typescript_ref(self.name.clone())
        )
    }

    fn as_typescript_schema(&self, r#type: SchemaType) -> String {
        format!(
            "{}: {}",
            replace_reserved_keyword(&self.name, Lang::TypeScript).camel_case(),
            self.datatype
                .as_typescript_schema(self.name.clone(), r#type)
        )
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "datatype")]
pub enum DataType<T = CommonModel> {
    #[default]
    String,
    Number,
    Boolean,
    Date,
    Enum {
        options: Option<Vec<String>>,
        #[serde(default)]
        reference: String,
    },
    Expandable(Expandable<T>),
    Array {
        #[serde(rename = "elementType")]
        element_type: Box<DataType<T>>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CommonEnum {
    #[serde(rename = "_id")]
    pub id: Id,
    pub name: String,
    pub options: Vec<String>,
}

fn replace_reserved_keyword(name: &str, lang: Lang) -> String {
    match lang {
        Lang::Rust => match name.to_lowercase().as_str() {
            "type" => "r#type".to_owned(),
            "enum" => "r#enum".to_owned(),
            "struct" => "r#struct".to_owned(),
            _ => name.to_owned(),
        },
        Lang::TypeScript => match name.to_lowercase().as_str() {
            "type" => "type_".to_owned(),
            "enum" => "enum_".to_owned(),
            "interface" => "interface_".to_owned(),
            _ => name.to_owned(),
        },
        _ => name.to_owned(),
    }
}

impl CommonEnum {
    pub fn as_rust_type(&self) -> String {
        format!(
            "pub enum {} {{ {} }}\n",
            replace_reserved_keyword(&self.name, Lang::Rust)
                .replace("::", "")
                .pascal_case(),
            self.options
                .iter()
                .map(|option| option.pascal_case())
                .collect::<HashSet<String>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    /// Generates a napi annotated enum for the enum rust type
    pub fn as_rust_schema(&self) -> String {
        format!(
            "{} pub enum {} {{ {} }}\n",
            "#[napi(string_enum = \"kebab-case\", js_name = {})]",
            replace_reserved_keyword(&self.name, Lang::Rust)
                .replace("::", "")
                .pascal_case(),
            self.options
                .iter()
                .map(|option| {
                    let option_name = option.pascal_case();
                    let option_value = if option.chars().all(char::is_uppercase) {
                        option.to_lowercase()
                    } else {
                        option.kebab_case()
                    };

                    let option_annotation = format!("#[napi(value = \"{}\")]", option_value);

                    format!("{} {}", option_annotation, option_name)
                })
                .collect::<HashSet<String>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn as_typescript_type(&self) -> String {
        // let's add the value directly to the enum
        format!(
            "export const enum {} {{ {} }}\n",
            replace_reserved_keyword(&self.name, Lang::TypeScript)
                .replace("::", "")
                .pascal_case(),
            self.options
                .iter()
                .map(|option| {
                    let option_name = option.pascal_case();
                    let option_value = if option.chars().all(char::is_uppercase) {
                        option.to_lowercase()
                    } else {
                        option.kebab_case()
                    };

                    format!("{} = '{}'", option_name, option_value)
                })
                .collect::<HashSet<String>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    /// Generates a effect Schema for the enum
    pub fn as_typescript_schema(&self) -> String {
        let name = replace_reserved_keyword(&self.name, Lang::TypeScript)
            .replace("::", "")
            .pascal_case();
        let native_enum = format!(
            "export enum {}Enum {{ {} }}\n",
            name,
            self.options
                .iter()
                .map(|option| {
                    let option_name = option.pascal_case();
                    let option_value = if option.chars().all(char::is_uppercase) {
                        option.to_lowercase()
                    } else {
                        option.kebab_case()
                    };

                    format!("{} = '{}'", option_name, option_value)
                })
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ")
        );

        let schema = format!(
            "export const {} = Schema.Enums({}Enum)\n // __SEPARATOR__\n",
            name, name
        );

        format!("{}\n{}", native_enum, schema)
    }
}

impl DataType {
    fn as_rust_ref(&self, e_name: String) -> String {
        match self {
            DataType::String => "String".into(),
            DataType::Number => "f64".into(),
            DataType::Boolean => "bool".into(),
            DataType::Date => "String".into(),
            DataType::Enum { reference, .. } => {
                if reference.is_empty() {
                    e_name.pascal_case()
                } else {
                    reference.into()
                }
            }
            DataType::Expandable(expandable) => expandable.reference(),
            DataType::Array { element_type } => {
                let name = (*element_type).as_rust_ref(e_name);
                format!("Vec<{}>", name)
            }
        }
    }

    fn as_typescript_ref(&self, enum_name: String) -> String {
        match self {
            DataType::String => "string".into(),
            DataType::Number => "number".into(),
            DataType::Boolean => "boolean".into(),
            DataType::Date => "Date".into(),
            DataType::Enum { reference, .. } => {
                if reference.is_empty() {
                    enum_name.pascal_case()
                } else {
                    reference.into()
                }
            }
            DataType::Expandable(expandable) => expandable.reference(),
            DataType::Array { element_type } => {
                let name = (*element_type).as_typescript_ref(enum_name);
                format!("{}[]", name)
            }
        }
    }

    fn as_typescript_schema(&self, enum_name: String, r#type: SchemaType) -> String {
        match self {
            DataType::String => {
                match r#type {
                    SchemaType::Lax => "Schema.optional(Schema.NullishOr(Schema.String))".into(),
                    SchemaType::Strict => "Schema.String".into()
                }
            },
            DataType::Number => {
                match r#type {
                    SchemaType::Lax => "Schema.optional(Schema.NullishOr(Schema.Number))".into(),
                    SchemaType::Strict => "Schema.Number".into()
                } },
            DataType::Boolean => {
                match r#type {
                    SchemaType::Lax => "Schema.optional(Schema.NullishOr(Schema.Boolean))".into(),
                    SchemaType::Strict => "Schema.Boolean".into()
                }

                },
            DataType::Date => {
                match r#type {
                    SchemaType::Lax => "Schema.optional(Schema.NullishOr(Schema.String.pipe(Schema.filter((d) => !isNaN(new Date(d).getTime())))))".into(),
                    SchemaType::Strict => "Schema.String.pipe(Schema.filter((d) => !isNaN(new Date(d).getTime())))".into()
                }
                },
            DataType::Enum { reference, .. } => {
                match r#type {
                    SchemaType::Lax => {
                        if reference.is_empty() {
                            format!(
                                "Schema.optional(Schema.NullishOr({}))",
                                enum_name.pascal_case()
                            )
                        } else {
                            format!("Schema.optional(Schema.NullishOr({}))", reference)
                        }
                    },
                    SchemaType::Strict => {
                        if reference.is_empty() {
                            enum_name.pascal_case()
                        } else {
                            reference.into()
                        }
                    }
                }

            }
            DataType::Expandable(expandable) => {
                match r#type {
                    SchemaType::Lax => {
                        format!(
                            "Schema.optional(Schema.NullishOr({}))",
                            expandable.reference()
                        )
                    },
                    SchemaType::Strict => {
                        expandable.reference()
                    }
                }
            }
            DataType::Array { element_type } => {
                match r#type {
                    SchemaType::Lax => {
                        let name = (*element_type).as_typescript_schema(enum_name, r#type);
                        let refined = if name.contains("Schema.optional") {
                            name.replace("Schema.optional(", "")
                                .replace(')', "")
                                .replace("Schema.NullishOr(", "")
                                .replace(')', "")
                        } else {
                            name
                        };
                        format!(
                            "Schema.optional(Schema.NullishOr(Schema.Array({})))",
                            refined
                        )
                    },
                    SchemaType::Strict => {
                        let name = (*element_type).as_typescript_schema(enum_name, r#type);
                        format!(
                            "Schema.Array({})",
                            name
                        )
                    }
                }

            }
        }
    }

    pub fn schema(&self, format: Option<String>) -> ReferenceOr<Box<Schema>> {
        match self {
            DataType::String => ReferenceOr::Item(Box::new(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType {
                    format: VariantOrUnknownOrEmpty::Unknown(format.unwrap_or_default()),
                    pattern: None,
                    ..Default::default()
                })),
            })),
            DataType::Number => ReferenceOr::Item(Box::new(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::Number(NumberType {
                    format: VariantOrUnknownOrEmpty::Unknown(format.unwrap_or_default()),
                    ..Default::default()
                })),
            })),
            DataType::Boolean => ReferenceOr::Item(Box::new(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::Boolean(BooleanType {
                    ..Default::default()
                })),
            })),
            DataType::Date => ReferenceOr::Item(Box::new(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType {
                    format: VariantOrUnknownOrEmpty::Unknown("date-time".to_string()),
                    ..Default::default()
                })),
            })),
            DataType::Enum { options, reference } => match options {
                Some(options) => ReferenceOr::Item(Box::new(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::String(StringType {
                        format: VariantOrUnknownOrEmpty::Unknown(format.unwrap_or_default()),
                        enumeration: options
                            .iter()
                            .map(|option| Some(option.to_owned()))
                            .collect(),
                        ..Default::default()
                    })),
                })),
                None => ReferenceOr::Reference {
                    reference: "#/components/schemas/".to_string() + reference,
                },
            },
            DataType::Array { element_type } => ReferenceOr::Item(Box::new(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::Array(ArrayType {
                    items: Some(element_type.schema(format)),
                    min_items: None,
                    max_items: None,
                    unique_items: false,
                })),
            })),
            DataType::Expandable(expandable) => match expandable {
                Expandable::Expanded { model, .. } => ReferenceOr::Item(Box::new(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                        properties: {
                            IndexMap::from_iter(
                                model
                                    .fields
                                    .iter()
                                    .map(|field| (field.name.clone(), field.datatype.schema(None)))
                                    .collect::<Vec<_>>(),
                            )
                        },
                        ..Default::default()
                    })),
                })),
                Expandable::Unexpanded { reference } => ReferenceOr::Reference {
                    reference: "#/components/schemas/".to_string() + reference,
                },
                _ => ReferenceOr::Item(Box::new(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::Object(Default::default())),
                })),
            },
        }
    }

    fn is_enum_reference(&self) -> bool {
        match self {
            DataType::Enum { reference, .. } => !reference.is_empty(),
            DataType::Array { element_type } => element_type.is_enum_reference(),
            _ => false,
        }
    }

    fn is_enum_field(&self) -> bool {
        match self {
            DataType::Enum { options, .. } => options.is_some(),
            DataType::Array { element_type } => element_type.is_enum_field(),
            _ => false,
        }
    }

    fn is_expandable(&self) -> bool {
        match self {
            DataType::Expandable { .. } => true,
            DataType::Array { element_type } => element_type.is_expandable(),
            _ => false,
        }
    }

    fn is_primitive(&self) -> bool {
        match self {
            DataType::String | DataType::Number | DataType::Boolean | DataType::Date => true,
            DataType::Array { element_type } => element_type.is_primitive(),
            _ => false,
        }
    }

    #[cfg(dummy)]
    fn to_fake(&self) -> Value {
        match &self {
            DataType::String => Value::String(fake::Faker.fake()),
            DataType::Number => Value::Number(fake::Faker.fake()),
            DataType::Boolean => Value::Bool(fake::Faker.fake()),
            DataType::Date => Value::Number(fake::Faker.fake()),
            DataType::Enum { options } => {
                let i: usize = (0..options.len()).fake();
                Value::String(options[i].clone())
            }
            DataType::Expandable(expandable) => match expandable {
                Expandable::Expanded { model, .. } => {
                    let mut map = Map::new();
                    for field in &model.fields {
                        map.insert(field.name.clone(), field.datatype.to_fake());
                    }
                    Value::Object(map)
                }
                _ => panic!(
                    "CommonModel must be fully expanded. Call `CommonModel::expand_all` first."
                ),
            },
            DataType::Array { element_type } => {
                let i: usize = (1..3).fake();
                let mut arr = vec![];
                for _ in 0..i {
                    arr.push((*element_type.clone()).to_fake());
                }
                Value::Array(arr)
            }
        }
    }

    pub fn to_name(&self) -> String {
        match &self {
            DataType::String => "String".to_owned(),
            DataType::Number => "Number".to_owned(),
            DataType::Boolean => "Boolean".to_owned(),
            DataType::Date => "Date".to_owned(),
            DataType::Enum { options, .. } => {
                // TODO: Add reference call
                let options = options.as_ref().unwrap_or(&vec![]).join("|");
                format!("Enum<{}>", options)
            }
            DataType::Expandable(expandable) => match expandable {
                Expandable::Expanded { reference, .. } => {
                    format!("Expandable<{reference}>")
                }
                Expandable::Unexpanded { reference } => {
                    format!("Expandable<{reference}>")
                }
                Expandable::NotFound { reference } => format!("Expandable<{reference}>"),
            },
            DataType::Array { element_type } => {
                let name = (*element_type).to_name();
                format!("Array<{name}>")
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum Expandable<T = CommonModel> {
    Expanded { reference: String, model: T },
    Unexpanded { reference: String },
    NotFound { reference: String },
}

impl<T> Expandable<T> {
    pub fn reference(&self) -> String {
        match self {
            Expandable::Expanded { reference, .. } => reference.clone(),
            Expandable::Unexpanded { reference } => reference.clone(),
            Expandable::NotFound { reference } => reference.clone(),
        }
    }
}

impl From<UnsavedCommonModel> for CommonModel {
    fn from(model: UnsavedCommonModel) -> Self {
        Self {
            id: Id::now(IdPrefix::CommonModel),
            name: model.name,
            fields: model.fields.into_iter().map(|f| f.into()).collect(),
            sample: model.sample,
            category: model.category,
            primary: model.primary,
            interface: model.interface,
            record_metadata: Default::default(),
        }
    }
}

impl From<Field<UnsavedCommonModel>> for Field {
    fn from(field: Field<UnsavedCommonModel>) -> Self {
        Self {
            name: field.name,
            datatype: field.datatype.into(),
            description: field.description,
            required: field.required,
        }
    }
}

impl From<DataType<UnsavedCommonModel>> for DataType {
    fn from(data_type: DataType<UnsavedCommonModel>) -> Self {
        match data_type {
            DataType::String => DataType::String,
            DataType::Number => DataType::Number,
            DataType::Boolean => DataType::Boolean,
            DataType::Date => DataType::Date,
            DataType::Enum { options, reference } => DataType::Enum { options, reference },
            DataType::Expandable(e) => DataType::Expandable(e.into()),
            DataType::Array { element_type } => DataType::Array {
                element_type: Box::new(element_type.deref().clone().into()),
            },
        }
    }
}

impl From<Expandable<UnsavedCommonModel>> for Expandable {
    fn from(expandable: Expandable<UnsavedCommonModel>) -> Self {
        match expandable {
            Expandable::Expanded { reference, model } => Expandable::Expanded {
                reference,
                model: model.into(),
            },
            Expandable::Unexpanded { reference } => Expandable::Unexpanded { reference },
            Expandable::NotFound { reference } => Expandable::NotFound { reference },
        }
    }
}

impl CommonModel {
    pub fn new(
        name: String,
        version: Version,
        fields: Vec<Field>,
        category: String,
        sample: Value,
        primary: bool,
        interface: HashMap<Lang, String>,
    ) -> Self {
        let mut record = Self {
            id: Id::new(IdPrefix::CommonModel, chrono::Utc::now()),
            name,
            fields,
            primary,
            sample,
            category,
            interface,
            record_metadata: Default::default(),
        };
        record.record_metadata.version = version;
        record
    }

    pub fn new_empty() -> Self {
        Self {
            id: Id::new(IdPrefix::CommonModel, chrono::Utc::now()),
            ..Default::default()
        }
    }

    pub fn reference(&self) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                properties: self.schema(),
                ..Default::default()
            })),
        }
    }

    /// Generates the model as a string in the specified language
    /// without recursively expanding inner models and enums. Simply
    /// provides a reference to the inner model or enum.
    ///
    /// # Arguments
    /// * `lang` - The language to generate the model in
    pub fn generate_as(&self, lang: &Lang) -> String {
        match lang {
            Lang::Rust => self.as_rust_ref(),
            Lang::TypeScript => self.as_typescript_ref(),
            _ => unimplemented!(),
        }
    }

    /// Generates the model as a string in the specified language
    /// with recursively expanded inner models and enums.
    /// This is useful for generating the entire model and its
    /// dependencies in a single file.
    ///
    /// # Arguments
    /// * `lang` - The language to generate the model in
    /// * `cm_store` - The store for common models
    /// * `ce_store` - The store for common enums
    pub async fn generate_as_expanded(
        &self,
        lang: &Lang,
        cm_store: &MongoStore<CommonModel>,
        ce_store: &MongoStore<CommonEnum>,
    ) -> String {
        match lang {
            Lang::Rust => self.as_rust_expanded(cm_store, ce_store).await,
            Lang::TypeScript => self.as_typescript_expanded(cm_store, ce_store).await,
            _ => unimplemented!(),
        }
    }

    fn as_rust_ref(&self) -> String {
        format!(
            "pub struct {} {{ {} }}\n",
            replace_reserved_keyword(&self.name, Lang::Rust)
                .replace("::", "")
                .pascal_case(),
            self.fields
                .iter()
                .map(|field| field.as_rust_ref())
                .collect::<HashSet<String>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(",\n    ")
        )
    }

    /// Generates an effect schema for the model in TypeScript
    fn as_typescript_schema(&self, r#type: SchemaType) -> String {
        format!(
            "export const {} = Schema.Struct({{ {} }}).annotations({{ title: '{}' }});\n",
            replace_reserved_keyword(&self.name, Lang::TypeScript)
                .replace("::", "")
                .pascal_case(),
            self.fields
                .iter()
                .map(|field| field.as_typescript_schema(r#type))
                .collect::<HashSet<String>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(",\n    "),
            self.name
        )
    }

    fn as_typescript_ref(&self) -> String {
        format!(
            "export interface {} {{ {} }}\n",
            replace_reserved_keyword(&self.name, Lang::TypeScript)
                .replace("::", "")
                .pascal_case(),
            self.fields
                .iter()
                .map(|field| field.as_typescript_ref())
                .collect::<HashSet<String>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(";\n    ")
        )
    }

    pub async fn as_typescript_schema_expanded(
        &self,
        cm_store: &MongoStore<CommonModel>,
        ce_store: &MongoStore<CommonEnum>,
        r#type: SchemaType,
    ) -> String {
        let mut visited_enums = HashSet::new();
        let mut visited_common_models = HashSet::new();

        let enums = self
            .fetch_all_enum_references(cm_store.clone(), ce_store.clone())
            .await
            .map(|enums| {
                enums
                    .iter()
                    .filter_map(|enum_model| {
                        if visited_enums.contains(&enum_model.id) {
                            return None;
                        }

                        visited_enums.insert(enum_model.id);

                        Some(enum_model.as_typescript_schema())
                    })
                    .collect::<HashSet<String>>()
                    .into_iter()
                    .collect::<Vec<_>>()
            })
            .ok()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();

        let children = self
            .fetch_all_children_common_models(cm_store.clone())
            .await
            .ok()
            .unwrap_or_default();

        let children_types = children
            .0
            .into_values()
            .filter_map(|child| {
                if visited_common_models.contains(&child.id) {
                    return None;
                }
                visited_common_models.insert(child.id);

                Some(child.as_typescript_schema(r#type))
            })
            .collect::<Vec<_>>()
            .join("\n // __SEPARATOR__ \n");

        let ce_types = enums.join("\n");

        let cm_types = self.as_typescript_schema(r#type);

        if visited_common_models.contains(&self.id) {
            format!(
                "// __SEPARATOR \n {}\n // __SEPARATOR__ \n {}",
                ce_types, children_types
            )
        } else {
            format!(
                "// __SEPARATOR__ \n {}\n{}\n // __SEPARATOR__ \n{}",
                ce_types, children_types, cm_types
            )
        }
    }

    async fn as_typescript_expanded(
        &self,
        cm_store: &MongoStore<CommonModel>,
        ce_store: &MongoStore<CommonEnum>,
    ) -> String {
        let mut visited_enums = HashSet::new();
        let mut visited_common_models = HashSet::new();

        let enums = self
            .fetch_all_enum_references(cm_store.clone(), ce_store.clone())
            .await
            .map(|enums| {
                enums
                    .iter()
                    .filter_map(|enum_model| {
                        if visited_enums.contains(&enum_model.id) {
                            return None;
                        }

                        visited_enums.insert(enum_model.id);

                        Some(enum_model.as_typescript_type())
                    })
                    .collect::<HashSet<String>>()
                    .into_iter()
                    .collect::<Vec<_>>()
            })
            .ok()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();

        let children = self
            .fetch_all_children_common_models(cm_store.clone())
            .await
            .ok()
            .unwrap_or_default();

        let children_types = children
            .0
            .into_values()
            .filter_map(|child| {
                if visited_common_models.contains(&child.id) {
                    return None;
                }
                visited_common_models.insert(child.id);
                Some(format!(
                    "export interface {} {{ {} }}\n",
                    replace_reserved_keyword(&child.name, Lang::TypeScript)
                        .replace("::", "")
                        .pascal_case(),
                    child
                        .fields
                        .iter()
                        .map(|field| field.as_typescript_ref())
                        .collect::<HashSet<String>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .join(";\n    ")
                ))
            })
            .collect::<Vec<_>>()
            .join("\n");

        let ce_types = enums.join("\n");

        let cm_types = format!(
            "export interface {} {{ {} }}\n",
            replace_reserved_keyword(&self.name, Lang::TypeScript)
                .replace("::", "")
                .pascal_case(),
            self.fields
                .iter()
                .map(|field| field.as_typescript_ref())
                .collect::<HashSet<String>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(";\n    ")
        );

        if visited_common_models.contains(&self.id) {
            format!("{}\n{}", ce_types, children_types)
        } else {
            format!("{}\n{}\n{}", ce_types, children_types, cm_types)
        }
    }

    async fn as_rust_expanded(
        &self,
        cm_store: &MongoStore<CommonModel>,
        ce_store: &MongoStore<CommonEnum>,
    ) -> String {
        let mut visited_enums = HashSet::new();
        let mut visited_common_models = HashSet::new();

        let enums = self
            .fetch_all_enum_references(cm_store.clone(), ce_store.clone())
            .await
            .map(|enums| {
                enums
                    .iter()
                    .filter_map(|enum_model| {
                        if visited_enums.contains(&enum_model.id) {
                            return None;
                        }

                        visited_enums.insert(enum_model.id);
                        Some(enum_model.as_rust_type())
                    })
                    .collect::<HashSet<String>>()
                    .into_iter()
                    .collect::<Vec<_>>()
            })
            .ok()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();

        let children = self
            .fetch_all_children_common_models(cm_store.clone())
            .await
            .ok()
            .unwrap_or_default();

        let children_types = children
            .0
            .into_values()
            .filter_map(|child| {
                if visited_common_models.contains(&child.id) {
                    return None;
                }
                visited_common_models.insert(child.id);
                Some(format!(
                    "pub struct {} {{ {} }}\n",
                    replace_reserved_keyword(&child.name, Lang::Rust)
                        .replace("::", "")
                        .pascal_case(),
                    child
                        .fields
                        .iter()
                        .map(|field| field.as_rust_ref())
                        .collect::<HashSet<String>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .join(",\n    ")
                ))
            })
            .collect::<Vec<_>>()
            .join("\n");

        let ce_types = enums.join("\n");

        let cm_types = format!(
            "pub struct {} {{ {} }}\n",
            replace_reserved_keyword(&self.name, Lang::Rust)
                .replace("::", "")
                .pascal_case(),
            self.fields
                .iter()
                .map(|field| field.as_rust_ref())
                .collect::<HashSet<String>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(",\n    ")
        );

        if visited_common_models.contains(&self.id) {
            format!("{}\n{}", ce_types, children_types)
        } else {
            format!("{}\n{}\n{}", ce_types, children_types, cm_types)
        }
    }

    fn schema(&self) -> IndexMap<String, ReferenceOr<Box<Schema>>> {
        self.fields
            .iter()
            .fold(IndexMap::new(), |mut index, field| {
                let schema = field.datatype.schema(Some(field.name.to_owned()));

                index.insert(field.name.clone(), schema);
                index
            })
    }

    pub fn request_body(&self, required: bool) -> RequestBody {
        let mut content = IndexMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType {
                schema: Some(ReferenceOr::Reference {
                    reference: "#/components/schemas/".to_owned() + self.name.as_str(),
                }),
                ..Default::default()
            },
        );

        RequestBody {
            content,
            required,
            ..Default::default()
        }
    }

    pub fn get_expandable_fields(&self) -> Vec<Field> {
        self.fields
            .iter()
            .filter(|field| field.is_expandable())
            .cloned()
            .collect()
    }

    pub fn get_primitive_fields(&self) -> Vec<Field> {
        self.fields
            .iter()
            .filter(|field| field.is_primitive())
            .cloned()
            .collect()
    }

    pub fn get_enum_references(&self) -> Vec<Field> {
        self.fields
            .iter()
            .filter(|field| field.is_enum_reference())
            .map(|field| {
                if let DataType::Array { element_type } = &field.datatype {
                    Field {
                        name: field.name.clone(),
                        datatype: element_type.deref().clone(),
                        description: field.description.clone(),
                        required: field.required,
                    }
                } else {
                    field.clone()
                }
            })
            .collect()
    }

    pub fn get_enum_fields(&self) -> Vec<Field> {
        self.fields
            .iter()
            .filter(|field| field.is_enum_field())
            .map(|field| {
                if let DataType::Array { element_type } = &field.datatype {
                    Field {
                        name: field.name.clone(),
                        datatype: element_type.deref().clone(),
                        description: field.description.clone(),
                        required: field.required,
                    }
                } else {
                    field.clone()
                }
            })
            .collect()
    }

    pub fn flatten(mut self) -> Vec<CommonModel> {
        let mut models = vec![];
        for field in &self.fields {
            match &field.datatype {
                DataType::Expandable(Expandable::Expanded { model, .. }) => {
                    models.extend(model.clone().flatten());
                }
                DataType::Array { element_type } => {
                    if let DataType::Expandable(Expandable::Expanded { model, .. }) =
                        element_type.deref()
                    {
                        models.extend(model.clone().flatten());
                    }
                }
                _ => {}
            }
        }

        for field in self.fields.iter_mut() {
            match field.datatype {
                DataType::Expandable(Expandable::Expanded { ref reference, .. }) => {
                    field.datatype = DataType::Expandable(Expandable::Unexpanded {
                        reference: reference.clone(),
                    })
                }
                DataType::Array { ref element_type } => {
                    if let DataType::Expandable(Expandable::Expanded { ref reference, .. }) =
                        element_type.deref()
                    {
                        field.datatype = DataType::Array {
                            element_type: Box::new(DataType::Expandable(Expandable::Unexpanded {
                                reference: reference.clone(),
                            })),
                        }
                    }
                }
                _ => {}
            }
        }

        models.push(self);

        models
    }

    pub async fn expand_all(
        &self,
        cm_store: MongoStore<CommonModel>,
        ce_store: MongoStore<CommonEnum>,
    ) -> Result<Self, IntegrationOSError> {
        const MAX_NESTING_LEVEL: u8 = 100; // Maximum nesting level of 10; adjust as needed
        self.expand_all_recursive(cm_store, ce_store, MAX_NESTING_LEVEL)
            .await
    }

    #[async_recursion]
    async fn expand_all_recursive(
        &self,
        cm_store: MongoStore<CommonModel>,
        ce_store: MongoStore<CommonEnum>,
        nesting: u8,
    ) -> Result<Self, IntegrationOSError> {
        if nesting == 0 {
            return Ok(self.clone()); // Avoid infinite recursion
        }

        let mut new_model = self.clone();
        let ts = self
            .generate_as_expanded(&Lang::TypeScript, &cm_store, &ce_store)
            .await;
        let rust = self
            .generate_as_expanded(&Lang::Rust, &cm_store, &ce_store)
            .await;
        let interface = HashMap::from_iter(vec![(Lang::Rust, rust), (Lang::TypeScript, ts)]);
        new_model.interface = interface;
        new_model.fields = Vec::new(); // Clear the fields to populate them freshly

        for field in &self.fields {
            match &field.datatype {
                DataType::Expandable(expandable) => {
                    let expanded = expandable.expand(cm_store.clone()).await?;
                    let expanded_field = Field {
                        name: field.name.clone(),
                        datatype: DataType::Expandable(expanded),
                        required: field.required,
                        description: field.description.clone(),
                    };

                    match &expanded_field.datatype {
                        DataType::Expandable(Expandable::Expanded { model, .. }) => {
                            let recursively_expanded_model = model
                                .expand_all_recursive(
                                    cm_store.clone(),
                                    ce_store.clone(),
                                    nesting - 1,
                                )
                                .await?;
                            new_model.fields.push(Field {
                                name: field.name.clone(),
                                datatype: DataType::Expandable(Expandable::Expanded {
                                    reference: model.name.clone(),
                                    model: recursively_expanded_model,
                                }),
                                required: field.required,
                                description: field.description.clone(),
                            });
                        }
                        _ => {
                            new_model.fields.push(expanded_field);
                        }
                    }
                }
                DataType::Array { element_type } => match &**element_type {
                    DataType::Expandable(expandable) => {
                        let mut expanded = expandable.expand(cm_store.clone()).await?;
                        if let Expandable::Expanded { model, .. } = &expanded {
                            let recursively_expanded_model = model
                                .expand_all_recursive(
                                    cm_store.clone(),
                                    ce_store.clone(),
                                    nesting - 1,
                                )
                                .await?;
                            expanded = Expandable::Expanded {
                                reference: model.name.clone(),
                                model: recursively_expanded_model,
                            };
                        }
                        let expanded_field = Field {
                            name: field.name.clone(),
                            datatype: DataType::Array {
                                element_type: Box::new(DataType::Expandable(expanded)),
                            },
                            required: field.required,
                            description: field.description.clone(),
                        };
                        new_model.fields.push(expanded_field);
                    }
                    DataType::Enum { reference, .. } if !reference.is_empty() => {
                        let enum_model = ce_store.get_one(doc! { "name": reference }).await?;
                        if let Some(enum_model) = enum_model {
                            new_model.fields.push(Field {
                                name: field.name.clone(),
                                datatype: DataType::Enum {
                                    options: Some(
                                        enum_model
                                            .options
                                            .iter()
                                            .map(|option| option.to_owned())
                                            .collect(),
                                    ),
                                    reference: reference.clone(),
                                },
                                required: field.required,
                                description: field.description.clone(),
                            });
                        }
                    }
                    _ => {
                        new_model.fields.push(field.clone());
                    }
                },
                DataType::Enum { reference, .. } if !reference.is_empty() => {
                    let enum_model = ce_store.get_one(doc! { "name": reference }).await?;
                    if let Some(enum_model) = enum_model {
                        new_model.fields.push(Field {
                            name: field.name.clone(),
                            datatype: DataType::Enum {
                                options: Some(
                                    enum_model
                                        .options
                                        .iter()
                                        .map(|option| option.to_owned())
                                        .collect(),
                                ),
                                reference: reference.clone(),
                            },
                            required: field.required,
                            description: field.description.clone(),
                        });
                    }
                }
                _ => {
                    new_model.fields.push(field.clone());
                }
            }
        }

        Ok(new_model)
    }

    /// Fetches all the enum references and non-enum references of the current model and its children
    ///
    /// # Arguments
    /// * `cm_store` - The store to fetch the common models from
    /// * `ce_store` - The store to fetch the common enums from
    ///
    /// # Returns
    /// A vector of all the enum references and flat enums that are not common
    pub async fn fetch_all_enum_references(
        &self,
        cm_store: MongoStore<CommonModel>,
        ce_store: MongoStore<CommonEnum>,
    ) -> Result<Vec<CommonEnum>, IntegrationOSError> {
        let mut enum_references = self
            .get_enum_references()
            .into_iter()
            .filter_map(|x| match x.datatype {
                DataType::Enum { reference, .. } => Some(reference.pascal_case()),
                _ => None,
            })
            .collect::<HashSet<_>>();

        let mut flat_enums = self
            .get_enum_fields()
            .into_iter()
            .filter_map(|e| match e.datatype {
                DataType::Enum { options, .. } => Some(CommonEnum {
                    id: Id::now(IdPrefix::CommonEnum),
                    name: e.name.pascal_case(),
                    options: options.unwrap_or_default(),
                }),
                _ => None,
            })
            .collect::<HashSet<_>>();

        for (_, child) in self
            .fetch_all_children_common_models(cm_store.clone())
            .await?
            .0
        {
            enum_references.extend(child.get_enum_references().into_iter().filter_map(|x| {
                match x.datatype {
                    DataType::Enum { reference, .. } => Some(reference.pascal_case()),
                    _ => None,
                }
            }));

            let child_enums = child
                .get_enum_fields()
                .into_iter()
                .filter_map(|e| match e.datatype {
                    DataType::Enum { options, .. } => Some(CommonEnum {
                        id: Id::now(IdPrefix::CommonEnum),
                        name: e.name.pascal_case(),
                        options: options.unwrap_or_default(),
                    }),
                    _ => None,
                })
                .collect::<HashSet<_>>();

            flat_enums.extend(child_enums);
        }

        let enums = ce_store
            .get_many(
                Some(doc! {
                    "name": {
                        "$in": bson::to_bson(&enum_references).map_err(|e| InternalError::invalid_argument(&e.to_string(), Some("enum references")))?,
                    }
                }),
                None,
                None,
                None,
                None,
            )
            .await?;

        let enums = enums
            .into_iter()
            .chain(flat_enums.into_iter())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        Ok(enums)
    }

    /// Fetches all the children of the current model and returns two values:
    ///
    /// * A map of the children models with their names as keys
    /// * A set of the names of the children models that were not found
    pub async fn fetch_all_children_common_models(
        &self,
        store: MongoStore<CommonModel>,
    ) -> Result<(HashMap<String, CommonModel>, HashSet<String>), IntegrationOSError> {
        let mut map = HashMap::new();
        let mut queue = vec![self.clone()];
        let mut not_found = HashSet::new();

        while !queue.is_empty() {
            let mut refs = HashSet::new();

            while let Some(common_model) = queue.pop() {
                for field in &common_model.fields {
                    let expandable = match &field.datatype {
                        DataType::Array { element_type } => {
                            let DataType::Expandable(expandable) = &**element_type else {
                                continue;
                            };

                            expandable
                        }
                        DataType::Expandable(expandable) => expandable,
                        _ => {
                            continue;
                        }
                    };

                    match expandable {
                        Expandable::Expanded { model, .. } => {
                            if map.contains_key(&model.name) {
                                continue;
                            }
                            map.insert(model.name.clone(), model.clone());
                            queue.push(model.clone());
                        }
                        Expandable::Unexpanded { reference } => {
                            if map.contains_key(reference) {
                                continue;
                            }
                            refs.insert(reference.clone());
                        }
                        _ => {
                            continue;
                        }
                    };
                }
            }

            let models = store
                .get_many(
                    Some(doc! {
                        "name": {
                            "$in": bson::to_bson(&refs).map_err(|e| InternalError::invalid_argument(&e.to_string(), Some("model references")))?,
                        }
                    }),
                    None,
                    None,
                    None,
                    None,
                )
                .await?;

            let not_found_refs: HashSet<String> = refs
                .difference(&models.iter().map(|model| model.name.clone()).collect())
                .cloned()
                .collect();

            not_found.extend(not_found_refs);

            for model in models {
                if map.contains_key(&model.name) {
                    continue;
                }
                map.insert(model.name.clone(), model.clone());
                queue.push(model.clone());
            }
        }
        Ok((map, not_found))
    }

    pub async fn get_all_common_models(
        store: MongoStore<CommonModel>,
    ) -> Result<Vec<String>, IntegrationOSError> {
        let docs = store
            .aggregate(vec![doc! {
                "$group": {
                    "_id": "",
                    "list": {"$addToSet": "$name"}
                }
            }])
            .await?;

        let first_doc = docs.first().unwrap_or(&doc! {}).clone();

        #[derive(Debug, Serialize, Deserialize)]
        struct AggregateResult {
            list: Vec<String>,
        }
        Ok(bson::from_document::<AggregateResult>(first_doc)
            .map_err(|e| {
                InternalError::invalid_argument(&e.to_string(), Some("common model names"))
            })?
            .list)
    }

    #[cfg(dummy)]
    pub fn to_fake(self) -> Value {
        let mut fake = Map::new();

        for field in self.fields {
            fake.insert(field.name, field.datatype.to_fake());
        }

        Value::Object(fake)
    }

    pub fn to_flat_json(&self) -> Value {
        let mut map = Map::new();

        for field in &self.fields {
            let name = field.datatype.to_name();
            map.insert(field.name.clone(), Value::String(name));
        }

        json!({
            "name": self.name,
            "fields": Value::Object(map)
        })
    }
}

impl Expandable {
    pub async fn expand(&self, store: MongoStore<CommonModel>) -> Result<Self, IntegrationOSError> {
        Ok(match self {
            Expandable::Unexpanded { reference } => {
                if let Some(model) = store.get_one(doc! { "name": &reference }).await? {
                    Expandable::Expanded {
                        reference: reference.clone(),
                        model,
                    }
                } else {
                    Expandable::NotFound {
                        reference: reference.clone(),
                    }
                }
            }
            _ => self.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_as_rust_ref_is_correct() {
        let field = Field {
            name: "name".to_string(),
            datatype: DataType::String,
            description: None,
            required: true,
        };

        assert_eq!(field.as_rust_ref(), "pub name: Option<String>");
    }

    #[test]
    fn test_data_type_as_rust_reference_is_correct() {
        let data_type = DataType::String;
        assert_eq!(data_type.as_rust_ref("String".into()), "String");

        let data_type = DataType::Number;
        assert_eq!(data_type.as_rust_ref("String".into()), "f64");

        let data_type = DataType::Boolean;
        assert_eq!(data_type.as_rust_ref("".into()), "bool");

        let data_type = DataType::Date;
        assert_eq!(data_type.as_rust_ref("".into()), "String");

        let data_type = DataType::Enum {
            options: Some(vec!["option1".to_string(), "option2".to_string()]),
            reference: "Reference".to_string(),
        };
        assert_eq!(data_type.as_rust_ref("".into()), "Reference");

        let data_type = DataType::Expandable(Expandable::Unexpanded {
            reference: "Reference".to_string(),
        });
        assert_eq!(data_type.as_rust_ref("".into()), "Reference");

        let data_type = DataType::Array {
            element_type: Box::new(DataType::String),
        };
        assert_eq!(data_type.as_rust_ref("String".into()), "Vec<String>");
    }

    #[test]
    fn test_common_model_as_rust_struct_is_correct() {
        let common_model = CommonModel {
            id: Id::new(IdPrefix::CommonModel, chrono::Utc::now()),
            name: "Model".to_string(),
            fields: vec![
                Field {
                    name: "name".to_string(),
                    datatype: DataType::String,
                    description: None,
                    required: true,
                },
                Field {
                    name: "age".to_string(),
                    datatype: DataType::Number,
                    description: None,
                    required: true,
                },
            ],
            sample: json!({
                "name": "John Doe",
                "age": 25
            }),
            primary: true,
            category: "Category".to_string(),
            interface: Default::default(),
            record_metadata: Default::default(),
        };

        let rust_struct = common_model.as_rust_ref();
        let typescript_interface = common_model.as_typescript_ref();

        assert!(
            rust_struct.contains(
                "pub struct Model { pub age: Option<f64>,\n    pub name: Option<String> }"
            ) || rust_struct.contains(
                "pub struct Model { pub name: Option<String>,\n    pub age: Option<f64> }"
            )
        );

        assert!(
            typescript_interface
                .contains("export interface Model { age?: number;\n    name?: string }")
                || typescript_interface
                    .contains("export interface Model { name?: string;\n    age?: number }")
        );
    }

    #[test]
    fn test_common_model_as_lax_schema_is_correct() {
        let common_model = CommonModel {
            id: Id::new(IdPrefix::CommonModel, chrono::Utc::now()),
            name: "Model".to_string(),
            fields: vec![
                Field {
                    name: "name".to_string(),
                    datatype: DataType::String,
                    description: None,
                    required: true,
                },
                Field {
                    name: "age".to_string(),
                    datatype: DataType::Number,
                    description: None,
                    required: true,
                },
            ],
            sample: json!({
                "name": "John Doe",
                "age": 25
            }),
            primary: true,
            category: "Category".to_string(),
            interface: Default::default(),
            record_metadata: Default::default(),
        };

        let lax_schema = common_model.as_typescript_schema(SchemaType::Lax);
        assert_eq!(
            lax_schema,
            "export const Model = Schema.Struct({ age: Schema.optional(Schema.NullishOr(Schema.Number)),\n    name: Schema.optional(Schema.NullishOr(Schema.String)) }).annotations({ title: 'Model' });\n"
        );
    }

    #[test]
    fn test_common_model_as_strict_schema_is_correct() {
        let common_model = CommonModel {
            id: Id::new(IdPrefix::CommonModel, chrono::Utc::now()),
            name: "Model".to_string(),
            fields: vec![
                Field {
                    name: "name".to_string(),
                    datatype: DataType::String,
                    description: None,
                    required: true,
                },
                Field {
                    name: "age".to_string(),
                    datatype: DataType::Number,
                    description: None,
                    required: true,
                },
            ],
            sample: json!({
                "name": "John Doe",
                "age": 25
            }),
            primary: true,
            category: "Category".to_string(),
            interface: Default::default(),
            record_metadata: Default::default(),
        };

        let strict_schema = common_model.as_typescript_schema(SchemaType::Strict);
        assert_eq!(
            strict_schema,
            "export const Model = Schema.Struct({ age: Schema.Number,\n    name: Schema.String }).annotations({ title: 'Model' });\n"
        );
    }
}
