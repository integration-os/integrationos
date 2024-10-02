/// | **URL Parameter**          | **SQL Equivalent**                                      |
/// |----------------------------|--------------------------------------------------------|
/// | `select=columns`            | `SELECT columns FROM table`                            |
/// | `column=eq.value`           | `WHERE column = 'value'`                               |
/// | `order=column.asc`          | `ORDER BY column ASC`                                  |
/// | `limit=<number>`            | `LIMIT number`                                         |
/// | `offset=<number>`           | `OFFSET number`                                        |
/// | `column=like.value%`        | `WHERE column LIKE 'value%'`                           |
/// | `column=ilike.value%`       | `WHERE column ILIKE 'value%'`                          |
/// | `column=gt.value`           | `WHERE column > 'value'`                               |
/// | `column=in.(value1,value2)` | `WHERE column IN ('value1', 'value2')`                 |
/// | `count=exact`               | Returns the total count of rows                        |
/// | `or=(column1.eq.val1,column2.eq.val2)` | `WHERE column1 = 'val1' OR column2 = 'val2'` |
/// | `jsonb_column->>key=eq.val` | `WHERE jsonb_column->>'key' = 'val'`                   |

pub enum Keyword {
    Select {
        columns: Vec<String>,
    },
    Filter {
        column: String,
        operator: Operator,
        value: String,
    },
    Order {
        column: String,
        direction: Direction,
    },
    Limit {
        number: u64,
    },
    Offset {
        number: u64,
    },
    Aggregate {
        function: AggregateFunction,
        column: Option<String>,
    },
    Join {
        pipeline: Box<Pipeline>,
    },
    Distinct {
        columns: Vec<String>,
    },
    FullTextSearch {
        column: String,
        query: String,
        search_type: FullTextSearchType,
    },
    Logic {
        left: Box<Keyword>,
        operator: LogicOperator,
        right: Box<Keyword>,
    },
}

impl Keyword {
    pub fn weight(&self) -> u8 {
        match self {
            Keyword::Select { .. } => 1,
            Keyword::Join { .. } => 2,
            Keyword::Filter { .. } => 3,
            Keyword::FullTextSearch { .. } => 3,
            Keyword::Logic { .. } => 3,
            Keyword::Order { .. } => 4,
            Keyword::Limit { .. } => 5,
            Keyword::Offset { .. } => 6,
            Keyword::Aggregate { .. } => 7,
            Keyword::Distinct { .. } => 8,
        }
    }
}

pub struct Condition {
    pub column: String,
    pub operator: Operator,
    pub value: ValueType,
}

pub enum LogicOperator {
    And,
    Or,
}

pub enum Operator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    ILike,
    In,
    NotIn,
    IsNull,
    IsNotNull,
}

impl Operator {
    pub fn as_postgres(&self) -> &str {
        match self {
            Operator::Eq => "=",
            Operator::Neq => "<>",
            Operator::Gt => ">",
            Operator::Lt => "<",
            Operator::Like => "LIKE",
            Operator::ILike => "ILIKE",
            Operator::In => "IN",
            Operator::NotIn => "NOT IN",
            Operator::IsNull => "IS NULL",
            Operator::IsNotNull => "IS NOT NULL",
            Operator::Gte => ">=",
            Operator::Lte => "<=",
        }
    }
}

pub enum Direction {
    Asc,
    Desc,
}

pub enum AggregateFunction {
    Count,
    Avg,
    Min,
    Max,
    Sum,
}

impl AggregateFunction {
    pub fn as_postgres(&self) -> &str {
        match self {
            AggregateFunction::Count => "COUNT",
            AggregateFunction::Avg => "AVG",
            AggregateFunction::Min => "MIN",
            AggregateFunction::Max => "MAX",
            AggregateFunction::Sum => "SUM",
        }
    }
}

pub enum FullTextSearchType {
    TsQuery,
    TsVector,
    PlainText,
    PhraseSearch,
    WebSearch,
}

impl FullTextSearchType {
    pub fn as_postgres(&self) -> &str {
        match self {
            FullTextSearchType::TsQuery => "to_tsquery",
            FullTextSearchType::TsVector => "to_tsvector",
            FullTextSearchType::PlainText => "plain_text", // Could represent using LIKE
            FullTextSearchType::PhraseSearch => "phrase_search", // Define how to represent this
            FullTextSearchType::WebSearch => "websearch_to_tsquery",
        }
    }
}

pub enum ValueType {
    String(String),
    Array(Vec<String>),
    Json(serde_json::Value),
}

pub struct Pipeline {
    pub steps: Vec<Keyword>, // A sequence of SQL keywords (operations) that form a pipeline
}
