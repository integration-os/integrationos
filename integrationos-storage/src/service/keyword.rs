use crate::domain::{
    keyword::{Direction, Keyword, LogicOperator},
    postgres::PostgresStorage,
};

pub trait KeywordQuery {
    fn parse(&self, keywords: Vec<Keyword>) -> String;
}

impl KeywordQuery for PostgresStorage {
    fn parse(&self, keywords: Vec<Keyword>) -> String {
        let mut sql = String::new();

        // Sort the keywords by their weight to maintain SQL order
        let mut sorted_keywords = keywords;
        sorted_keywords.sort_by_key(|k| k.weight());

        // Process each keyword in order
        for keyword in sorted_keywords {
            match keyword {
                Keyword::Select { columns } => {
                    sql.push_str(&format!("SELECT {} ", columns.join(", ")));
                }
                Keyword::Join { pipeline } => {
                    // Assuming pipeline has its own parse method
                    sql.push_str(&format!("JOIN ({}) ", self.parse(pipeline.steps)));
                }
                Keyword::Filter {
                    column,
                    operator,
                    value,
                } => {
                    sql.push_str(&format!(
                        "WHERE {} {} '{}' ",
                        column,
                        operator.as_postgres(),
                        value
                    ));
                }
                Keyword::FullTextSearch {
                    column,
                    search_type,
                    query,
                } => {
                    sql.push_str(&format!(
                        "WHERE {} {} '{}' ",
                        column,
                        search_type.as_postgres(),
                        query
                    ));
                }
                Keyword::Logic {
                    left,
                    operator,
                    right,
                } => {
                    let left_query = self.parse(vec![*left]);
                    let right_query = self.parse(vec![*right]);
                    let operator_str = match operator {
                        LogicOperator::And => "AND",
                        LogicOperator::Or => "OR",
                    };
                    sql.push_str(&format!(
                        "({}) {} ({}) ",
                        left_query, operator_str, right_query
                    ));
                }
                Keyword::Order { column, direction } => {
                    sql.push_str(&format!(
                        "ORDER BY {} {} ",
                        column,
                        match direction {
                            Direction::Asc => "ASC",
                            Direction::Desc => "DESC",
                        }
                    ));
                }
                Keyword::Limit { number } => {
                    sql.push_str(&format!("LIMIT {} ", number));
                }
                Keyword::Offset { number } => {
                    sql.push_str(&format!("OFFSET {} ", number));
                }
                Keyword::Aggregate { function, column } => {
                    if let Some(col) = column {
                        sql.push_str(&format!("{}({}) ", function.as_postgres(), col));
                    }
                }
                Keyword::Distinct { columns } => {
                    sql.push_str(&format!("DISTINCT {} ", columns.join(", ")));
                }
            }
        }

        sql.trim_end().to_string() // Remove trailing spaces
    }
}
