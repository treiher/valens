use std::collections::BTreeMap;

use chrono::{Duration, Local, NaiveDate};

use crate::{
    CreateError, DeleteError, ReadError, SyncError, UpdateError, ValidationError,
    value_based_centered_moving_average,
};

#[allow(async_fn_in_trait)]
pub trait BodyWeightService {
    async fn get_body_weight(&self) -> Result<Vec<BodyWeight>, ReadError>;
    async fn get_body_weight_on(&self, date: NaiveDate) -> Result<BodyWeight, ReadError>;
    async fn create_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, CreateError>;
    async fn replace_body_weight(&self, body_weight: BodyWeight)
    -> Result<BodyWeight, UpdateError>;
    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, DeleteError>;

    async fn validate_body_weight_date(&self, date: &str) -> Result<NaiveDate, ValidationError> {
        match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(parsed_date) => {
                if parsed_date <= Local::now().date_naive() {
                    match self.get_body_weight().await {
                        Ok(body_weights) => {
                            if body_weights.iter().all(|u| u.date != parsed_date) {
                                Ok(parsed_date)
                            } else {
                                Err(ValidationError::Conflict("date".to_string()))
                            }
                        }
                        Err(err) => Err(ValidationError::Other(err.into())),
                    }
                } else {
                    Err(ValidationError::Other(
                        "Date must not be in the future".into(),
                    ))
                }
            }
            Err(_) => Err(ValidationError::Other("Invalid date".into())),
        }
    }

    fn validate_body_weight_weight(&self, weight: &str) -> Result<f32, ValidationError> {
        match weight.replace(',', ".").trim().parse::<f32>() {
            Ok(parsed_weight) => {
                if parsed_weight > 0.0 {
                    Ok(parsed_weight)
                } else {
                    Err(ValidationError::Other(
                        "Weight must be a positive decimal number".into(),
                    ))
                }
            }
            Err(_) => Err(ValidationError::Other(
                "Weight must be a decimal number".into(),
            )),
        }
    }

    #[must_use]
    fn avg_body_weight(&self, body_weight: &[BodyWeight]) -> Vec<BodyWeight> {
        avg_body_weight(&body_weight.iter().map(|bw| (bw.date, bw.clone())).collect())
            .values()
            .cloned()
            .collect()
    }

    #[must_use]
    fn avg_weekly_change(
        &self,
        avg_body_weight: &[BodyWeight],
        current: Option<&BodyWeight>,
    ) -> Option<f32> {
        avg_weekly_change(
            &avg_body_weight
                .iter()
                .map(|bw| (bw.date, bw.clone()))
                .collect::<BTreeMap<NaiveDate, BodyWeight>>(),
            current,
        )
    }
}

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

#[must_use]
pub fn avg_weekly_change(
    avg_body_weight: &BTreeMap<NaiveDate, BodyWeight>,
    current: Option<&BodyWeight>,
) -> Option<f32> {
    let prev_date = current?.date - Duration::days(7);
    let prev_avg_bw = if let Some(avg_bw) = avg_body_weight.get(&prev_date) {
        avg_bw.clone()
    } else {
        let n = neighbors(avg_body_weight, prev_date);
        interpolate_avg_body_weight(n?.0, n?.1, prev_date)
    };
    Some((current?.weight - prev_avg_bw.weight) / prev_avg_bw.weight * 100.)
}

fn neighbors(
    body_weight: &BTreeMap<NaiveDate, BodyWeight>,
    date: NaiveDate,
) -> Option<(&BodyWeight, &BodyWeight)> {
    use std::ops::Bound::{Excluded, Unbounded};

    let mut before = body_weight.range((Unbounded, Excluded(date)));
    let mut after = body_weight.range((Excluded(date), Unbounded));

    Some((
        before.next_back().map(|(_, v)| v)?,
        after.next().map(|(_, v)| v)?,
    ))
}

fn interpolate_avg_body_weight(a: &BodyWeight, b: &BodyWeight, date: NaiveDate) -> BodyWeight {
    #[allow(clippy::cast_precision_loss)]
    BodyWeight {
        date,
        weight: a.weight
            + (b.weight - a.weight)
                * ((date - a.date).num_days() as f32 / (b.date - a.date).num_days() as f32),
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;
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

    #[test]
    fn test_avg_weekly_change() {
        assert_eq!(
            avg_weekly_change(
                &BTreeMap::new(),
                Some(&BodyWeight {
                    date: from_num_days(1),
                    weight: 70.0
                })
            ),
            None
        );
        assert_eq!(
            avg_weekly_change(
                &BTreeMap::from([(
                    from_num_days(0),
                    BodyWeight {
                        date: from_num_days(0),
                        weight: 70.0
                    }
                )]),
                Some(&BodyWeight {
                    date: from_num_days(7),
                    weight: 70.0
                })
            ),
            Some(0.0)
        );
        assert_approx_eq!(
            avg_weekly_change(
                &BTreeMap::from([(
                    from_num_days(0),
                    BodyWeight {
                        date: from_num_days(0),
                        weight: 70.0
                    }
                )]),
                Some(&BodyWeight {
                    date: from_num_days(7),
                    weight: 70.7
                })
            )
            .unwrap(),
            1.0,
            0.001
        );
        assert_approx_eq!(
            avg_weekly_change(
                &BTreeMap::from([
                    (
                        from_num_days(0),
                        BodyWeight {
                            date: from_num_days(0),
                            weight: 69.0
                        }
                    ),
                    (
                        from_num_days(2),
                        BodyWeight {
                            date: from_num_days(2),
                            weight: 71.0
                        }
                    )
                ]),
                Some(&BodyWeight {
                    date: from_num_days(8),
                    weight: 69.44
                })
            )
            .unwrap(),
            -0.8,
            0.001
        );
    }

    fn from_num_days(days: i32) -> NaiveDate {
        NaiveDate::from_num_days_from_ce_opt(days).unwrap()
    }
}
