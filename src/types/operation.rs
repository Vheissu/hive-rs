use serde::de::Error as _;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::types::Asset;

// Field declaration order in each operation struct is intentionally aligned with
// Hive's binary serializer order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Vote(VoteOperation),                           // 0
    Comment(CommentOperation),                     // 1
    Transfer(TransferOperation),                   // 2
    CustomJson(CustomJsonOperation),               // 18
    RecurrentTransfer(RecurrentTransferOperation), // 49
}

impl Operation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Vote(_) => "vote",
            Self::Comment(_) => "comment",
            Self::Transfer(_) => "transfer",
            Self::CustomJson(_) => "custom_json",
            Self::RecurrentTransfer(_) => "recurrent_transfer",
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Self::Vote(_) => 0,
            Self::Comment(_) => 1,
            Self::Transfer(_) => 2,
            Self::CustomJson(_) => 18,
            Self::RecurrentTransfer(_) => 49,
        }
    }
}

impl Serialize for Operation {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(self.name())?;
        match self {
            Self::Vote(op) => seq.serialize_element(op)?,
            Self::Comment(op) => seq.serialize_element(op)?,
            Self::Transfer(op) => seq.serialize_element(op)?,
            Self::CustomJson(op) => seq.serialize_element(op)?,
            Self::RecurrentTransfer(op) => seq.serialize_element(op)?,
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Vec::<Value>::deserialize(deserializer)?;
        if value.len() != 2 {
            return Err(D::Error::custom("operation must be a 2-item array"));
        }

        let op_name = value[0]
            .as_str()
            .ok_or_else(|| D::Error::custom("operation name must be a string"))?;
        let op_value = value[1].clone();

        match op_name {
            "vote" => Ok(Self::Vote(
                serde_json::from_value(op_value).map_err(D::Error::custom)?,
            )),
            "comment" => Ok(Self::Comment(
                serde_json::from_value(op_value).map_err(D::Error::custom)?,
            )),
            "transfer" => Ok(Self::Transfer(
                serde_json::from_value(op_value).map_err(D::Error::custom)?,
            )),
            "custom_json" => Ok(Self::CustomJson(
                serde_json::from_value(op_value).map_err(D::Error::custom)?,
            )),
            "recurrent_transfer" => Ok(Self::RecurrentTransfer(
                serde_json::from_value(op_value).map_err(D::Error::custom)?,
            )),
            _ => Err(D::Error::custom(format!(
                "unsupported operation type '{op_name}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OperationName {
    Vote = 0,
    Comment = 1,
    Transfer = 2,
    CustomJson = 18,
    RecurrentTransfer = 49,
}

impl OperationName {
    pub fn id(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoteOperation {
    pub voter: String,
    pub author: String,
    pub permlink: String,
    pub weight: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentOperation {
    pub parent_author: String,
    pub parent_permlink: String,
    pub author: String,
    pub permlink: String,
    pub title: String,
    pub body: String,
    pub json_metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferOperation {
    pub from: String,
    pub to: String,
    pub amount: Asset,
    pub memo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CustomJsonOperation {
    #[serde(default)]
    pub required_auths: Vec<String>,
    #[serde(default)]
    pub required_posting_auths: Vec<String>,
    pub id: String,
    pub json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecurrentTransferOperation {
    pub from: String,
    pub to: String,
    pub amount: Asset,
    pub memo: String,
    pub recurrence: u16,
    pub executions: u16,
    #[serde(default)]
    pub extensions: Vec<()>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Operation, TransferOperation};
    use crate::types::Asset;

    #[test]
    fn operation_tuple_format_round_trip() {
        let op = Operation::Transfer(TransferOperation {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: Asset::from_string("1.000 HIVE").expect("asset should parse"),
            memo: "hello".to_string(),
        });

        let serialized = serde_json::to_value(&op).expect("operation should serialize");
        assert_eq!(
            serialized,
            json!([
                "transfer",
                {
                    "from": "alice",
                    "to": "bob",
                    "amount": "1.000 HIVE",
                    "memo": "hello"
                }
            ])
        );

        let parsed: Operation = serde_json::from_value(serialized).expect("operation should parse");
        match parsed {
            Operation::Transfer(value) => {
                assert_eq!(value.from, "alice");
                assert_eq!(value.to, "bob");
            }
            _ => panic!("expected transfer operation"),
        }
    }
}
