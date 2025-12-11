use serde::{Deserialize, Serialize};

// Custom deserializer for comma-separated integers in query parameters
pub fn deserialize_comma_separated_ints<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<i32>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, Visitor};
    use std::fmt;

    struct OptionalCommaSeparatedInts;

    impl<'de> Visitor<'de> for OptionalCommaSeparatedInts {
        type Value = Option<Vec<i32>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("optional comma-separated integers or a single integer")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            if s.is_empty() {
                return Ok(Some(Vec::new()));
            }

            let values: Result<Vec<i32>, _> =
                s.split(',').map(|s| s.trim().parse::<i32>()).collect();

            match values {
                Ok(vec) => Ok(Some(vec)),
                Err(e) => Err(D::Error::custom(format!("Failed to parse integers: {}", e))),
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            if value.is_empty() {
                return Ok(Some(Vec::new()));
            }

            let values: Result<Vec<i32>, _> =
                value.split(',').map(|s| s.trim().parse::<i32>()).collect();

            match values {
                Ok(vec) => Ok(Some(vec)),
                Err(e) => Err(E::custom(format!("Failed to parse integers: {}", e))),
            }
        }
    }

    deserializer.deserialize_option(OptionalCommaSeparatedInts)
}

// Custom deserializer for Vec<String> that handles both single string and sequence
pub fn deserialize_vec_string_from_query<'de, D>(
    deserializer: D,
) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, Visitor};
    use std::fmt;

    struct StringOrVec;

    impl<'de> Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or list of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(vec![value.to_owned()])
        }

        fn visit_seq<S>(self, mut visitor: S) -> Result<Self::Value, S::Error>
        where
            S: serde::de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(elem) = visitor.next_element()? {
                vec.push(elem);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Factor {
    #[serde(rename = "type")]
    pub factor_type: String,
    pub level: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillFactor {
    #[serde(rename = "skillId")]
    pub skill_id: i32,
    pub level: i32,
}
