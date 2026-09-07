//! Editing of a tempo as a row of phase fields.

use valens_domain as domain;

use crate::ui::form::FieldValue;

/// Creates a field value per phase slot, the absent phases being empty.
pub fn phase_fields(tempo: &domain::Tempo) -> Vec<FieldValue<domain::Time>> {
    (0..domain::Tempo::MAX_PHASES)
        .map(|index| match tempo.phases().get(index) {
            Some(phase) => FieldValue::new(*phase),
            None => FieldValue::new_with_empty_default(domain::Time::default()),
        })
        .collect()
}

pub fn update_phase_field(field: &mut FieldValue<domain::Time>, value: &str) {
    field.input = value.to_string();
    field.validated = if field.input.is_empty() {
        Ok(domain::Time::default())
    } else {
        domain::Time::try_from(field.input.as_ref()).map_err(|err| err.to_string())
    };
}

/// The tempo of the phase fields, if it can be played.
///
/// A field that cannot be read counts as no tempo, since the phase it stands for is unknown.
pub fn playable_tempo(fields: &[FieldValue<domain::Time>]) -> Option<domain::Tempo> {
    if fields.iter().any(|field| field.validated.is_err()) {
        return None;
    }
    let tempo = validate_tempo(fields).ok()?;
    (!tempo.phases().is_empty()).then_some(tempo)
}

/// Builds the tempo of the phase fields, an empty field before a filled one being a phase of zero
/// seconds and a tempo summing to zero being the empty tempo.
pub fn validate_tempo(fields: &[FieldValue<domain::Time>]) -> Result<domain::Tempo, String> {
    let Some(last) = fields.iter().rposition(|field| !field.input.is_empty()) else {
        return Ok(domain::Tempo::default());
    };
    let phases = fields[..=last]
        .iter()
        .map(|field| u32::from(field.validated.clone().unwrap_or_default()))
        .collect::<Vec<_>>();
    match domain::Tempo::new(&phases) {
        Ok(tempo) => Ok(tempo),
        Err(domain::TempoError::Zero) => Ok(domain::Tempo::default()),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::empty(&["", "", "", ""], Ok(&[][..]))]
    #[case::single_phase(&["4", "", "", ""], Ok(&[4][..]))]
    #[case::trailing_empty_field(&["3", "1", "1", ""], Ok(&[3, 1, 1][..]))]
    #[case::zero_phase_before_a_filled_one(&["3", "", "1", ""], Ok(&[3, 0, 1][..]))]
    #[case::trailing_zero(&["3", "1", "1", "0"], Ok(&[3, 1, 1, 0][..]))]
    #[case::zeros(&["0", "0", "", ""], Ok(&[][..]))]
    #[case::sum_out_of_range(&["999", "1", "", ""], Err("tempo must not be longer than 999 s"))]
    fn test_validate_tempo(#[case] inputs: &[&str], #[case] expected: Result<&[u32], &str>) {
        let fields = inputs
            .iter()
            .map(|input| {
                let mut field = FieldValue::<domain::Time>::default();
                update_phase_field(&mut field, input);
                field
            })
            .collect::<Vec<_>>();

        assert_eq!(
            validate_tempo(&fields).map(|tempo| tempo
                .phases()
                .iter()
                .copied()
                .map(u32::from)
                .collect::<Vec<_>>()),
            expected
                .map(<[u32]>::to_vec)
                .map_err(std::string::ToString::to_string)
        );
    }

    #[rstest]
    #[case::phases(&["3", "1", "1", ""], Some(&[3, 1, 1][..]))]
    #[case::empty(&["", "", "", ""], None)]
    #[case::zeros(&["0", "0", "", ""], None)]
    #[case::field_out_of_range(&["1000", "1", "", ""], None)]
    #[case::sum_out_of_range(&["999", "1", "", ""], None)]
    fn test_playable_tempo(#[case] inputs: &[&str], #[case] expected: Option<&[u32]>) {
        let fields = inputs
            .iter()
            .map(|input| {
                let mut field = FieldValue::<domain::Time>::default();
                update_phase_field(&mut field, input);
                field
            })
            .collect::<Vec<_>>();

        assert_eq!(
            playable_tempo(&fields).map(|tempo| tempo
                .phases()
                .iter()
                .copied()
                .map(u32::from)
                .collect::<Vec<_>>()),
            expected.map(<[u32]>::to_vec)
        );
    }
}
