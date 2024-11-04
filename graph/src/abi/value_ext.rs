use alloy::dyn_abi::DynSolType;
use alloy::dyn_abi::DynSolValue;
use anyhow::anyhow;
use anyhow::Result;
use itertools::Itertools;

pub trait DynSolValueExt {
    /// Creates a fixed-byte decoded value from a slice.
    ///
    /// Fails if the source slice exceeds 32 bytes.
    fn fixed_bytes_from_slice(s: &[u8]) -> Result<DynSolValue>;

    /// Returns the decoded value as a string.
    ///
    /// The resulting string contains no type information.
    fn to_string(&self) -> String;

    /// Checks whether the value is of the specified type.
    ///
    /// For types with additional size information, returns true if the size of the value is less
    /// than or equal to the size of the specified type.
    #[must_use]
    fn type_check(&self, ty: &DynSolType) -> bool;
}

impl DynSolValueExt for DynSolValue {
    fn fixed_bytes_from_slice(s: &[u8]) -> Result<Self> {
        let num_bytes = s.len();

        if num_bytes > 32 {
            return Err(anyhow!(
                "input slice must contain a maximum of 32 bytes, got {num_bytes}"
            ));
        }

        let mut bytes = [0u8; 32];
        bytes[..num_bytes].copy_from_slice(s);

        Ok(Self::FixedBytes(bytes.into(), num_bytes))
    }

    fn to_string(&self) -> String {
        let s = |v: &[Self]| v.iter().map(|x| x.to_string()).collect_vec().join(",");

        match self {
            Self::Bool(v) => v.to_string(),
            Self::Int(v, _) => format!("{v:x}"),
            Self::Uint(v, _) => format!("{v:x}"),
            Self::FixedBytes(v, _) => hex::encode(v),
            Self::Address(v) => format!("{v:x}"),
            Self::Function(v) => format!("{v:x}"),
            Self::Bytes(v) => hex::encode(v),
            Self::String(v) => v.to_owned(),
            Self::Array(v) => format!("[{}]", s(v)),
            Self::FixedArray(v) => format!("[{}]", s(v)),
            Self::Tuple(v) => format!("({})", s(v)),
        }
    }

    fn type_check(&self, ty: &DynSolType) -> bool {
        match self {
            Self::Bool(_) => *ty == DynSolType::Bool,
            Self::Int(_, a) => {
                if let DynSolType::Int(b) = ty {
                    b >= a
                } else {
                    false
                }
            }
            Self::Uint(_, a) => {
                if let DynSolType::Uint(b) = ty {
                    b >= a
                } else {
                    false
                }
            }
            Self::FixedBytes(_, a) => {
                if let DynSolType::FixedBytes(b) = ty {
                    b >= a
                } else {
                    false
                }
            }
            Self::Address(_) => *ty == DynSolType::Address,
            Self::Function(_) => *ty == DynSolType::Function,
            Self::Bytes(_) => *ty == DynSolType::Bytes,
            Self::String(_) => *ty == DynSolType::String,
            Self::Array(values) => {
                if let DynSolType::Array(ty) = ty {
                    values.iter().all(|x| x.type_check(ty))
                } else {
                    false
                }
            }
            Self::FixedArray(values) => {
                if let DynSolType::FixedArray(ty, size) = ty {
                    *size == values.len() && values.iter().all(|x| x.type_check(ty))
                } else {
                    false
                }
            }
            Self::Tuple(values) => {
                if let DynSolType::Tuple(types) = ty {
                    values
                        .iter()
                        .enumerate()
                        .all(|(i, x)| x.type_check(&types[i]))
                } else {
                    false
                }
            }
        }
    }
}
