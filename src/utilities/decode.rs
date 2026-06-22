use std::mem::size_of;

use ionic::mzml::structs::{NumericArray, NumericType};

pub(crate) fn byte_width(numeric_type: NumericType) -> usize {
    match numeric_type {
        NumericType::Float16 | NumericType::Int16 => 2,
        NumericType::Float32 | NumericType::Int32 => 4,
        NumericType::Float64 | NumericType::Int64 => 8,
    }
}

pub(crate) fn decode_values(bytes: &[u8], numeric_type: NumericType) -> NumericArray {
    match numeric_type {
        NumericType::Float64 => {
            NumericArray::F64(read_values(bytes, |chunk| f64::from_le_bytes(chunk.try_into().unwrap())))
        }
        NumericType::Float32 => {
            NumericArray::F32(read_values(bytes, |chunk| f32::from_le_bytes(chunk.try_into().unwrap())))
        }
        NumericType::Float16 => {
            NumericArray::F16(read_values(bytes, |chunk| u16::from_le_bytes(chunk.try_into().unwrap())))
        }
        NumericType::Int64 => {
            NumericArray::I64(read_values(bytes, |chunk| i64::from_le_bytes(chunk.try_into().unwrap())))
        }
        NumericType::Int32 => {
            NumericArray::I32(read_values(bytes, |chunk| i32::from_le_bytes(chunk.try_into().unwrap())))
        }
        NumericType::Int16 => {
            NumericArray::I16(read_values(bytes, |chunk| i16::from_le_bytes(chunk.try_into().unwrap())))
        }
    }
}

fn read_values<T, F: Fn(&[u8]) -> T>(bytes: &[u8], from_le: F) -> Vec<T> {
    let stride = size_of::<T>();
    let mut values = Vec::with_capacity(bytes.len() / stride);
    for chunk in bytes.chunks_exact(stride) {
        values.push(from_le(chunk));
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_widths_match_each_type() {
        assert_eq!(byte_width(NumericType::Float16), 2);
        assert_eq!(byte_width(NumericType::Float32), 4);
        assert_eq!(byte_width(NumericType::Float64), 8);
        assert_eq!(byte_width(NumericType::Int16), 2);
        assert_eq!(byte_width(NumericType::Int32), 4);
        assert_eq!(byte_width(NumericType::Int64), 8);
    }

    #[test]
    fn decodes_float64() {
        let bytes: Vec<u8> = [1.5f64, -2.25].iter().flat_map(|v| v.to_le_bytes()).collect();
        match decode_values(&bytes, NumericType::Float64) {
            NumericArray::F64(values) => assert_eq!(values, vec![1.5, -2.25]),
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[test]
    fn decodes_int32() {
        let bytes: Vec<u8> = [7i32, -3].iter().flat_map(|v| v.to_le_bytes()).collect();
        match decode_values(&bytes, NumericType::Int32) {
            NumericArray::I32(values) => assert_eq!(values, vec![7, -3]),
            other => panic!("expected i32, got {other:?}"),
        }
    }

    #[test]
    fn decodes_empty_input() {
        match decode_values(&[], NumericType::Float32) {
            NumericArray::F32(values) => assert!(values.is_empty()),
            other => panic!("expected empty f32, got {other:?}"),
        }
    }
}
