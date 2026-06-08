use chrono::NaiveDate;

use crate::{BodyFat, BodyWeight, Sex};

const REFERENCE_HEIGHT_M: f32 = 1.8;

/// Normalized fat-free mass index for each body fat measurement with a Jackson-Pollock 3 value.
///
/// Each measurement is paired with the most recent body weight on or before its date; measurements
/// without a preceding body weight are omitted. `height` is given in centimeters.
#[must_use]
pub fn ffmi(
    body_weight: &[BodyWeight],
    body_fat: &[BodyFat],
    sex: Sex,
    height: u8,
) -> Vec<(NaiveDate, f32)> {
    body_fat
        .iter()
        .filter_map(|bf| {
            let body_fat_percentage = bf.jp3(sex)?;
            let weight = most_recent_weight(body_weight, bf.date)?;
            Some((
                bf.date,
                normalized_ffmi(weight, body_fat_percentage, height),
            ))
        })
        .collect()
}

fn most_recent_weight(body_weight: &[BodyWeight], date: NaiveDate) -> Option<f32> {
    body_weight
        .iter()
        .filter(|bw| bw.date <= date)
        .max_by_key(|bw| bw.date)
        .map(|bw| bw.weight)
}

fn normalized_ffmi(weight: f32, body_fat_percentage: f32, height: u8) -> f32 {
    let height_m = f32::from(height) / 100.0;
    let fat_free_mass = weight * (1.0 - body_fat_percentage / 100.0);
    let ffmi = fat_free_mass / (height_m * height_m);
    ffmi + 6.1 * (REFERENCE_HEIGHT_M - height_m)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    fn body_weight(year: i32, month: u32, day: u32, weight: f32) -> BodyWeight {
        BodyWeight {
            date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            weight,
        }
    }

    fn body_fat(year: i32, month: u32, day: u32, chest: u8, abdominal: u8, thigh: u8) -> BodyFat {
        BodyFat {
            date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            chest: Some(chest),
            abdominal: Some(abdominal),
            thigh: Some(thigh),
            tricep: None,
            subscapular: None,
            suprailiac: None,
            midaxillary: None,
        }
    }

    #[rstest]
    #[case::reference_height(180, 22.07)]
    #[case::short(160, 29.16)]
    #[case::tall(200, 16.66)]
    fn test_ffmi(#[case] height: u8, #[case] expected: f32) {
        let weights = [body_weight(2020, 2, 1, 80.0)];
        let fats = [body_fat(2020, 2, 2, 5, 15, 15)];

        let result = ffmi(&weights, &fats, Sex::MALE, height);

        let [(date, value)] = result[..] else {
            panic!("expected exactly one value, got {result:?}");
        };
        assert_eq!(date, fats[0].date);
        assert!((value - expected).abs() < 0.005, "value = {value}");
    }

    #[test]
    fn test_ffmi_uses_most_recent_preceding_weight() {
        let weights = [
            body_weight(2020, 1, 1, 70.0),
            body_weight(2020, 3, 1, 90.0),
            body_weight(2020, 2, 1, 80.0),
        ];
        let fats = [body_fat(2020, 2, 15, 5, 15, 15)];

        assert_eq!(
            ffmi(&weights, &fats, Sex::MALE, 180),
            ffmi(&[body_weight(2020, 2, 1, 80.0)], &fats, Sex::MALE, 180)
        );
    }

    #[test]
    fn test_ffmi_omits_measurement_without_preceding_weight() {
        let weights = [body_weight(2020, 3, 1, 80.0)];
        let fats = [body_fat(2020, 2, 2, 5, 15, 15)];

        assert_eq!(ffmi(&weights, &fats, Sex::MALE, 180), vec![]);
    }

    #[test]
    fn test_ffmi_omits_measurement_without_jp3() {
        let weights = [body_weight(2020, 2, 1, 80.0)];
        let fats = [BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: None,
            abdominal: None,
            thigh: None,
            tricep: None,
            subscapular: None,
            suprailiac: None,
            midaxillary: None,
        }];

        assert_eq!(ffmi(&weights, &fats, Sex::MALE, 180), vec![]);
    }
}
