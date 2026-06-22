use std::collections::HashMap;

use ionic::mzml::structs::BinaryDataArray;

use crate::error::ImzmlError;
use crate::utilities::{
    byte_width, decode_values, ArrayGroup, BinarySource, EXTERNAL_ARRAY_LENGTH, EXTERNAL_OFFSET,
};

pub(crate) fn fill_array(
    array: &mut BinaryDataArray,
    groups: &HashMap<String, ArrayGroup>,
    source: &mut dyn BinarySource,
) -> Result<(), ImzmlError> {
    let Some(offset) = find_cv_u64(array, EXTERNAL_OFFSET) else {
        return Ok(());
    };
    let group_id = first_group_ref(array).unwrap_or_default();
    let group = groups
        .get(&group_id)
        .ok_or(ImzmlError::UnknownDataType { group: group_id })?;
    let length = find_cv_usize(array, EXTERNAL_ARRAY_LENGTH).ok_or(ImzmlError::MissingArrayLength)?;
    let byte_count = length
        .checked_mul(byte_width(group.numeric_type))
        .ok_or(ImzmlError::ByteCountOverflow)?;
    let bytes = source
        .read_bytes(offset, byte_count)
        .map_err(ImzmlError::io("cannot read imzML binary data"))?;

    array.cv_params = group.inline_params.clone();
    array.numeric_type = Some(group.numeric_type);
    array.array_length = Some(length);
    array.encoded_length = None;
    array.binary = Some(decode_values(&bytes, group.numeric_type));
    Ok(())
}

fn first_group_ref(array: &BinaryDataArray) -> Option<String> {
    array
        .referenceable_param_group_refs
        .first()
        .map(|reference| reference.r#ref.clone())
}

fn find_cv_usize(array: &BinaryDataArray, accession: &str) -> Option<usize> {
    find_cv_value(array, accession).and_then(|value| value.parse().ok())
}

fn find_cv_u64(array: &BinaryDataArray, accession: &str) -> Option<u64> {
    find_cv_value(array, accession).and_then(|value| value.parse().ok())
}

fn find_cv_value<'a>(array: &'a BinaryDataArray, accession: &str) -> Option<&'a str> {
    array
        .cv_params
        .iter()
        .find(|param| param.accession.as_deref() == Some(accession))
        .and_then(|param| param.value.as_deref())
}
