use chrono::NaiveDate;

use crate::{CreateError, DeleteError, ReadError, Sex, SyncError, UpdateError};

#[allow(async_fn_in_trait)]
pub trait BodyFatRepository {
    async fn sync_body_fat(&self) -> Result<Vec<BodyFat>, SyncError>;
    async fn read_body_fat(&self) -> Result<Vec<BodyFat>, ReadError>;
    async fn create_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, CreateError>;
    async fn replace_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, UpdateError>;
    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, DeleteError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyFat {
    pub date: NaiveDate,
    pub chest: Option<u8>,
    pub abdominal: Option<u8>,
    pub thigh: Option<u8>,
    pub tricep: Option<u8>,
    pub subscapular: Option<u8>,
    pub suprailiac: Option<u8>,
    pub midaxillary: Option<u8>,
}

impl BodyFat {
    #[must_use]
    pub fn jp3(&self, sex: Sex) -> Option<f32> {
        match sex {
            Sex::FEMALE => Some(Self::jackson_pollock(
                f32::from(self.tricep?) + f32::from(self.suprailiac?) + f32::from(self.thigh?),
                1.099_492_1,
                0.000_992_9,
                0.000_002_3,
                0.000_139_2,
            )),
            Sex::MALE => Some(Self::jackson_pollock(
                f32::from(self.chest?) + f32::from(self.abdominal?) + f32::from(self.thigh?),
                1.109_38,
                0.000_826_7,
                0.000_001_6,
                0.000_257_4,
            )),
        }
    }

    #[must_use]
    pub fn jp7(&self, sex: Sex) -> Option<f32> {
        match sex {
            Sex::FEMALE => Some(Self::jackson_pollock(
                f32::from(self.chest?)
                    + f32::from(self.abdominal?)
                    + f32::from(self.thigh?)
                    + f32::from(self.tricep?)
                    + f32::from(self.subscapular?)
                    + f32::from(self.suprailiac?)
                    + f32::from(self.midaxillary?),
                1.097,
                0.000_469_71,
                0.000_000_56,
                0.000_128_28,
            )),
            Sex::MALE => Some(Self::jackson_pollock(
                f32::from(self.chest?)
                    + f32::from(self.abdominal?)
                    + f32::from(self.thigh?)
                    + f32::from(self.tricep?)
                    + f32::from(self.subscapular?)
                    + f32::from(self.suprailiac?)
                    + f32::from(self.midaxillary?),
                1.112,
                0.000_434_99,
                0.000_000_55,
                0.000_288_26,
            )),
        }
    }

    fn jackson_pollock(sum: f32, k0: f32, k1: f32, k2: f32, ka: f32) -> f32 {
        let age = 30.; // assume an age of 30
        (495. / (k0 - (k1 * sum) + (k2 * sum * sum) - (ka * age))) - 450.
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::female_none(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: None,
            abdominal: None,
            thigh: None,
            tricep: None,
            subscapular: None,
            suprailiac: None,
            midaxillary: None,
        },
        Sex::FEMALE,
        None,
        None
    )]
    #[case::female_jp3(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: None,
            abdominal: None,
            thigh: Some(20),
            tricep: Some(15),
            subscapular: None,
            suprailiac: Some(5),
            midaxillary: None,
        },
        Sex::FEMALE,
        Some(17.298_523),
        None
    )]
    #[case::female_jp7(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(5),
            abdominal: Some(10),
            thigh: Some(20),
            tricep: Some(15),
            subscapular: Some(5),
            suprailiac: Some(5),
            midaxillary: Some(5),
        },
        Sex::FEMALE,
        Some(17.298_523),
        Some(14.794_678)
    )]
    #[case::male_none(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: None,
            abdominal: None,
            thigh: None,
            tricep: None,
            subscapular: None,
            suprailiac: None,
            midaxillary: None,
        },
        Sex::MALE,
        None,
        None
    )]
    #[case::male_jp3(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(5),
            abdominal: Some(15),
            thigh: Some(15),
            tricep: None,
            subscapular: None,
            suprailiac: None,
            midaxillary: None,
        },
        Sex::MALE,
        Some(10.600_708),
        None
    )]
    #[case::male_jp7(
        BodyFat {
            date: NaiveDate::from_ymd_opt(2020, 2, 2).unwrap(),
            chest: Some(5),
            abdominal: Some(15),
            thigh: Some(15),
            tricep: Some(15),
            subscapular: Some(10),
            suprailiac: Some(10),
            midaxillary: Some(10),
        },
        Sex::MALE,
        Some(10.600_708),
        Some(11.722_29)
    )]
    fn test_body_fat_jp(
        #[case] body_fat: BodyFat,
        #[case] sex: Sex,
        #[case] expected_jp3: Option<f32>,
        #[case] expected_jp7: Option<f32>,
    ) {
        assert_eq!(body_fat.jp3(sex), expected_jp3);
        assert_eq!(body_fat.jp7(sex), expected_jp7);
    }
}
