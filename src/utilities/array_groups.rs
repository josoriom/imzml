use std::collections::HashMap;

use ionic::mzml::structs::{CvParam, MzML, NumericType, ReferenceableParamGroup};

use crate::utilities::{
    EXTERNAL_DATA, FLOAT_16_BIT, FLOAT_32_BIT, FLOAT_64_BIT, INT_32_BIT, INT_64_BIT,
};

pub(crate) struct ArrayGroup {
    pub(crate) numeric_type: NumericType,
    pub(crate) inline_params: Vec<CvParam>,
}

pub(crate) fn collect_array_groups(mzml: &MzML) -> HashMap<String, ArrayGroup> {
    let mut groups = HashMap::new();
    let Some(group_list) = mzml.referenceable_param_group_list.as_ref() else {
        return groups;
    };
    for group in &group_list.referenceable_param_groups {
        if let Some(numeric_type) = get_numeric_type(group) {
            groups.insert(
                group.id.clone(),
                ArrayGroup {
                    numeric_type,
                    inline_params: get_inline_params(group),
                },
            );
        }
    }
    groups
}

fn get_numeric_type(group: &ReferenceableParamGroup) -> Option<NumericType> {
    for param in &group.cv_params {
        match param.accession.as_deref() {
            Some(FLOAT_64_BIT) => return Some(NumericType::Float64),
            Some(FLOAT_32_BIT) => return Some(NumericType::Float32),
            Some(FLOAT_16_BIT) => return Some(NumericType::Float16),
            Some(INT_64_BIT) => return Some(NumericType::Int64),
            Some(INT_32_BIT) => return Some(NumericType::Int32),
            _ => {}
        }
    }
    None
}

fn get_inline_params(group: &ReferenceableParamGroup) -> Vec<CvParam> {
    group
        .cv_params
        .iter()
        .filter(|param| param.accession.as_deref() != Some(EXTERNAL_DATA))
        .cloned()
        .collect()
}
