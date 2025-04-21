use std::collections::BTreeMap;

use chrono::NaiveDate;

use crate::{
    CreateError, DeleteError, ReadError, SyncError, UpdateError,
    value_based_centered_moving_average,
};

#[allow(async_fn_in_trait)]
pub trait BodyWeightRepository {
    async fn sync_body_weight(&self) -> Result<Vec<BodyWeight>, SyncError>;
    async fn read_body_weight(&self) -> Result<Vec<BodyWeight>, ReadError>;
    async fn create_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, CreateError>;
    async fn replace_body_weight(&self, body_weight: BodyWeight)
    -> Result<BodyWeight, UpdateError>;
    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, DeleteError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct BodyWeight {
    pub date: NaiveDate,
    pub weight: f32,
}

#[must_use]
pub fn avg_body_weight(
    body_weight: &BTreeMap<NaiveDate, BodyWeight>,
) -> BTreeMap<NaiveDate, BodyWeight> {
    let data = body_weight
        .values()
        .map(|bw| (bw.date, bw.weight))
        .collect::<Vec<_>>();
    value_based_centered_moving_average(&data, 4)
        .into_iter()
        .map(|(date, weight)| (date, BodyWeight { date, weight }))
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::no_value(vec![], vec![])]
    #[case::one_value(
        vec![BodyWeight { date: from_num_days(0), weight: 80.0 }],
        vec![BodyWeight { date: from_num_days(0), weight: 80.0 }],
    )]
    #[case::less_values_than_radius(
        vec![
            BodyWeight { date: from_num_days(0), weight: 80.0 },
            BodyWeight { date: from_num_days(2), weight: 82.0 },
            BodyWeight { date: from_num_days(3), weight: 79.0 },
            BodyWeight { date: from_num_days(5), weight: 79.0 },
        ],
        vec![
            BodyWeight { date: from_num_days(0), weight: 80.0 },
            BodyWeight { date: from_num_days(2), weight: 80.0 },
            BodyWeight { date: from_num_days(3), weight: 80.0 },
            BodyWeight { date: from_num_days(5), weight: 80.0 },
        ],
    )]
    #[case::more_values_than_radius(
        vec![
            BodyWeight { date: from_num_days(0), weight: 81.0 },
            BodyWeight { date: from_num_days(2), weight: 82.0 },
            BodyWeight { date: from_num_days(3), weight: 83.0 },
            BodyWeight { date: from_num_days(5), weight: 84.0 },
            BodyWeight { date: from_num_days(6), weight: 85.0 },
            BodyWeight { date: from_num_days(8), weight: 86.0 },
            BodyWeight { date: from_num_days(9), weight: 87.0 },
            BodyWeight { date: from_num_days(10), weight: 88.0 },
            BodyWeight { date: from_num_days(12), weight: 89.0 },
        ],
        vec![
            BodyWeight { date: from_num_days(0), weight: 83.0 },
            BodyWeight { date: from_num_days(2), weight: 83.5 },
            BodyWeight { date: from_num_days(3), weight: 84.0 },
            BodyWeight { date: from_num_days(5), weight: 84.5 },
            BodyWeight { date: from_num_days(6), weight: 85.0 },
            BodyWeight { date: from_num_days(8), weight: 85.5 },
            BodyWeight { date: from_num_days(9), weight: 86.0 },
            BodyWeight { date: from_num_days(10), weight: 86.5 },
            BodyWeight { date: from_num_days(12), weight: 87.0 },
        ],
    )]
    fn test_avg_body_weight(
        #[case] body_weight: Vec<BodyWeight>,
        #[case] expected: Vec<BodyWeight>,
    ) {
        assert_eq!(
            avg_body_weight(&body_weight.into_iter().map(|bw| (bw.date, bw)).collect()),
            expected.into_iter().map(|bw| (bw.date, bw)).collect()
        );
    }

    fn from_num_days(days: i32) -> NaiveDate {
        NaiveDate::from_num_days_from_ce_opt(days).unwrap()
    }
}
